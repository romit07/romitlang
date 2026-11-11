use crate::ast::*;
use crate::check::{CheckedProgram, CheckedScript};
use crate::diag::{snippet_at, Diagnostic};
use std::collections::HashMap;
use std::fmt::Write as _;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Text(String),
    Int(i64),
    Money(i64),
    Bool(bool),
    Enum(String),
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

fn values_equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Text(x), Value::Text(y)) => x == y,
        (Value::Int(x), Value::Int(y)) => x == y,
        (Value::Money(x), Value::Money(y)) => x == y,
        (Value::Int(x), Value::Money(y)) | (Value::Money(x), Value::Int(y)) => x == y,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Enum(x), Value::Enum(y)) => x == y,
        (Value::Text(x), Value::Enum(y)) => x == y,
        (Value::Enum(x), Value::Text(y)) => x == y,
        _ => false,
    }
}

fn literal_to_value(l: &Literal) -> Value {
    match l {
        Literal::Int(n) => Value::Int(*n),
        Literal::Decimal(s) => Value::Money(s.parse().unwrap_or(0)),
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
        _ => true,
    }
}

fn neg_value(v: Value) -> Value {
    match v {
        Value::Int(n) => Value::Int(-n),
        Value::Money(m) => Value::Money(-m),
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
        BinaryOp::In => Value::Bool(false),
    }
}

fn to_i64(v: &Value) -> i64 {
    match v {
        Value::Int(n) => *n,
        Value::Money(n) => *n,
        _ => 0,
    }
}

fn compare_values(op: &BinaryOp, l: &Value, r: &Value) -> Value {
    let (li, ri) = (to_i64(l), to_i64(r));
    let b = match op {
        BinaryOp::Lt => li < ri,
        BinaryOp::Le => li <= ri,
        BinaryOp::Gt => li > ri,
        BinaryOp::Ge => li >= ri,
        _ => false,
    };
    Value::Bool(b)
}

fn add_values(l: &Value, r: &Value) -> Value {
    Value::Int(to_i64(l) + to_i64(r))
}

fn sub_values(l: &Value, r: &Value) -> Value {
    Value::Int(to_i64(l) - to_i64(r))
}

fn mul_values(l: &Value, r: &Value) -> Value {
    Value::Int(to_i64(l) * to_i64(r))
}

fn div_values(l: &Value, r: &Value) -> Value {
    let b = to_i64(r);
    if b == 0 {
        return Value::Null;
    }
    Value::Int(to_i64(l) / b)
}

fn mod_values(l: &Value, r: &Value) -> Value {
    let b = to_i64(r);
    if b == 0 {
        return Value::Null;
    }
    Value::Int(to_i64(l) % b)
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
                match exec_script_stmt(
                    stmt, &rt, &mut env, &self.fns, file, source, &mut out, false,
                )? {
                    ExecControl::Next => {}
                    ExecControl::Return(_) => {}
                }
            }
        }
        Ok(RunOutcome { stdout: out, steps })
    }
}

#[derive(Debug, Clone, Default)]
struct ScriptEnv {
    locals: HashMap<String, Value>,
    call_depth: u32,
}

enum ExecControl {
    Next,
    Return(Value),
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
            writeln!(out, "{line}").ok();
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
                    ExecControl::Next => {}
                }
            }
            Ok(ExecControl::Next)
        }
        ScriptStmt::ForEachRange {
            var,
            from,
            to,
            body,
            ..
        } => {
            let start = to_i64(&eval_script_expr(rt, from, env, fns, file, source)?);
            let end = to_i64(&eval_script_expr(rt, to, env, fns, file, source)?);
            let (lo, hi) = if start <= end {
                (start, end)
            } else {
                (end, start)
            };
            for i in lo..=hi {
                env.locals.insert(var.clone(), Value::Int(i));
                match exec_script_block(body, rt, env, fns, file, source, out, in_fn)? {
                    ExecControl::Return(v) => return Ok(ExecControl::Return(v)),
                    ExecControl::Next => {}
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
                    ExecControl::Next => {}
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
            ExecControl::Return(v) => return Ok(ExecControl::Return(v)),
            ExecControl::Next => {}
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
        Expr::Path(p, span) => {
            if p.parts.len() == 1 {
                let n = &p.parts[0];
                if let Some(v) = env.locals.get(n) {
                    return Ok(v.clone());
                }
                if let Some(v) = rt.state.get(n) {
                    return Ok(v.clone());
                }
                if fns.contains_key(n) && !env.locals.contains_key(n) {
                    return call_script_fn(n, &[], &[], rt, env, fns, file, source, span);
                }
            }
            Ok(Value::Null)
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
    let Some(defn) = fns.get(name) else {
        return Ok(Value::Null);
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
        .with_help("add a base case or simplify mutual recursion")]);
    }
    let mut call_env = ScriptEnv {
        locals: HashMap::new(),
        call_depth: env.call_depth + 1,
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
        ExecControl::Next => Err(vec![Diagnostic::error(
            "E113",
            format!("function `{name}` finished without `give`"),
            file,
            span.line,
            span.column,
            snippet_at(source, span.line, span.column, name),
        )]),
    }
}

fn clone_value(v: &Value) -> Value {
    match v {
        Value::Text(s) => Value::Text(s.clone()),
        Value::Int(n) => Value::Int(*n),
        Value::Money(m) => Value::Money(*m),
        Value::Bool(b) => Value::Bool(*b),
        Value::Enum(s) => Value::Enum(s.clone()),
        Value::Null => Value::Null,
    }
}

pub fn value_to_display(v: &Value) -> String {
    match v {
        Value::Text(s) => s.clone(),
        Value::Int(n) => n.to_string(),
        Value::Money(m) => m.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Enum(s) => s.clone(),
        Value::Null => "null".into(),
    }
}

fn format_say_value(v: &Value, env: &Env) -> String {
    let mut text = value_to_display(v);
    if let Value::Text(s) = v {
        text = interpolate_say_string(s, env);
    }
    text
}

fn interpolate_say_string(s: &str, env: &Env) -> String {
    if let Some(idx) = s.rfind(", ") {
        let ident = s[idx + 2..].trim();
        if !ident.is_empty()
            && ident.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
            && !ident.chars().next().is_some_and(|c| c.is_ascii_digit())
        {
            if let Some(v) = env.locals.get(ident) {
                return format!("{}{}", &s[..idx + 2], value_to_display(v));
            }
            if let Some(v) = env.field(ident, ident) {
                return format!("{}{}", &s[..idx + 2], value_to_display(&v));
            }
        }
    }
    s.to_string()
}
