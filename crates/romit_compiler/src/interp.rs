use crate::ast::*;
use crate::check::{CheckedProgram, CheckedScript};
use crate::diag::{snippet_at, Diagnostic};
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Write as _;

type SaySink = Box<dyn FnMut(&str)>;
type AskSource = Box<dyn FnMut(&str) -> Option<String>>;

thread_local! {
    static SAY_SINK: RefCell<Option<SaySink>> = const { RefCell::new(None) };
    static ASK_SOURCE: RefCell<Option<AskSource>> = const { RefCell::new(None) };
}

/// Send script output somewhere as it happens (a terminal, the playground).
/// Without a sink, output is collected into [`RunOutcome::stdout`].
pub fn set_say_sink(sink: Option<SaySink>) {
    SAY_SINK.with(|s| *s.borrow_mut() = sink);
}

/// Answer `ask "Question?"` in scripts. The source gets the question and returns
/// the answer, or `None` when there is no more input.
pub fn set_ask_source(source: Option<AskSource>) {
    ASK_SOURCE.with(|s| *s.borrow_mut() = source);
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Text(String),
    Int(i64),
    Money(i64),
    Bool(bool),
    Enum(String),
    /// Decimal number (`1.5`, `7 / 2`).
    Float(f64),
    /// Ordered list (`list 1, 2, 3`).
    List(Vec<Value>),
    /// Named fields in order (`with name "Ada", age 36`, `map "tea" to 3`).
    Record(Vec<(String, Value)>),
    Null,
}

#[derive(Debug, Clone)]
pub struct OutputRecord {
    pub name: String,
    pub args: Vec<Value>,
}

#[derive(Debug, Clone)]
pub struct RuntimeState {
    pub config: HashMap<String, Value>,
    pub state: HashMap<String, Value>,
    /// entity name -> row id -> field -> value
    pub entities: HashMap<String, HashMap<String, HashMap<String, Value>>>,
}

#[derive(Debug, Clone)]
pub struct StepResult {
    pub outputs: Vec<OutputRecord>,
    pub rejected: Option<String>,
    pub fired_rules: Vec<(String, String)>,
}

pub struct Interpreter {
    pub program: CheckedProgram,
    entity_key: HashMap<String, String>,
}

impl Interpreter {
    pub fn new(program: CheckedProgram) -> Self {
        let mut entity_key = HashMap::new();
        for decl in &program.spec.decls {
            if let Decl::Entity(e) = decl {
                entity_key.insert(e.name.clone(), e.key.clone());
            }
        }
        Self {
            program,
            entity_key,
        }
    }

    pub fn init_from_spec(&self) -> RuntimeState {
        let mut rt = RuntimeState {
            config: HashMap::new(),
            state: HashMap::new(),
            entities: HashMap::new(),
        };
        for decl in &self.program.spec.decls {
            match decl {
                Decl::Config(c) => {
                    for f in &c.fields {
                        if let Some(d) = &f.default {
                            rt.config.insert(f.name.clone(), literal_to_value(d));
                        }
                    }
                }
                Decl::State(s) => {
                    for f in &s.fields {
                        if let Some(d) = &f.default {
                            rt.state.insert(f.name.clone(), literal_to_value(d));
                        }
                    }
                }
                _ => {}
            }
        }
        rt
    }

    pub fn apply_given(&self, rt: &mut RuntimeState, given: &Given) {
        match given {
            Given::StateAssign { name, value, .. } => {
                rt.state
                    .insert(name.clone(), self.eval_expr(rt, value, &Env::default()));
            }
            Given::EntityInstance { entity, fields, .. } => {
                let key_field = self
                    .entity_key
                    .get(entity)
                    .cloned()
                    .unwrap_or_else(|| "id".into());
                let id = fields
                    .get(&key_field)
                    .map(|e| value_to_key(&self.eval_expr(rt, e, &Env::default())))
                    .unwrap_or_else(|| "unknown".into());
                let mut row_values: HashMap<String, Value> = HashMap::new();
                for (k, v) in fields {
                    row_values.insert(k.clone(), self.eval_expr(rt, v, &Env::default()));
                }
                let row = rt.entities.entry(entity.clone()).or_default();
                row.insert(id, row_values);
            }
        }
    }

    pub fn run_event(
        &self,
        rt: &mut RuntimeState,
        event_name: &str,
        args: &HashMap<String, Value>,
    ) -> Result<StepResult, String> {
        let event = self
            .program
            .events
            .get(event_name)
            .ok_or_else(|| format!("unknown event {}", event_name))?;

        let mut snapshot = rt.clone();
        let mut outputs = Vec::new();
        let mut fired = Vec::new();

        self.apply_stores(&mut snapshot, event, args)?;

        let handlers: Vec<_> = self
            .program
            .handlers
            .iter()
            .filter(|h| h.event == event_name)
            .collect();

        let mut stop_handlers = false;
        for h in handlers {
            if stop_handlers {
                break;
            }
            let env = Env::from_handler(h, args, &self.program);
            if let Some(cond) = &h.condition {
                if !truthy(&self.eval_expr(&snapshot, cond, &env)) {
                    continue;
                }
            }

            let handler_snapshot = snapshot.clone();
            let mut handler_state = handler_snapshot;
            let mut any_rule_fired = false;

            for rule in &h.rules {
                let matches_rule = match &rule.when {
                    RuleWhen::Otherwise => !h.run_all && !any_rule_fired,
                    RuleWhen::Expr(e) => truthy(&self.eval_expr(&handler_state, e, &env)),
                };
                if !matches_rule {
                    continue;
                }

                any_rule_fired = true;
                fired.push((event_name.to_string(), rule.name.clone()));

                for act in &rule.actions {
                    match act {
                        Action::Set { path, value, .. } => {
                            let v = self.eval_expr(&handler_state, value, &env);
                            if let Err(e) = self.set_path(&mut handler_state, path, &env, v) {
                                return Ok(StepResult {
                                    outputs: vec![],
                                    rejected: Some(e),
                                    fired_rules: fired,
                                });
                            }
                        }
                        Action::Emit { name, args, .. } => {
                            let vals: Vec<_> = args
                                .iter()
                                .map(|a| self.eval_expr(&handler_state, a, &env))
                                .collect();
                            outputs.push(OutputRecord {
                                name: name.clone(),
                                args: vals,
                            });
                        }
                        Action::Reject { message, .. } => {
                            return Ok(StepResult {
                                outputs: vec![],
                                rejected: Some(message.clone()),
                                fired_rules: fired,
                            });
                        }
                        Action::Say { parts, .. } => {
                            let mut text = String::new();
                            for part in parts {
                                let v = self.eval_expr(&handler_state, part, &env);
                                text.push_str(&value_to_display(&v));
                            }
                            outputs.push(OutputRecord {
                                name: "__say__".into(),
                                args: vec![Value::Text(text)],
                            });
                        }
                    }
                }

                if !h.run_all {
                    break;
                }
            }
            snapshot = handler_state;
            if any_rule_fired && !h.run_all {
                stop_handlers = true;
            }
        }

        if let Err(msg) = self.check_invariants(&snapshot) {
            return Ok(StepResult {
                outputs: vec![],
                rejected: Some(msg),
                fired_rules: fired,
            });
        }

        if let Err(msg) = self.check_limits(&outputs) {
            return Ok(StepResult {
                outputs: vec![],
                rejected: Some(msg),
                fired_rules: fired,
            });
        }

        if let Err(msg) = self.check_machine_sets(&snapshot) {
            let _ = msg;
        }

        *rt = snapshot;
        Ok(StepResult {
            outputs,
            rejected: None,
            fired_rules: fired,
        })
    }

    fn apply_stores(
        &self,
        rt: &mut RuntimeState,
        event: &EventDecl,
        args: &HashMap<String, Value>,
    ) -> Result<(), String> {
        match &event.stores {
            StoreMode::None => Ok(()),
            StoreMode::All => {
                for (k, v) in args {
                    rt.state.insert(k.clone(), v.clone());
                }
                Ok(())
            }
            StoreMode::Named(names) => {
                let leg_key = event.params.first().map(|p| p.name.clone());
                let entity = event.params.first().and_then(|p| {
                    if let Type::Named(e) = &p.ty {
                        Some(e.clone())
                    } else {
                        None
                    }
                });
                for n in names {
                    if let Some(v) = args.get(n) {
                        if let (Some(ent), Some(key)) = (&entity, &leg_key) {
                            if let Some(Value::Text(id)) = args.get(key) {
                                let row = rt.entities.entry(ent.clone()).or_default();
                                let rec = row.entry(id.clone()).or_default();
                                rec.insert(n.clone(), v.clone());
                                continue;
                            }
                        }
                        rt.state.insert(n.clone(), v.clone());
                    }
                }
                Ok(())
            }
        }
    }

    fn set_path(
        &self,
        rt: &mut RuntimeState,
        path: &PathExpr,
        env: &Env,
        value: Value,
    ) -> Result<(), String> {
        if path.parts.len() == 1 {
            let name = &path.parts[0];
            if rt.state.contains_key(name) || self.program.state_fields.contains_key(name) {
                rt.state.insert(name.clone(), value);
                return Ok(());
            }
            return Err(format!("cannot assign to `{}`", name));
        }
        if path.parts.len() == 2 {
            let root = &path.parts[0];
            let field = &path.parts[1];
            if root == "config" {
                rt.config.insert(field.clone(), value);
                return Ok(());
            }
            if let Some((ent, id)) = env.entity_ref(root) {
                let row = rt.entities.entry(ent).or_default();
                let rec = row.entry(id).or_default();
                rec.insert(field.clone(), value);
                return Ok(());
            }
            if rt.state.contains_key(field) || self.program.state_fields.contains_key(field) {
                rt.state.insert(field.clone(), value);
                return Ok(());
            }
            if rt.state.contains_key(root) {
                rt.state.insert(root.clone(), value);
                return Ok(());
            }
        }
        Err(format!("cannot assign to {}", path.parts.join(".")))
    }

    fn check_invariants(&self, rt: &RuntimeState) -> Result<(), String> {
        for inv in &self.program.invariants {
            if !truthy(&self.eval_invariant(rt, &inv.expr)) {
                return Err(format!("invariant {} violated", inv.name));
            }
        }
        Ok(())
    }

    fn check_limits(&self, outputs: &[OutputRecord]) -> Result<(), String> {
        for lim in &self.program.limits {
            let count = outputs.iter().filter(|o| o.name == lim.output).count();
            if count as i64 > lim.max {
                return Err(format!("limit {} exceeded", lim.output));
            }
        }
        Ok(())
    }

    fn check_machine_sets(&self, _rt: &RuntimeState) -> Result<(), String> {
        Ok(())
    }

    fn eval_invariant(&self, rt: &RuntimeState, expr: &Expr) -> Value {
        self.eval_expr(rt, expr, &Env::default())
    }

    pub fn eval_expr(&self, rt: &RuntimeState, expr: &Expr, env: &Env) -> Value {
        match expr {
            Expr::Literal(l, _) => literal_to_value(l),
            Expr::Path(p, _) => self.eval_path(rt, p, env),
            Expr::Call {
                name, args, named, ..
            } => {
                let _ = named;
                if env.call_depth > 128 {
                    return Value::Null;
                }
                self.eval_call(rt, name, args, env)
            }
            Expr::Unary { op, expr, .. } => {
                let v = self.eval_expr(rt, expr, env);
                match op {
                    UnaryOp::Not => Value::Bool(!truthy(&v)),
                    UnaryOp::Neg => neg_value(v),
                }
            }
            Expr::Binary {
                op, left, right, ..
            } => {
                let l = self.eval_expr(rt, left, env);
                let r = self.eval_expr(rt, right, env);
                eval_binary(op, &l, &r)
            }
            Expr::If {
                cond,
                then_br,
                else_br,
                ..
            } => {
                if truthy(&self.eval_expr(rt, cond, env)) {
                    self.eval_expr(rt, then_br, env)
                } else {
                    self.eval_expr(rt, else_br, env)
                }
            }
            Expr::Aggregate {
                kind,
                entity,
                filter,
                ..
            } => self.eval_aggregate(rt, kind, entity, filter.as_deref(), env),
            Expr::Index {
                entity, key, field, ..
            } => {
                let id = value_to_key(&self.eval_expr(rt, key, env));
                rt.entities
                    .get(entity)
                    .and_then(|m| m.get(&id))
                    .and_then(|row| row.get(field))
                    .cloned()
                    .unwrap_or(Value::Null)
            }
            Expr::ForEach {
                var, entity, body, ..
            } => {
                let rows: Vec<_> = rt
                    .entities
                    .get(entity)
                    .map(|m| m.values().cloned().collect())
                    .unwrap_or_default();
                for row in rows {
                    let mut sub = env.clone();
                    if let Some(id) = row.get("id") {
                        sub.locals.insert(var.clone(), id.clone());
                    }
                    for (f, v) in row {
                        sub.bind_field(var, &f, v.clone());
                    }
                    if !truthy(&self.eval_expr(rt, body, &sub)) {
                        return Value::Bool(false);
                    }
                }
                Value::Bool(true)
            }
        }
    }

    fn eval_path(&self, rt: &RuntimeState, p: &PathExpr, env: &Env) -> Value {
        if p.parts.len() == 1 {
            let n = &p.parts[0];
            if let Some(v) = env.locals.get(n) {
                return v.clone();
            }
            if let Some(v) = rt.config.get(n) {
                return v.clone();
            }
            if let Some(v) = rt.state.get(n) {
                return v.clone();
            }
            for (_en, variants) in &self.program.enums {
                if variants.iter().any(|v| v == n) {
                    return Value::Enum(n.clone());
                }
            }
            return Value::Null;
        }
        if p.parts.len() == 2 {
            let (a, b) = (&p.parts[0], &p.parts[1]);
            if a == "config" {
                return rt.config.get(b).cloned().unwrap_or(Value::Null);
            }
            if let Some(v) = env.field(a, b) {
                return v;
            }
            if let Some((ent, id)) = env.entity_ref(a) {
                if let Some(row) = rt.entities.get(&ent).and_then(|m| m.get(&id)) {
                    if let Some(v) = row.get(b) {
                        return v.clone();
                    }
                    if b == "decision" || b == "status" {
                        return row.get(b).cloned().unwrap_or(Value::Null);
                    }
                }
            }
        }
        Value::Null
    }

    fn eval_call(&self, rt: &RuntimeState, name: &str, args: &[Expr], env: &Env) -> Value {
        let Some(def) = self.program.defines.get(name) else {
            return Value::Null;
        };
        let mut sub = env.clone();
        sub.call_depth = env.call_depth.saturating_add(1);
        for (p, arg) in def.params.iter().zip(args.iter()) {
            let val = self.eval_expr(rt, arg, env);
            sub.locals.insert(p.name.clone(), val.clone());
            if let Expr::Path(path, _) = arg {
                if path.parts.len() == 1 {
                    if let Some(er) = env.entity_refs.get(&path.parts[0]) {
                        sub.entity_refs.insert(p.name.clone(), er.clone());
                    }
                }
            }
            if let Type::Named(ent) = &p.ty {
                if self.program.entity_fields.contains_key(ent) {
                    if let Value::Text(id) = &val {
                        sub.entity_refs
                            .insert(p.name.clone(), (ent.clone(), id.clone()));
                    }
                }
            }
        }
        self.eval_expr(rt, &def.body, &sub)
    }

    fn eval_aggregate(
        &self,
        rt: &RuntimeState,
        kind: &AggregateKind,
        entity: &str,
        filter: Option<&Expr>,
        env: &Env,
    ) -> Value {
        let rows: Vec<_> = rt
            .entities
            .get(entity)
            .map(|m| m.values().cloned().collect())
            .unwrap_or_default();
        let mut count = 0i64;
        for row in rows {
            let mut sub = env.clone();
            let id = row.get("id").cloned().unwrap_or(Value::Null);
            sub.locals.insert("leg".into(), id);
            for (f, v) in row {
                sub.bind_field("leg", &f, v);
            }
            let pass = filter
                .map(|f| truthy(&self.eval_expr(rt, f, &sub)))
                .unwrap_or(true);
            if pass {
                count += 1;
            }
        }
        match kind {
            AggregateKind::Count => Value::Int(count),
            _ => Value::Int(count),
        }
    }

    pub fn run_tests(&self, file: &str, source: &str) -> Result<Vec<String>, Vec<Diagnostic>> {
        let mut diags = Vec::new();
        let mut passed = Vec::new();
        for test in &self.program.tests {
            let mut rt = self.init_from_spec();
            for g in &test.givens {
                self.apply_given(&mut rt, g);
            }
            let mut last = StepResult {
                outputs: vec![],
                rejected: None,
                fired_rules: vec![],
            };
            let mut ok = true;
            for step in &test.steps {
                match step {
                    TestStep::Send(send) => {
                        let mut args = HashMap::new();
                        for (k, e) in &send.args {
                            args.insert(k.clone(), self.eval_expr(&rt, e, &Env::default()));
                        }
                        match self.run_event(&mut rt, &send.event, &args) {
                            Ok(r) => last = r,
                            Err(e) => {
                                ok = false;
                                diags.push(Diagnostic::error(
                                    "E199",
                                    e,
                                    file,
                                    send.span.line,
                                    send.span.column,
                                    snippet_at(source, send.span.line, send.span.column, "send"),
                                ));
                            }
                        }
                    }
                    TestStep::Expect(ex) => {
                        if !check_expect(self, &rt, ex, &last) {
                            ok = false;
                            diags.push(Diagnostic::error(
                                "E199",
                                format!("test `{}` failed expectation", test.name),
                                file,
                                ex.span().line,
                                ex.span().column,
                                snippet_at(source, ex.span().line, ex.span().column, "expect"),
                            ));
                        }
                    }
                }
            }
            if ok {
                passed.push(test.name.clone());
            }
        }
        if diags.is_empty() {
            Ok(passed)
        } else {
            Err(diags)
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Env {
    locals: HashMap<String, Value>,
    /// binding -> (entity, id)
    entity_refs: HashMap<String, (String, String)>,
    /// binding.field -> value
    fields: HashMap<String, Value>,
    call_depth: u8,
}

impl Env {
    fn from_handler(h: &HandlerDecl, args: &HashMap<String, Value>, prog: &CheckedProgram) -> Self {
        let mut e = Env::default();
        let mut param_types: HashMap<String, String> = HashMap::new();
        if let Some(ev) = prog.events.get(&h.event) {
            for p in &ev.params {
                if let Type::Named(ent) = &p.ty {
                    param_types.insert(p.name.clone(), ent.clone());
                }
                if let Some(v) = args.get(&p.name) {
                    e.locals.insert(p.name.clone(), v.clone());
                }
            }
        }
        for b in &h.bindings {
            if let Some(v) = args.get(b) {
                e.locals.insert(b.clone(), v.clone());
                if let Value::Text(id) = v {
                    if let Some(ent) = param_types.get(b) {
                        e.entity_refs.insert(b.clone(), (ent.clone(), id.clone()));
                    }
                }
            }
        }
        e
    }

    fn bind_field(&mut self, root: &str, field: &str, value: Value) {
        self.fields.insert(format!("{}.{}", root, field), value);
    }

    fn field(&self, root: &str, field: &str) -> Option<Value> {
        self.fields
            .get(&format!("{}.{}", root, field))
            .cloned()
            .or_else(|| {
                if let Some((ent, id)) = self.entity_refs.get(root) {
                    // loaded dynamically in eval_path
                    let _ = (ent, id);
                    None
                } else {
                    None
                }
            })
    }

    fn entity_ref(&self, binding: &str) -> Option<(String, String)> {
        self.entity_refs.get(binding).cloned()
    }
}

trait EntityKeyHelper {
    fn entity_key(&self, ent: &str) -> Option<&String>;
}

impl EntityKeyHelper for CheckedProgram {
    fn entity_key(&self, ent: &str) -> Option<&String> {
        self.spec.decls.iter().find_map(|d| {
            if let Decl::Entity(e) = d {
                if e.name == ent {
                    Some(&e.key)
                } else {
                    None
                }
            } else {
                None
            }
        })
    }
}

impl Expect {
    fn span(&self) -> &Span {
        match self {
            Expect::Expr { span, .. } => span,
            Expect::Emitted { span, .. } => span,
            Expect::Rejected { span, .. } => span,
            Expect::NoOutputs { span } => span,
        }
    }
}

fn check_expect(interp: &Interpreter, rt: &RuntimeState, ex: &Expect, result: &StepResult) -> bool {
    match ex {
        Expect::Expr { expr, .. } => truthy(&interp.eval_expr(rt, expr, &Env::default())),
        Expect::Emitted { output, args, .. } => result.outputs.iter().any(|o| {
            if o.name != *output {
                return false;
            }
            if args.is_empty() {
                return true;
            }
            if args.len() == o.args.len() {
                return args
                    .iter()
                    .zip(o.args.iter())
                    .all(|(e, v)| values_equal(&interp.eval_expr(rt, e, &Env::default()), v));
            }
            if o.args.len() == 1 && args.len() >= 1 {
                return values_equal(
                    &interp.eval_expr(rt, &args[args.len() - 1], &Env::default()),
                    &o.args[0],
                );
            }
            false
        }),
        Expect::Rejected { message, .. } => result.rejected.as_deref() == Some(message.as_str()),
        Expect::NoOutputs { .. } => result.outputs.is_empty() && result.rejected.is_none(),
    }
}

fn as_num(v: &Value) -> Option<f64> {
    match v {
        Value::Int(n) | Value::Money(n) => Some(*n as f64),
        Value::Float(f) => Some(*f),
        _ => None,
    }
}

fn values_equal(a: &Value, b: &Value) -> bool {
    if let (Some(x), Some(y)) = (as_num(a), as_num(b)) {
        return x == y;
    }
    match (a, b) {
        (Value::Text(x), Value::Text(y)) => x == y,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Enum(x), Value::Enum(y)) => x == y,
        (Value::Text(x), Value::Enum(y)) => x == y,
        (Value::Enum(x), Value::Text(y)) => x == y,
        (Value::List(x), Value::List(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(a, b)| values_equal(a, b))
        }
        (Value::Record(x), Value::Record(y)) => {
            x.len() == y.len()
                && x.iter()
                    .all(|(k, v)| y.iter().any(|(k2, v2)| k == k2 && values_equal(v, v2)))
        }
        (Value::Null, Value::Null) => true,
        _ => false,
    }
}

fn literal_to_value(l: &Literal) -> Value {
    match l {
        Literal::Int(n) => Value::Int(*n),
        Literal::Decimal(s) => Value::Float(s.parse().unwrap_or(0.0)),
        Literal::Text(s) => Value::Text(s.clone()),
        Literal::Bool(b) => Value::Bool(*b),
        Literal::Enum(s) => Value::Enum(s.clone()),
    }
}

fn value_to_key(v: &Value) -> String {
    match v {
        Value::Text(s) => s.clone(),
        Value::Int(n) => n.to_string(),
        Value::Enum(s) => s.clone(),
        _ => String::new(),
    }
}

fn truthy(v: &Value) -> bool {
    match v {
        Value::Bool(b) => *b,
        Value::Null => false,
        Value::Int(0) => false,
        Value::Float(f) => *f != 0.0,
        Value::Text(s) => !s.is_empty(),
        Value::List(l) => !l.is_empty(),
        Value::Record(r) => !r.is_empty(),
        _ => true,
    }
}

fn neg_value(v: Value) -> Value {
    match v {
        Value::Int(n) => Value::Int(-n),
        Value::Money(m) => Value::Money(-m),
        Value::Float(f) => Value::Float(-f),
        _ => v,
    }
}

fn eval_binary(op: &BinaryOp, l: &Value, r: &Value) -> Value {
    match op {
        BinaryOp::Or => Value::Bool(truthy(l) || truthy(r)),
        BinaryOp::And => Value::Bool(truthy(l) && truthy(r)),
        BinaryOp::Eq => Value::Bool(values_equal(l, r)),
        BinaryOp::Ne => Value::Bool(!values_equal(l, r)),
        BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge => compare_values(op, l, r),
        BinaryOp::Add => add_values(l, r),
        BinaryOp::Sub => sub_values(l, r),
        BinaryOp::Mul => mul_values(l, r),
        BinaryOp::Div => div_values(l, r),
        BinaryOp::Mod => mod_values(l, r),
        // `x in list` / `"ell" in "hello"`
        BinaryOp::In => Value::Bool(match r {
            Value::List(items) => items.iter().any(|it| values_equal(it, l)),
            Value::Text(hay) => hay.contains(&value_to_display(l)),
            Value::Record(fields) => fields.iter().any(|(k, _)| *k == value_to_display(l)),
            _ => false,
        }),
    }
}

fn to_i64(v: &Value) -> i64 {
    match v {
        Value::Int(n) => *n,
        Value::Money(n) => *n,
        Value::Float(f) => *f as i64,
        _ => 0,
    }
}

fn compare_values(op: &BinaryOp, l: &Value, r: &Value) -> Value {
    let ord = match (as_num(l), as_num(r)) {
        (Some(a), Some(b)) => a.partial_cmp(&b),
        _ => match (l, r) {
            (Value::Text(a), Value::Text(b)) => Some(a.cmp(b)),
            _ => None,
        },
    };
    let Some(ord) = ord else {
        return Value::Bool(false);
    };
    use std::cmp::Ordering::*;
    let b = match op {
        BinaryOp::Lt => ord == Less,
        BinaryOp::Le => ord != Greater,
        BinaryOp::Gt => ord == Greater,
        BinaryOp::Ge => ord != Less,
        _ => false,
    };
    Value::Bool(b)
}

/// Combine two numbers: integers stay integers, anything with a decimal becomes decimal.
fn num_op(
    l: &Value,
    r: &Value,
    int_op: fn(i64, i64) -> Option<i64>,
    f_op: fn(f64, f64) -> f64,
) -> Value {
    match (l, r) {
        (Value::Int(a), Value::Int(b)) => match int_op(*a, *b) {
            Some(n) => Value::Int(n),
            None => Value::Float(f_op(*a as f64, *b as f64)),
        },
        (Value::Money(a), Value::Money(b))
        | (Value::Money(a), Value::Int(b))
        | (Value::Int(a), Value::Money(b)) => match int_op(*a, *b) {
            Some(n) => Value::Money(n),
            None => Value::Float(f_op(*a as f64, *b as f64)),
        },
        _ => match (as_num(l), as_num(r)) {
            (Some(a), Some(b)) => Value::Float(f_op(a, b)),
            _ => Value::Null,
        },
    }
}

fn add_values(l: &Value, r: &Value) -> Value {
    match (l, r) {
        (Value::Text(a), b) => Value::Text(format!("{a}{}", value_to_display(b))),
        (a, Value::Text(b)) => Value::Text(format!("{}{b}", value_to_display(a))),
        (Value::List(a), Value::List(b)) => {
            Value::List(a.iter().chain(b.iter()).cloned().collect())
        }
        (Value::List(a), b) => {
            let mut out = a.clone();
            out.push(b.clone());
            Value::List(out)
        }
        _ => num_op(l, r, i64::checked_add, |a, b| a + b),
    }
}

fn sub_values(l: &Value, r: &Value) -> Value {
    if let Value::List(a) = l {
        return Value::List(a.iter().filter(|x| !values_equal(x, r)).cloned().collect());
    }
    num_op(l, r, i64::checked_sub, |a, b| a - b)
}

fn mul_values(l: &Value, r: &Value) -> Value {
    if let (Value::Text(s), Value::Int(n)) = (l, r) {
        return Value::Text(s.repeat((*n).max(0) as usize));
    }
    num_op(l, r, i64::checked_mul, |a, b| a * b)
}

fn div_values(l: &Value, r: &Value) -> Value {
    if as_num(r) == Some(0.0) {
        return Value::Null;
    }
    // `7 / 2` is 3.5 — whole-number results stay whole (`6 / 2` is 3).
    num_op(
        l,
        r,
        |a, b| if a % b == 0 { Some(a / b) } else { None },
        |a, b| a / b,
    )
}

fn mod_values(l: &Value, r: &Value) -> Value {
    if as_num(r) == Some(0.0) {
        return Value::Null;
    }
    num_op(l, r, i64::checked_rem, |a, b| a % b)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunOutcome {
    pub stdout: String,
    pub steps: u64,
}

const SCRIPT_CALL_DEPTH_LIMIT: u32 = 256;

pub struct ScriptInterpreter {
    script: CheckedScript,
    fns: HashMap<String, ScriptFn>,
}

impl ScriptInterpreter {
    pub fn new(script: CheckedScript) -> Self {
        let mut fns = HashMap::new();
        for f in script.script.fns() {
            fns.insert(f.name.clone(), f.clone());
        }
        Self { script, fns }
    }

    pub fn run_with_limits(
        &self,
        file: &str,
        source: &str,
        max_steps: u64,
    ) -> Result<RunOutcome, Vec<Diagnostic>> {
        let rt = RuntimeState {
            config: HashMap::new(),
            state: HashMap::new(),
            entities: HashMap::new(),
        };
        let mut env = ScriptEnv::default();
        let mut out = String::new();
        let mut steps = 0u64;
        for item in &self.script.script.items {
            if let ScriptItem::Stmt(stmt) = item {
                steps += 1;
                if steps > max_steps {
                    return Err(vec![step_limit_err(
                        file,
                        source,
                        &stmt_span_stmt(stmt),
                        max_steps,
                    )]);
                }
                exec_script_stmt(
                    stmt, &rt, &mut env, &self.fns, file, source, &mut out, false,
                )?;
            }
        }
        Ok(RunOutcome { stdout: out, steps })
    }
}

#[derive(Debug, Clone, Default)]
struct ScriptEnv {
    locals: HashMap<String, Value>,
    call_depth: u32,
    /// Top-level values, readable inside functions.
    globals: std::rc::Rc<HashMap<String, Value>>,
}

enum ExecControl {
    Next,
    Return(Value),
    Stop,
    Skip,
}

fn exec_script_stmt(
    stmt: &ScriptStmt,
    rt: &RuntimeState,
    env: &mut ScriptEnv,
    fns: &HashMap<String, ScriptFn>,
    file: &str,
    source: &str,
    out: &mut String,
    in_fn: bool,
) -> Result<ExecControl, Vec<Diagnostic>> {
    match stmt {
        ScriptStmt::Set { name, value, .. } => {
            let v = eval_script_expr(rt, value, env, fns, file, source)?;
            env.locals.insert(name.clone(), v);
            Ok(ExecControl::Next)
        }
        ScriptStmt::Say { parts, span } => {
            let line = format_say_parts(parts, rt, env, fns, file, source)?;
            let streamed = SAY_SINK.with(|s| match s.borrow_mut().as_mut() {
                Some(sink) => {
                    sink(&line);
                    true
                }
                None => false,
            });
            if !streamed {
                writeln!(out, "{line}").ok();
            }
            let _ = span;
            Ok(ExecControl::Next)
        }
        ScriptStmt::Give { value, .. } => {
            let v = eval_script_expr(rt, value, env, fns, file, source)?;
            Ok(ExecControl::Return(v))
        }
        ScriptStmt::If {
            cond,
            body,
            elif,
            else_body,
            ..
        } => {
            if truthy(&eval_script_expr(rt, cond, env, fns, file, source)?) {
                return exec_script_block(body, rt, env, fns, file, source, out, in_fn);
            }
            for branch in elif {
                if truthy(&eval_script_expr(rt, &branch.cond, env, fns, file, source)?) {
                    return exec_script_block(&branch.body, rt, env, fns, file, source, out, in_fn);
                }
            }
            if let Some(else_b) = else_body {
                exec_script_block(else_b, rt, env, fns, file, source, out, in_fn)
            } else {
                Ok(ExecControl::Next)
            }
        }
        ScriptStmt::Repeat { count, body, .. } => {
            let n = to_i64(&eval_script_expr(rt, count, env, fns, file, source)?);
            for _ in 0..n.max(0) {
                match exec_script_block(body, rt, env, fns, file, source, out, in_fn)? {
                    ExecControl::Return(v) => return Ok(ExecControl::Return(v)),
                    ExecControl::Stop => break,
                    ExecControl::Next | ExecControl::Skip => {}
                }
            }
            Ok(ExecControl::Next)
        }
        ScriptStmt::ForEachRange {
            var,
            from,
            to,
            step,
            body,
            ..
        } => {
            let start = to_i64(&eval_script_expr(rt, from, env, fns, file, source)?);
            let end = to_i64(&eval_script_expr(rt, to, env, fns, file, source)?);
            // `5 to 1` counts down; `by 2` sets the step size
            let size = match step {
                Some(st) => to_i64(&eval_script_expr(rt, st, env, fns, file, source)?)
                    .abs()
                    .max(1),
                None => 1,
            };
            let values: Vec<i64> = if start <= end {
                (start..=end).step_by(size as usize).collect()
            } else {
                (end..=start).rev().step_by(size as usize).collect()
            };
            for i in values {
                env.locals.insert(var.clone(), Value::Int(i));
                match exec_script_block(body, rt, env, fns, file, source, out, in_fn)? {
                    ExecControl::Return(v) => return Ok(ExecControl::Return(v)),
                    ExecControl::Stop => break,
                    ExecControl::Next | ExecControl::Skip => {}
                }
            }
            Ok(ExecControl::Next)
        }
        ScriptStmt::SetPath {
            root, path, value, ..
        } => {
            let v = eval_script_expr(rt, value, env, fns, file, source)?;
            let mut keys = Vec::with_capacity(path.len());
            for seg in path {
                keys.push(match seg {
                    PathSeg::Field(f) => f.clone(),
                    PathSeg::Key(k) => {
                        value_to_display(&eval_script_expr(rt, k, env, fns, file, source)?)
                    }
                });
            }
            let slot = env.locals.entry(root.clone()).or_insert(Value::Null);
            set_in(slot, &keys, v);
            Ok(ExecControl::Next)
        }
        ScriptStmt::Fail { message, span } => {
            let m = value_to_display(&eval_script_expr(rt, message, env, fns, file, source)?);
            Err(vec![Diagnostic::error(
                "E901",
                m,
                file,
                span.line,
                span.column,
                snippet_at(source, span.line, span.column, "fail"),
            )
            .with_help("handle it with `try` … `if it fails`")])
        }
        ScriptStmt::Try { body, handler, .. } => {
            match exec_script_block(body, rt, env, fns, file, source, out, in_fn) {
                Ok(ctrl) => Ok(ctrl),
                // the step limit protects the machine; don't let `try` hide it
                Err(diags) if diags.iter().any(|d| d.code == "E900") => Err(diags),
                Err(diags) => {
                    let msg = diags
                        .first()
                        .map(|d| d.message.clone())
                        .unwrap_or_else(|| "something went wrong".into());
                    if let Some(h) = handler {
                        env.locals.insert("error".into(), Value::Text(msg));
                        exec_script_block(h, rt, env, fns, file, source, out, in_fn)
                    } else {
                        Ok(ExecControl::Next)
                    }
                }
            }
        }
        ScriptStmt::Stop { .. } => Ok(ExecControl::Stop),
        ScriptStmt::Skip { .. } => Ok(ExecControl::Skip),
        ScriptStmt::ForEachList {
            var, list, body, ..
        } => {
            let items = match eval_script_expr(rt, list, env, fns, file, source)? {
                Value::List(items) => items,
                Value::Text(t) => t.chars().map(|c| Value::Text(c.to_string())).collect(),
                Value::Null => Vec::new(),
                other => vec![other],
            };
            for it in items {
                env.locals.insert(var.clone(), it);
                match exec_script_block(body, rt, env, fns, file, source, out, in_fn)? {
                    ExecControl::Return(v) => return Ok(ExecControl::Return(v)),
                    ExecControl::Stop => break,
                    ExecControl::Next | ExecControl::Skip => {}
                }
            }
            Ok(ExecControl::Next)
        }
        ScriptStmt::While { cond, body, .. } => {
            let mut guard = 0i64;
            while truthy(&eval_script_expr(rt, cond, env, fns, file, source)?) {
                guard += 1;
                if guard > 1_000_000 {
                    return Err(vec![Diagnostic::error(
                        "E900",
                        "while loop step limit exceeded",
                        file,
                        stmt_span_stmt(stmt).line,
                        stmt_span_stmt(stmt).column,
                        snippet_at(
                            source,
                            stmt_span_stmt(stmt).line,
                            stmt_span_stmt(stmt).column,
                            "while",
                        ),
                    )]);
                }
                match exec_script_block(body, rt, env, fns, file, source, out, in_fn)? {
                    ExecControl::Return(v) => return Ok(ExecControl::Return(v)),
                    ExecControl::Stop => break,
                    ExecControl::Next | ExecControl::Skip => {}
                }
            }
            Ok(ExecControl::Next)
        }
    }
}

fn exec_script_block(
    stmts: &[ScriptStmt],
    rt: &RuntimeState,
    env: &mut ScriptEnv,
    fns: &HashMap<String, ScriptFn>,
    file: &str,
    source: &str,
    out: &mut String,
    in_fn: bool,
) -> Result<ExecControl, Vec<Diagnostic>> {
    for stmt in stmts {
        match exec_script_stmt(stmt, rt, env, fns, file, source, out, in_fn)? {
            ExecControl::Next => {}
            other => return Ok(other),
        }
    }
    Ok(ExecControl::Next)
}

fn stmt_span_stmt(stmt: &ScriptStmt) -> Span {
    match stmt {
        ScriptStmt::Say { span, .. }
        | ScriptStmt::Set { span, .. }
        | ScriptStmt::If { span, .. }
        | ScriptStmt::Repeat { span, .. }
        | ScriptStmt::ForEachRange { span, .. }
        | ScriptStmt::ForEachList { span, .. }
        | ScriptStmt::SetPath { span, .. }
        | ScriptStmt::Try { span, .. }
        | ScriptStmt::Fail { span, .. }
        | ScriptStmt::Stop { span }
        | ScriptStmt::Skip { span }
        | ScriptStmt::While { span, .. }
        | ScriptStmt::Give { span, .. } => span.clone(),
    }
}

fn step_limit_err(file: &str, source: &str, span: &Span, max_steps: u64) -> Diagnostic {
    Diagnostic::error(
        "E900",
        format!("execution step limit ({max_steps}) exceeded"),
        file,
        span.line,
        span.column,
        snippet_at(source, span.line, span.column, "statement"),
    )
}

fn format_say_parts(
    parts: &[Expr],
    rt: &RuntimeState,
    env: &ScriptEnv,
    fns: &HashMap<String, ScriptFn>,
    file: &str,
    source: &str,
) -> Result<String, Vec<Diagnostic>> {
    let mut line = String::new();
    for part in parts {
        let v = eval_script_expr(rt, part, env, fns, file, source)?;
        line.push_str(&value_to_display(&v));
    }
    Ok(line)
}

fn eval_script_expr(
    rt: &RuntimeState,
    expr: &Expr,
    env: &ScriptEnv,
    fns: &HashMap<String, ScriptFn>,
    file: &str,
    source: &str,
) -> Result<Value, Vec<Diagnostic>> {
    match expr {
        Expr::Literal(l, _) => Ok(literal_to_value(l)),
        Expr::Path(p, span) if p.parts.len() > 1 => {
            // `person.name`, `order.items.count`
            let root = Expr::Path(
                PathExpr {
                    parts: vec![p.parts[0].clone()],
                    span: span.clone(),
                },
                span.clone(),
            );
            let mut v = eval_script_expr(rt, &root, env, fns, file, source)?;
            for f in &p.parts[1..] {
                v = get_field(&v, f);
            }
            Ok(v)
        }
        Expr::Path(p, span) => {
            if p.parts.len() == 1 {
                let n = &p.parts[0];
                if n == "nothing" || n == "none" {
                    return Ok(Value::Null);
                }
                if let Some(v) = env.locals.get(n) {
                    return Ok(v.clone());
                }
                if let Some(v) = rt.state.get(n) {
                    return Ok(v.clone());
                }
                if let Some(v) = env.globals.get(n) {
                    return Ok(v.clone());
                }
                if fns.contains_key(n) && !env.locals.contains_key(n) {
                    return call_script_fn(n, &[], &[], rt, env, fns, file, source, span);
                }
                if crate::check::is_script_builtin(n) {
                    return Ok(call_builtin(n, Vec::new()));
                }
            }
            Ok(Value::Null)
        }
        Expr::Call { name, args, .. }
            if matches!(name.as_str(), "__where" | "__each")
                && args.len() == 2
                && !fns.contains_key(name) =>
        {
            let items = as_list(eval_script_expr(rt, &args[0], env, fns, file, source)?);
            let mut out = Vec::new();
            for it in items {
                // `it` is the item; a record's fields are also visible by name
                let mut inner = env.clone();
                if let Value::Record(fields) = &it {
                    // the item's own fields win over outer names (like Kotlin's `it.age`)
                    for (k, v) in fields {
                        inner.locals.insert(k.clone(), v.clone());
                    }
                }
                inner.locals.insert("it".into(), it.clone());
                let v = eval_script_expr(rt, &args[1], &inner, fns, file, source)?;
                if name == "__each" {
                    out.push(v);
                } else if truthy(&v) {
                    out.push(it);
                }
            }
            Ok(Value::List(out))
        }
        Expr::Call {
            name,
            args,
            named,
            span,
        } => call_script_fn(name, args, named, rt, env, fns, file, source, span),
        Expr::Binary {
            op, left, right, ..
        } => {
            let l = eval_script_expr(rt, left, env, fns, file, source)?;
            let r = eval_script_expr(rt, right, env, fns, file, source)?;
            Ok(eval_binary(op, &l, &r))
        }
        Expr::Unary { op, expr, .. } => {
            let e = eval_script_expr(rt, expr, env, fns, file, source)?;
            Ok(match op {
                UnaryOp::Not => Value::Bool(!truthy(&e)),
                UnaryOp::Neg => neg_value(e),
            })
        }
        Expr::If {
            cond,
            then_br,
            else_br,
            ..
        } => {
            if truthy(&eval_script_expr(rt, cond, env, fns, file, source)?) {
                eval_script_expr(rt, then_br, env, fns, file, source)
            } else {
                eval_script_expr(rt, else_br, env, fns, file, source)
            }
        }
        _ => Ok(Value::Null),
    }
}

fn call_script_fn(
    name: &str,
    args: &[Expr],
    named: &[NamedArg],
    rt: &RuntimeState,
    env: &ScriptEnv,
    fns: &HashMap<String, ScriptFn>,
    file: &str,
    source: &str,
    span: &Span,
) -> Result<Value, Vec<Diagnostic>> {
    if name == "__record" && !fns.contains_key(name) {
        let mut fields = Vec::with_capacity(named.len());
        for n in named {
            fields.push((
                n.name.clone(),
                eval_script_expr(rt, &n.value, env, fns, file, source)?,
            ));
        }
        return Ok(Value::Record(fields));
    }
    let Some(defn) = fns.get(name) else {
        let mut vals = Vec::with_capacity(args.len());
        for a in args {
            vals.push(eval_script_expr(rt, a, env, fns, file, source)?);
        }
        return Ok(call_builtin(name, vals));
    };
    if env.call_depth >= SCRIPT_CALL_DEPTH_LIMIT {
        return Err(vec![Diagnostic::error(
            "E116",
            format!("call depth limit ({SCRIPT_CALL_DEPTH_LIMIT}) exceeded; possible infinite recursion in `{name}`"),
            file,
            span.line,
            span.column,
            snippet_at(source, span.line, span.column, name),
        )
        .with_help(format!("check the base case; also note a call takes the rest of its line, so `{name} n minus 1 plus {name} n minus 2` means {name}(n minus 1 plus {name}(n minus 2)). Put each call on its own line:\n  a = {name} n minus 1\n  b = {name} n minus 2\n  give a plus b"))]);
    }
    let mut call_env = ScriptEnv {
        locals: HashMap::new(),
        call_depth: env.call_depth + 1,
        globals: if env.call_depth == 0 {
            std::rc::Rc::new(env.locals.clone())
        } else {
            env.globals.clone()
        },
    };
    let mut pos = 0usize;
    for param in &defn.params {
        if pos < args.len() {
            let v = eval_script_expr(rt, &args[pos], env, fns, file, source)?;
            call_env.locals.insert(param.name.clone(), v);
            pos += 1;
            continue;
        }
        if let Some(n) = named.iter().find(|n| n.name == param.name) {
            let v = eval_script_expr(rt, &n.value, env, fns, file, source)?;
            call_env.locals.insert(param.name.clone(), v);
            continue;
        }
        if let Some(def) = &param.default {
            let v = eval_script_expr(rt, def, env, fns, file, source)?;
            call_env.locals.insert(param.name.clone(), v);
        }
    }
    for n in named {
        if defn.params.iter().any(|p| p.name == n.name) && !call_env.locals.contains_key(&n.name) {
            let v = eval_script_expr(rt, &n.value, env, fns, file, source)?;
            call_env.locals.insert(n.name.clone(), v);
        }
    }
    let mut sink = String::new();
    match exec_script_block(
        &defn.body,
        rt,
        &mut call_env,
        fns,
        file,
        source,
        &mut sink,
        true,
    )? {
        ExecControl::Return(v) => Ok(v),
        _ => Err(vec![Diagnostic::error(
            "E113",
            format!("function `{name}` finished without `give`"),
            file,
            span.line,
            span.column,
            snippet_at(source, span.line, span.column, name),
        )]),
    }
}

/// Print decimals without noise: `3.5`, `0.3` (not `0.30000000000000004`), `2` for `2.0`.
pub fn format_float(f: f64) -> String {
    if f.is_nan() {
        return "not a number".into();
    }
    if f.is_infinite() {
        return if f > 0.0 {
            "infinity".into()
        } else {
            "-infinity".into()
        };
    }
    if f.fract() == 0.0 && f.abs() < 1e15 {
        return format!("{}", f as i64);
    }
    let s = format!("{:.10}", f);
    let s = s.trim_end_matches('0').trim_end_matches('.');
    s.to_string()
}

pub fn value_to_display(v: &Value) -> String {
    match v {
        Value::Text(s) => s.clone(),
        Value::Int(n) => n.to_string(),
        Value::Money(m) => m.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Enum(s) => s.clone(),
        Value::Float(f) => format_float(*f),
        Value::List(items) => items
            .iter()
            .map(value_to_display)
            .collect::<Vec<_>>()
            .join(", "),
        Value::Record(fields) => format!(
            "({})",
            fields
                .iter()
                .map(|(k, v)| match v {
                    Value::Text(t) => format!("{k}: \"{t}\""),
                    other => format!("{k}: {}", value_to_display(other)),
                })
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Value::Null => "nothing".into(),
    }
}

/// Days since 1970-01-01 → (year, month, day). Dates are plain text like "2026-11-12" (UTC).
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// "2026-11-12" (optionally followed by a time) → days since 1970-01-01.
fn parse_date(t: &str) -> Option<i64> {
    let date = t.trim().split([' ', 'T']).next()?;
    let mut parts = date.split('-');
    let y: i64 = parts.next()?.parse().ok()?;
    let m: i64 = parts.next()?.parse().ok()?;
    let d: i64 = parts.next()?.parse().ok()?;
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Some(era * 146_097 + doe - 719_468)
}

/// Read a field of a record. `count`, `length` and `size` also work on lists and text.
fn get_field(v: &Value, f: &str) -> Value {
    if let Value::Record(fields) = v {
        if let Some((_, x)) = fields.iter().find(|(k, _)| k == f) {
            return x.clone();
        }
    }
    match f {
        "count" | "length" | "size" => call_builtin("count", vec![v.clone()]),
        "first" | "last" | "keys" | "values" | "upper" | "lower" | "trim" => {
            call_builtin(f, vec![v.clone()])
        }
        _ => Value::Null,
    }
}

/// Write `value` at `keys` inside `slot`, creating records along the way.
fn set_in(slot: &mut Value, keys: &[String], value: Value) {
    let Some((first, rest)) = keys.split_first() else {
        *slot = value;
        return;
    };
    if !matches!(slot, Value::Record(_)) {
        *slot = Value::Record(Vec::new());
    }
    if let Value::Record(fields) = slot {
        if let Some((_, existing)) = fields.iter_mut().find(|(k, _)| k == first) {
            set_in(existing, rest, value);
        } else {
            let mut fresh = Value::Null;
            set_in(&mut fresh, rest, value);
            fields.push((first.clone(), fresh));
        }
    }
}

fn as_list(v: Value) -> Vec<Value> {
    match v {
        Value::List(items) => items,
        Value::Null => Vec::new(),
        other => vec![other],
    }
}

fn sort_values(items: &mut [Value]) {
    items.sort_by(|a, b| match (as_num(a), as_num(b)) {
        (Some(x), Some(y)) => x.partial_cmp(&y).unwrap_or(std::cmp::Ordering::Equal),
        _ => value_to_display(a).cmp(&value_to_display(b)),
    });
}

fn float_or_int(f: f64) -> Value {
    if f.fract() == 0.0 && f.abs() < 9.0e15 {
        Value::Int(f as i64)
    } else {
        Value::Float(f)
    }
}

/// Built-in script functions: lists, text, and math.
pub fn call_builtin(name: &str, mut args: Vec<Value>) -> Value {
    let first = if args.is_empty() {
        Value::Null
    } else {
        args.remove(0)
    };
    let second = if args.is_empty() {
        Value::Null
    } else {
        args.remove(0)
    };
    match name {
        "list" => {
            let mut out = Vec::new();
            if !matches!(first, Value::Null) {
                out.push(first);
            }
            if !matches!(second, Value::Null) {
                out.push(second);
            }
            out.extend(args);
            Value::List(out)
        }
        "range" => {
            let (a, b) = (to_i64(&first), to_i64(&second));
            Value::List((a.min(b)..=a.max(b)).map(Value::Int).collect())
        }
        "__map" => {
            let mut fields: Vec<(String, Value)> = Vec::new();
            let mut all = vec![first, second];
            all.extend(args);
            for pair in all.chunks(2) {
                let k = value_to_display(&pair[0]);
                let v = pair.get(1).cloned().unwrap_or(Value::Null);
                if let Some(slot) = fields.iter_mut().find(|(x, _)| *x == k) {
                    slot.1 = v;
                } else {
                    fields.push((k, v));
                }
            }
            Value::Record(fields)
        }
        "ask" => {
            let question = match first {
                Value::Null => String::new(),
                other => value_to_display(&other),
            };
            let answer =
                ASK_SOURCE.with(|s| s.borrow_mut().as_mut().and_then(|ask| ask(&question)));
            match answer {
                Some(a) => {
                    let a = a.trim_end_matches(['\n', '\r']).to_string();
                    // numbers come back as numbers, so `age = ask "Age?"` can do math
                    // (only when the number reads back the same, so "007" stays text)
                    let t = a.trim();
                    match (t.parse::<i64>(), t.parse::<f64>()) {
                        (Ok(n), _) if n.to_string() == t => Value::Int(n),
                        (_, Ok(f)) if format_float(f) == t => Value::Float(f),
                        _ => Value::Text(a),
                    }
                }
                None => Value::Null,
            }
        }
        "__or_else" => match first {
            Value::Null => second,
            Value::Text(ref t) if t.is_empty() => second,
            other => other,
        },
        "__sortby" => {
            let field = value_to_display(&second);
            let desc = args.first().is_some_and(truthy);
            let mut l = as_list(first);
            if field.is_empty() {
                sort_values(&mut l);
            } else {
                l.sort_by(|a, b| {
                    let (x, y) = (get_field(a, &field), get_field(b, &field));
                    match (as_num(&x), as_num(&y)) {
                        (Some(p), Some(q)) => {
                            p.partial_cmp(&q).unwrap_or(std::cmp::Ordering::Equal)
                        }
                        _ => value_to_display(&x).cmp(&value_to_display(&y)),
                    }
                });
            }
            if desc {
                l.reverse();
            }
            Value::List(l)
        }
        "__group" => {
            let field = value_to_display(&second);
            let mut groups: Vec<(String, Value)> = Vec::new();
            for it in as_list(first) {
                let k = value_to_display(&get_field(&it, &field));
                match groups.iter_mut().find(|(g, _)| *g == k) {
                    Some((_, Value::List(l))) => l.push(it),
                    _ => groups.push((k, Value::List(vec![it]))),
                }
            }
            Value::Record(groups)
        }
        "take" | "__take_last" | "drop" => {
            let n = to_i64(&first).max(0) as usize;
            let is_text = matches!(second, Value::Text(_));
            let l: Vec<Value> = match second {
                Value::Text(t) => t.chars().map(|c| Value::Text(c.to_string())).collect(),
                other => as_list(other),
            };
            let n = n.min(l.len());
            let part: Vec<Value> = match name {
                "take" => l[..n].to_vec(),
                "drop" => l[n..].to_vec(),
                _ => l[l.len() - n..].to_vec(),
            };
            if is_text {
                Value::Text(part.iter().map(value_to_display).collect())
            } else {
                Value::List(part)
            }
        }
        "slice" => {
            let (a, b) = (to_i64(&first), to_i64(&second));
            let src = args.into_iter().next().unwrap_or(Value::Null);
            let is_text = matches!(src, Value::Text(_));
            let l: Vec<Value> = match src {
                Value::Text(t) => t.chars().map(|c| Value::Text(c.to_string())).collect(),
                other => as_list(other),
            };
            let lo = (a.max(1) - 1) as usize;
            let hi = (b.max(0) as usize).min(l.len());
            let part = if lo < hi {
                l[lo..hi].to_vec()
            } else {
                Vec::new()
            };
            if is_text {
                Value::Text(part.iter().map(value_to_display).collect())
            } else {
                Value::List(part)
            }
        }
        "today" | "now" => {
            let secs = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);
            let (y, m, d) = civil_from_days(secs.div_euclid(86_400));
            if name == "today" {
                Value::Text(format!("{y:04}-{m:02}-{d:02}"))
            } else {
                let s = secs.rem_euclid(86_400);
                Value::Text(format!(
                    "{y:04}-{m:02}-{d:02} {:02}:{:02}",
                    s / 3600,
                    (s % 3600) / 60
                ))
            }
        }
        "year" | "month" | "day" | "weekday" => match parse_date(&value_to_display(&first)) {
            Some(days) => {
                let (y, m, d) = civil_from_days(days);
                match name {
                    "year" => Value::Int(y),
                    "month" => Value::Int(m as i64),
                    "day" => Value::Int(d as i64),
                    _ => Value::Text(
                        [
                            "Thursday",
                            "Friday",
                            "Saturday",
                            "Sunday",
                            "Monday",
                            "Tuesday",
                            "Wednesday",
                        ][days.rem_euclid(7) as usize]
                            .into(),
                    ),
                }
            }
            None => Value::Null,
        },
        "__days_between" => match (
            parse_date(&value_to_display(&first)),
            parse_date(&value_to_display(&second)),
        ) {
            (Some(a), Some(b)) => Value::Int(b - a),
            _ => Value::Null,
        },
        "replace" => {
            let (from, to) = (
                value_to_display(&second),
                value_to_display(&args.into_iter().next().unwrap_or(Value::Null)),
            );
            let text = value_to_display(&first);
            Value::Text(if from.is_empty() {
                text
            } else {
                text.replace(&from, &to)
            })
        }
        "__starts_with" => {
            Value::Bool(value_to_display(&first).starts_with(&value_to_display(&second)))
        }
        "__ends_with" => {
            Value::Bool(value_to_display(&first).ends_with(&value_to_display(&second)))
        }
        "keys" => match first {
            Value::Record(f) => Value::List(f.into_iter().map(|(k, _)| Value::Text(k)).collect()),
            _ => Value::List(Vec::new()),
        },
        "values" => match first {
            Value::Record(f) => Value::List(f.into_iter().map(|(_, v)| v).collect()),
            other => Value::List(as_list(other)),
        },
        "__append" => {
            let mut l = as_list(first);
            l.push(second);
            Value::List(l)
        }
        "__remove" => {
            let l = as_list(first);
            Value::List(
                l.into_iter()
                    .filter(|x| !values_equal(x, &second))
                    .collect(),
            )
        }
        "count" | "size" | "length" => match first {
            Value::List(l) => Value::Int(l.len() as i64),
            Value::Record(r) => Value::Int(r.len() as i64),
            Value::Text(t) => Value::Int(t.chars().count() as i64),
            Value::Null => Value::Int(0),
            _ => Value::Int(1),
        },
        "first" => match first {
            Value::List(l) => l.into_iter().next().unwrap_or(Value::Null),
            Value::Text(t) => t
                .chars()
                .next()
                .map(|c| Value::Text(c.into()))
                .unwrap_or(Value::Null),
            other => other,
        },
        "last" => match first {
            Value::List(l) => l.into_iter().last().unwrap_or(Value::Null),
            Value::Text(t) => t
                .chars()
                .last()
                .map(|c| Value::Text(c.into()))
                .unwrap_or(Value::Null),
            other => other,
        },
        "item" if matches!(second, Value::Record(_)) => {
            get_field(&second, &value_to_display(&first))
        }
        "item" => {
            let idx = to_i64(&first);
            let l = match second {
                Value::Text(t) => t.chars().map(|c| Value::Text(c.into())).collect(),
                other => as_list(other),
            };
            if idx >= 1 && (idx as usize) <= l.len() {
                l[idx as usize - 1].clone()
            } else {
                Value::Null
            }
        }
        "sum" => {
            let total = as_list(first).iter().filter_map(as_num).sum::<f64>();
            float_or_int(total)
        }
        "average" => {
            let l = as_list(first);
            let nums: Vec<f64> = l.iter().filter_map(as_num).collect();
            if nums.is_empty() {
                Value::Null
            } else {
                float_or_int(nums.iter().sum::<f64>() / nums.len() as f64)
            }
        }
        "min" | "max" => {
            let mut l = if matches!(second, Value::Null) {
                as_list(first)
            } else {
                vec![first, second]
            };
            sort_values(&mut l);
            if name == "min" {
                l.into_iter().next().unwrap_or(Value::Null)
            } else {
                l.into_iter().last().unwrap_or(Value::Null)
            }
        }
        "sort" | "sorted" => {
            let mut l = as_list(first);
            sort_values(&mut l);
            Value::List(l)
        }
        "reverse" | "reversed" => match first {
            Value::Text(t) => Value::Text(t.chars().rev().collect()),
            other => {
                let mut l = as_list(other);
                l.reverse();
                Value::List(l)
            }
        },
        "unique" => {
            let mut out: Vec<Value> = Vec::new();
            for v in as_list(first) {
                if !out.iter().any(|o| values_equal(o, &v)) {
                    out.push(v);
                }
            }
            Value::List(out)
        }
        "join" => {
            let sep = match second {
                Value::Null => ", ".to_string(),
                other => value_to_display(&other),
            };
            Value::Text(
                as_list(first)
                    .iter()
                    .map(value_to_display)
                    .collect::<Vec<_>>()
                    .join(&sep),
            )
        }
        "split" => {
            let text = value_to_display(&first);
            let sep = match second {
                Value::Null => " ".to_string(),
                other => value_to_display(&other),
            };
            Value::List(
                text.split(sep.as_str())
                    .filter(|p| !p.is_empty())
                    .map(|p| Value::Text(p.to_string()))
                    .collect(),
            )
        }
        "upper" => Value::Text(value_to_display(&first).to_uppercase()),
        "lower" => Value::Text(value_to_display(&first).to_lowercase()),
        "trim" => Value::Text(value_to_display(&first).trim().to_string()),
        "text" => Value::Text(value_to_display(&first)),
        "number" => {
            let t = value_to_display(&first);
            let t = t.trim();
            if let Ok(n) = t.parse::<i64>() {
                Value::Int(n)
            } else if let Ok(f) = t.parse::<f64>() {
                Value::Float(f)
            } else {
                Value::Null
            }
        }
        "round" => {
            let Some(x) = as_num(&first) else {
                return Value::Null;
            };
            match as_num(&second) {
                Some(places) if places > 0.0 => {
                    let m = 10f64.powi(places as i32);
                    float_or_int((x * m).round() / m)
                }
                _ => Value::Int(x.round() as i64),
            }
        }
        "floor" => as_num(&first)
            .map(|x| Value::Int(x.floor() as i64))
            .unwrap_or(Value::Null),
        "ceil" => as_num(&first)
            .map(|x| Value::Int(x.ceil() as i64))
            .unwrap_or(Value::Null),
        "abs" => match first {
            Value::Int(n) => Value::Int(n.abs()),
            other => as_num(&other)
                .map(|x| Value::Float(x.abs()))
                .unwrap_or(Value::Null),
        },
        "sqrt" => as_num(&first)
            .map(|x| float_or_int(x.sqrt()))
            .unwrap_or(Value::Null),
        "random" => {
            // Deterministic-per-run pseudo random in [first, second] (defaults 1..100).
            use std::time::{SystemTime, UNIX_EPOCH};
            let lo = if matches!(first, Value::Null) {
                1
            } else {
                to_i64(&first)
            };
            let hi = if matches!(second, Value::Null) {
                100
            } else {
                to_i64(&second)
            };
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.subsec_nanos() as i64)
                .unwrap_or(7);
            let span = (hi - lo).abs() + 1;
            Value::Int(lo.min(hi) + nanos.rem_euclid(span))
        }
        _ => Value::Null,
    }
}
