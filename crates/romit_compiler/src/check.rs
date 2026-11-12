use crate::ast::*;
use crate::diag::{snippet_at, Diagnostic};
use crate::suggest::did_you_mean;
use crate::types::{type_from_ast, RomitType};
use std::collections::{HashMap, HashSet};

#[derive(Clone)]
pub struct CheckedScript {
    pub script: Script,
}

#[derive(Clone)]
pub enum CheckedUnit {
    Spec(CheckedProgram),
    Script(CheckedScript),
}

impl CheckedUnit {
    pub fn as_spec(&self) -> Option<&CheckedProgram> {
        match self {
            CheckedUnit::Spec(p) => Some(p),
            CheckedUnit::Script(_) => None,
        }
    }

    pub fn is_script(&self) -> bool {
        matches!(self, CheckedUnit::Script(_))
    }
}

#[derive(Clone)]
pub struct CheckedProgram {
    pub spec: Spec,
    pub enums: HashMap<String, Vec<String>>,
    pub entity_fields: HashMap<String, HashMap<String, RomitType>>,
    pub state_fields: HashMap<String, RomitType>,
    pub config_fields: HashMap<String, RomitType>,
    pub events: HashMap<String, EventDecl>,
    pub outputs: HashMap<String, OutputDecl>,
    pub defines: HashMap<String, DefineDecl>,
    pub machines: HashMap<String, MachineDecl>,
    pub handlers: Vec<HandlerDecl>,
    pub invariants: Vec<InvariantDecl>,
    pub limits: Vec<LimitDecl>,
    pub tests: Vec<TestDecl>,
}

pub fn check_program(
    program: Program,
    file: &str,
    source: &str,
) -> Result<CheckedUnit, Vec<Diagnostic>> {
    match program {
        Program::Spec(spec) => check(spec, file, source).map(CheckedUnit::Spec),
        Program::Script(script) => check_script(script, file, source).map(CheckedUnit::Script),
    }
}

pub fn check_script(
    script: Script,
    file: &str,
    source: &str,
) -> Result<CheckedScript, Vec<Diagnostic>> {
    let mut diags = Vec::new();
    if script.items.is_empty() {
        diags.push(Diagnostic::error(
            "E006",
            "script is empty; add at least one `say`, `print`, `x = …`, `fn` function, or control-flow statement",
            file,
            script.span.line,
            script.span.column,
            snippet_at(source, script.span.line, script.span.column, "script"),
        ));
    }
    let fn_map: HashMap<String, &ScriptFn> = script
        .items
        .iter()
        .filter_map(|it| match it {
            ScriptItem::Fn(f) => Some((f.name.clone(), f)),
            _ => None,
        })
        .collect();

    let mut seen_fns = HashSet::new();
    for f in script.fns() {
        if !seen_fns.insert(f.name.clone()) {
            diags.push(
                type_err(
                    file,
                    source,
                    &f.span,
                    "E110",
                    format!("duplicate function `{}`", f.name),
                )
                .with_help("each `name:`, `fn name`, or `function name` definition must have a unique name"),
            );
        }
    }

    // Functions may read values set at the top level (like Kotlin top-level `val`s).
    let top_level: Vec<String> = script
        .items
        .iter()
        .filter_map(|it| match it {
            ScriptItem::Stmt(ScriptStmt::Set { name, .. }) => Some(name.clone()),
            ScriptItem::Stmt(ScriptStmt::SetPath { root, .. }) => Some(root.clone()),
            _ => None,
        })
        .collect();
    let mut scope: Vec<String> = Vec::new();
    for item in &script.items {
        match item {
            ScriptItem::Fn(f) => {
                let mut param_names = HashSet::new();
                for p in &f.params {
                    if scope.contains(&p.name) {
                        diags.push(type_err(
                            file,
                            source,
                            &p.span,
                            "E111",
                            format!(
                                "parameter `{name}` shadows variable in outer scope",
                                name = p.name
                            ),
                        ));
                    }
                    if !param_names.insert(p.name.clone()) {
                        diags.push(type_err(
                            file,
                            source,
                            &p.span,
                            "E112",
                            format!(
                                "duplicate parameter `{name}` in `{fname}`",
                                name = p.name,
                                fname = f.name
                            ),
                        ));
                    }
                }
                let mut fn_scope: Vec<String> = f.params.iter().map(|p| p.name.clone()).collect();
                fn_scope.extend(top_level.iter().cloned());
                check_script_block(
                    &f.body,
                    &mut fn_scope,
                    &fn_map,
                    file,
                    source,
                    &mut diags,
                    true,
                );
                match block_must_return(&f.body) {
                    BlockFlow::MissingReturn(span) => {
                        diags.push(
                            type_err(
                                file,
                                source,
                                &span,
                                "E113",
                                format!(
                                    "function `{name}` can finish without `give`",
                                    name = f.name
                                ),
                            )
                            .with_help(
                                "end every path with `give <value>` or add `give` in each branch",
                            ),
                        );
                    }
                    BlockFlow::Unreachable(span) => {
                        diags.push(
                            type_err(
                                file,
                                source,
                                &span,
                                "E114",
                                "unreachable statement after `give`".into(),
                            )
                            .with_help("remove code below `give` or move it above the return"),
                        );
                    }
                    BlockFlow::Ok => {}
                }
            }
            ScriptItem::Stmt(stmt) => {
                check_script_stmt(stmt, &mut scope, &fn_map, file, source, &mut diags, false);
            }
        }
    }

    if diags
        .iter()
        .any(|d| d.severity == crate::diag::Severity::Error)
    {
        return Err(diags);
    }
    Ok(CheckedScript { script })
}

fn check_script_block(
    stmts: &[ScriptStmt],
    scope: &mut Vec<String>,
    fns: &HashMap<String, &ScriptFn>,
    file: &str,
    source: &str,
    diags: &mut Vec<Diagnostic>,
    in_fn: bool,
) {
    let mut after_give = false;
    for stmt in stmts {
        if after_give && in_fn {
            let span = stmt_span(stmt);
            diags.push(type_err(
                file,
                source,
                &span,
                "E114",
                "unreachable statement after `give`".into(),
            ));
        }
        if matches!(stmt, ScriptStmt::Give { .. }) && in_fn {
            after_give = true;
        }
        check_script_stmt(stmt, scope, fns, file, source, diags, in_fn);
    }
}

fn check_script_stmt(
    stmt: &ScriptStmt,
    scope: &mut Vec<String>,
    fns: &HashMap<String, &ScriptFn>,
    file: &str,
    source: &str,
    diags: &mut Vec<Diagnostic>,
    in_fn: bool,
) {
    match stmt {
        ScriptStmt::Give { value, span } => {
            if !in_fn {
                diags.push(type_err(
                    file,
                    source,
                    span,
                    "P022",
                    "`give` is only allowed inside a function body (`fn name …`)".into(),
                ));
            }
            check_script_expr(value, scope, fns, file, source, diags);
        }
        ScriptStmt::Set { name, value, .. } => {
            check_script_expr(value, scope, fns, file, source, diags);
            if !scope.contains(name) {
                scope.push(name.clone());
            }
        }
        ScriptStmt::Say { parts, span } => {
            for part in parts {
                check_script_expr(part, scope, fns, file, source, diags);
                if !expr_is_printable(part) {
                    diags.push(type_err(
                        file,
                        source,
                        span,
                        "E107",
                        "`say` or `print` needs a text, number, or boolean value".into(),
                    ));
                }
            }
        }
        ScriptStmt::If {
            cond,
            body,
            elif,
            else_body,
            ..
        } => {
            check_script_expr(cond, scope, fns, file, source, diags);
            let mut branch_scope = scope.clone();
            check_script_block(body, &mut branch_scope, fns, file, source, diags, in_fn);
            for eif in elif {
                check_script_expr(&eif.cond, scope, fns, file, source, diags);
                let mut elif_scope = scope.clone();
                check_script_block(&eif.body, &mut elif_scope, fns, file, source, diags, in_fn);
            }
            if let Some(else_b) = else_body {
                let mut else_scope = scope.clone();
                check_script_block(else_b, &mut else_scope, fns, file, source, diags, in_fn);
            }
        }
        ScriptStmt::Repeat { count, body, .. } => {
            check_script_expr(count, scope, fns, file, source, diags);
            let mut loop_scope = scope.clone();
            check_script_block(body, &mut loop_scope, fns, file, source, diags, in_fn);
        }
        ScriptStmt::ForEachRange {
            var,
            from,
            to,
            step,
            body,
            span,
        } => {
            check_script_expr(from, scope, fns, file, source, diags);
            check_script_expr(to, scope, fns, file, source, diags);
            if let Some(st) = step {
                check_script_expr(st, scope, fns, file, source, diags);
            }
            let mut loop_scope = scope.clone();
            if loop_scope.contains(var) {
                diags.push(type_err(
                    file,
                    source,
                    span,
                    "E111",
                    format!("loop variable `{var}` shadows an outer name"),
                ));
            }
            loop_scope.push(var.clone());
            check_script_block(body, &mut loop_scope, fns, file, source, diags, in_fn);
        }
        ScriptStmt::While { cond, body, .. } => {
            check_script_expr(cond, scope, fns, file, source, diags);
            let mut loop_scope = scope.clone();
            check_script_block(body, &mut loop_scope, fns, file, source, diags, in_fn);
        }
        ScriptStmt::ForEachList {
            var, list, body, ..
        } => {
            check_script_expr(list, scope, fns, file, source, diags);
            let mut loop_scope = scope.clone();
            if !loop_scope.contains(var) {
                loop_scope.push(var.clone());
            }
            check_script_block(body, &mut loop_scope, fns, file, source, diags, in_fn);
        }
        ScriptStmt::SetPath {
            root,
            path,
            value,
            span,
        } => {
            check_script_expr(value, scope, fns, file, source, diags);
            for seg in path {
                if let PathSeg::Key(k) = seg {
                    check_script_expr(k, scope, fns, file, source, diags);
                }
            }
            if !scope.contains(root) {
                diags.push(
                    type_err(
                        file,
                        source,
                        span,
                        "E108",
                        format!("undefined name `{root}`"),
                    )
                    .with_help(format!(
                        "create it first, e.g. `{root} = with name \"Ada\"`"
                    )),
                );
            }
        }
        ScriptStmt::Try { body, handler, .. } => {
            let mut body_scope = scope.clone();
            check_script_block(body, &mut body_scope, fns, file, source, diags, in_fn);
            if let Some(h) = handler {
                let mut h_scope = scope.clone();
                h_scope.push("error".into());
                check_script_block(h, &mut h_scope, fns, file, source, diags, in_fn);
            }
        }
        ScriptStmt::Fail { message, .. } => {
            check_script_expr(message, scope, fns, file, source, diags);
        }
        ScriptStmt::Stop { .. } | ScriptStmt::Skip { .. } => {}
    }
}

/// Functions every script can call without defining them.
pub const SCRIPT_BUILTINS: &[&str] = &[
    "list",
    "count",
    "size",
    "length",
    "first",
    "last",
    "item",
    "sum",
    "min",
    "max",
    "average",
    "sort",
    "sorted",
    "reverse",
    "reversed",
    "join",
    "split",
    "upper",
    "lower",
    "trim",
    "round",
    "floor",
    "ceil",
    "abs",
    "sqrt",
    "number",
    "text",
    "random",
    "range",
    "unique",
    "__append",
    "__remove",
    "keys",
    "values",
    "replace",
    "ask",
    "today",
    "now",
    "weekday",
    "year",
    "month",
    "day",
    "take",
    "drop",
    "slice",
    "__record",
    "__map",
    "__or_else",
    "__nothing",
    "__where",
    "__each",
    "__sortby",
    "__group",
    "__take_last",
    "__starts_with",
    "__ends_with",
    "__days_between",
];

pub fn is_script_builtin(name: &str) -> bool {
    SCRIPT_BUILTINS.contains(&name)
}

enum BlockFlow {
    Ok,
    MissingReturn(Span),
    Unreachable(Span),
}

fn block_must_return(stmts: &[ScriptStmt]) -> BlockFlow {
    let mut seen_give = false;
    for stmt in stmts {
        if seen_give {
            return BlockFlow::Unreachable(stmt_span(stmt));
        }
        match stmt {
            ScriptStmt::Give { .. } | ScriptStmt::Fail { .. } => seen_give = true,
            ScriptStmt::If {
                body,
                elif,
                else_body,
                span,
                ..
            } => {
                let mut all_return = !body.is_empty();
                if all_return {
                    all_return = matches!(block_must_return(body), BlockFlow::Ok);
                }
                for eif in elif {
                    if eif.body.is_empty() {
                        all_return = false;
                        break;
                    }
                    if !matches!(block_must_return(&eif.body), BlockFlow::Ok) {
                        all_return = false;
                        break;
                    }
                }
                if let Some(else_b) = else_body {
                    if else_b.is_empty() || !matches!(block_must_return(else_b), BlockFlow::Ok) {
                        all_return = false;
                    }
                } else {
                    all_return = false;
                }
                // An `if` whose every branch gives counts as a give; otherwise
                // keep checking — a later `give` can still cover this path.
                if all_return {
                    seen_give = true;
                }
                let _ = span;
            }
            _ => {}
        }
    }
    if seen_give {
        BlockFlow::Ok
    } else {
        BlockFlow::MissingReturn(stmts.last().map(stmt_span).unwrap_or(Span {
            line: 1,
            column: 1,
            end_line: 1,
            end_column: 1,
        }))
    }
}

fn stmt_span(stmt: &ScriptStmt) -> Span {
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

fn expr_is_printable(expr: &Expr) -> bool {
    match expr {
        Expr::Literal(_, _)
        | Expr::Path(_, _)
        | Expr::Binary { .. }
        | Expr::Unary { .. }
        | Expr::Call { .. }
        | Expr::If { .. } => true,
        _ => false,
    }
}

fn check_script_expr(
    expr: &Expr,
    scope: &[String],
    fns: &HashMap<String, &ScriptFn>,
    file: &str,
    source: &str,
    diags: &mut Vec<Diagnostic>,
) {
    match expr {
        Expr::Path(p, span) if p.parts.len() == 1 => {
            let name = &p.parts[0];
            if matches!(name.as_str(), "true" | "false" | "nothing" | "none") {
                return;
            }
            if fns.contains_key(name) {
                return;
            }
            if !scope.iter().any(|v| v == name) && !is_script_builtin(name) {
                let mut candidates: Vec<String> = scope.to_vec();
                candidates.extend(fns.keys().cloned());
                let mut msg = format!("undefined name `{name}`");
                if let Some(hint) = did_you_mean(name, &candidates) {
                    msg.push_str(&format!("; did you mean `{hint}`?"));
                }
                let mut d = type_err(file, source, span, "E108", msg);
                d = d.with_help(format!(
                    "create it first with `{name} = …`, define `fn {name} …`, or check the spelling"
                ));
                diags.push(d);
            }
        }
        Expr::Path(p, span) => {
            // `person.name`: the first word must be a known value
            let root = Expr::Path(
                PathExpr {
                    parts: vec![p.parts[0].clone()],
                    span: span.clone(),
                },
                span.clone(),
            );
            check_script_expr(&root, scope, fns, file, source, diags);
        }
        Expr::Binary { left, right, .. } => {
            check_script_expr(left, scope, fns, file, source, diags);
            check_script_expr(right, scope, fns, file, source, diags);
        }
        Expr::Unary { expr, .. } => {
            check_script_expr(expr, scope, fns, file, source, diags);
        }
        Expr::Literal(_, _) => {}
        Expr::Call {
            name,
            args,
            named,
            span,
        } => {
            if matches!(name.as_str(), "__where" | "__each") && args.len() == 2 {
                // the body sees `it` and the item's fields by name
                check_script_expr(&args[0], scope, fns, file, source, diags);
                let mut inner = scope.to_vec();
                inner.push("it".into());
                let mut found = Vec::new();
                check_script_expr(&args[1], &inner, fns, file, source, &mut found);
                diags.extend(
                    found
                        .into_iter()
                        .filter(|d| !(d.code == "E108" && d.message.starts_with("undefined name"))),
                );
                return;
            }
            if !fns.contains_key(name) && is_script_builtin(name) {
                for a in args {
                    check_script_expr(a, scope, fns, file, source, diags);
                }
                for n in named {
                    check_script_expr(&n.value, scope, fns, file, source, diags);
                }
                return;
            }
            let Some(defn) = fns.get(name) else {
                let mut candidates: Vec<String> = fns.keys().cloned().collect();
                candidates.extend(
                    SCRIPT_BUILTINS
                        .iter()
                        .filter(|b| !b.starts_with('_'))
                        .map(|b| b.to_string()),
                );
                let mut msg = format!("undefined function `{name}`");
                if let Some(hint) = did_you_mean(name, &candidates) {
                    msg.push_str(&format!("; did you mean `{hint}`?"));
                }
                diags.push(type_err(file, source, span, "E115", msg).with_help(
                    "define it with `name: …`, `fn name …`, or `function name …` in the same file",
                ));
                for a in args {
                    check_script_expr(a, scope, fns, file, source, diags);
                }
                for n in named {
                    check_script_expr(&n.value, scope, fns, file, source, diags);
                }
                return;
            };
            for a in args {
                check_script_expr(a, scope, fns, file, source, diags);
            }
            for n in named {
                check_script_expr(&n.value, scope, fns, file, source, diags);
            }
            let mut bound = HashSet::new();
            for (i, p) in defn.params.iter().enumerate() {
                if i < args.len() {
                    bound.insert(p.name.clone());
                }
            }
            for n in named {
                if !defn.params.iter().any(|p| p.name == n.name) {
                    diags.push(type_err(
                        file,
                        source,
                        &n.span,
                        "E112",
                        format!("`{name}` has no parameter `{pname}`", pname = n.name),
                    ));
                } else {
                    bound.insert(n.name.clone());
                }
            }
            let required = defn.params.iter().filter(|p| p.default.is_none()).count();
            let positional = args.len();
            // a required parameter can also be given by name: `greet with name "Bo"`
            let missing = defn
                .params
                .iter()
                .enumerate()
                .filter(|(i, p)| {
                    p.default.is_none() && *i >= positional && !bound.contains(&p.name)
                })
                .count();
            if missing > 0 {
                diags.push(
                    type_err(
                        file,
                        source,
                        span,
                        "E112",
                        format!(
                            "`{name}` needs at least {required} argument(s), got {positional}",
                            name = name
                        ),
                    )
                    .with_help("pass values after the name or use `with param value`"),
                );
            }
            if positional > defn.params.len() {
                diags.push(type_err(
                    file,
                    source,
                    span,
                    "E112",
                    format!(
                        "`{name}` takes at most {} argument(s), got {positional}",
                        defn.params.len()
                    ),
                ));
            }
            for p in &defn.params {
                if p.default.is_none() && !bound.contains(&p.name) {
                    diags.push(type_err(
                        file,
                        source,
                        span,
                        "E112",
                        format!("missing argument `{pname}` for `{name}`", pname = p.name),
                    ));
                }
            }
            for p in &defn.params {
                if scope.contains(&p.name) {
                    // only flag when param name used as param in fn def — checked at defn site
                }
            }
        }
        Expr::If {
            cond,
            then_br,
            else_br,
            ..
        } => {
            check_script_expr(cond, scope, fns, file, source, diags);
            check_script_expr(then_br, scope, fns, file, source, diags);
            check_script_expr(else_br, scope, fns, file, source, diags);
        }
        Expr::Aggregate { filter, .. } => {
            if let Some(f) = filter {
                check_script_expr(f, scope, fns, file, source, diags);
            }
        }
        Expr::Index { key, .. } => {
            check_script_expr(key, scope, fns, file, source, diags);
        }
        Expr::ForEach { body, .. } => {
            check_script_expr(body, scope, fns, file, source, diags);
        }
    }
}

pub fn check(spec: Spec, file: &str, source: &str) -> Result<CheckedProgram, Vec<Diagnostic>> {
    let mut diags = Vec::new();
    let mut enums = HashMap::new();
    let mut entity_fields = HashMap::new();
    let mut state_fields = HashMap::new();
    let mut config_fields = HashMap::new();
    let mut events = HashMap::new();
    let mut outputs = HashMap::new();
    let mut defines = HashMap::new();
    let mut machines = HashMap::new();
    let mut handlers = Vec::new();
    let mut invariants = Vec::new();
    let mut limits = Vec::new();
    let mut tests = Vec::new();

    let enum_names: Vec<String> = spec
        .decls
        .iter()
        .filter_map(|d| {
            if let Decl::Enum(e) = d {
                Some(e.name.clone())
            } else {
                None
            }
        })
        .collect();

    for decl in &spec.decls {
        match decl {
            Decl::Enum(e) => {
                if enums.contains_key(&e.name) {
                    diags.push(type_err(
                        file,
                        source,
                        &e.span,
                        "E101",
                        format!("duplicate enum `{}`", e.name),
                    ));
                }
                enums.insert(e.name.clone(), e.variants.clone());
            }
            Decl::Config(c) => collect_fields(
                &c.fields,
                &enum_names,
                &mut config_fields,
                file,
                source,
                &mut diags,
            ),
            Decl::State(s) => collect_fields(
                &s.fields,
                &enum_names,
                &mut state_fields,
                file,
                source,
                &mut diags,
            ),
            Decl::Entity(ent) => {
                let mut m = HashMap::new();
                for f in &ent.fields {
                    let ty = type_from_ast(&f.ty, &enum_names);
                    if ty == RomitType::Unknown {
                        diags.push(type_err(
                            file,
                            source,
                            &f.span,
                            "E102",
                            format!("unknown type for field `{}`", f.name),
                        ));
                    }
                    m.insert(f.name.clone(), ty);
                }
                entity_fields.insert(ent.name.clone(), m);
            }
            Decl::Event(ev) => {
                if events.insert(ev.name.clone(), ev.clone()).is_some() {
                    diags.push(type_err(
                        file,
                        source,
                        &ev.span,
                        "E103",
                        format!("duplicate event `{}`", ev.name),
                    ));
                }
            }
            Decl::Output(o) => {
                outputs.insert(o.name.clone(), o.clone());
            }
            Decl::Define(d) => {
                defines.insert(d.name.clone(), d.clone());
            }
            Decl::Machine(m) => {
                let key = path_key(&m.path);
                machines.insert(key, m.clone());
            }
            Decl::Handler(h) => handlers.push(h.clone()),
            Decl::Invariant(i) => invariants.push(i.clone()),
            Decl::Limit(l) => limits.push(l.clone()),
            Decl::Test(t) => tests.push(t.clone()),
            _ => {}
        }
    }

    // conflict check E210
    for h in &handlers {
        if !h.run_all || h.rules.len() < 2 {
            continue;
        }
        for i in 0..h.rules.len() {
            for j in (i + 1)..h.rules.len() {
                if rules_may_conflict(&h.rules[i], &h.rules[j]) {
                    diags.push(
                        Diagnostic::error(
                            "E210",
                            "two rules can both fire and disagree",
                            file,
                            h.span.line,
                            h.span.column,
                            snippet_at(source, h.span.line, h.span.column, "all"),
                        )
                        .with_help(format!(
                            "rules `{}` and `{}` may both set the same field; remove `all` or make conditions exclusive",
                            h.rules[i].name, h.rules[j].name
                        )),
                    );
                }
            }
        }
    }

    // machine check E220 on rule actions
    for h in &handlers {
        for rule in &h.rules {
            for act in &rule.actions {
                if let Action::Set { path, span, .. } = act {
                    if let Some(mach) = find_machine_for_path(path, &machines) {
                        if let Some(to) = path_last(path) {
                            // static check: we cannot know from-state at compile time for all cases;
                            // flag transitions to final from non-listed states only when obviously wrong
                            let _ = (mach, to);
                            if is_final_target(path, &machines) {
                                diags.push(
                                    Diagnostic::error(
                                        "E220",
                                        format!(
                                            "change to `{}` may not be allowed by the state machine",
                                            path_display(path)
                                        ),
                                        file,
                                        span.line,
                                        span.column,
                                        snippet_at(source, span.line, span.column, "set"),
                                    )
                                    .with_help(
                                        "add the edge to the machine, or guard the rule with the allowed from-state",
                                    ),
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    // W410 shadowed rules
    for h in &handlers {
        if h.run_all {
            continue;
        }
        for i in 0..h.rules.len() {
            for j in (i + 1)..h.rules.len() {
                if rule_covers(&h.rules[i], &h.rules[j]) {
                    diags.push(Diagnostic::warning(
                        "W410",
                        format!(
                            "rule `{}` can never run",
                            h.rules[j].name
                        ),
                        file,
                        h.rules[j].span.line,
                        h.rules[j].span.column,
                        snippet_at(
                            source,
                            h.rules[j].span.line,
                            h.rules[j].span.column,
                            &h.rules[j].name,
                        ),
                    )
                    .with_help(format!(
                        "rule `{}` always matches before it; narrow the earlier condition or reorder rules",
                        h.rules[i].name
                    )));
                }
            }
        }
    }

    let has_errors = diags
        .iter()
        .any(|d| d.severity == crate::diag::Severity::Error);
    if has_errors {
        return Err(diags);
    }

    Ok(CheckedProgram {
        spec,
        enums,
        entity_fields,
        state_fields,
        config_fields,
        events,
        outputs,
        defines,
        machines,
        handlers,
        invariants,
        limits,
        tests,
    })
}

fn collect_fields(
    fields: &[Field],
    enum_names: &[String],
    target: &mut HashMap<String, RomitType>,
    file: &str,
    source: &str,
    diags: &mut Vec<Diagnostic>,
) {
    for f in fields {
        let ty = type_from_ast(&f.ty, enum_names);
        if ty == RomitType::Unknown {
            diags.push(type_err(
                file,
                source,
                &f.span,
                "E102",
                format!("unknown type for field `{}`", f.name),
            ));
        }
        target.insert(f.name.clone(), ty);
    }
}

fn type_err(file: &str, source: &str, span: &Span, code: &str, msg: String) -> Diagnostic {
    Diagnostic::error(
        code,
        msg,
        file,
        span.line,
        span.column,
        snippet_at(source, span.line, span.column, "here"),
    )
}

fn path_key(p: &PathExpr) -> String {
    p.parts.join(".")
}

fn path_last(p: &PathExpr) -> Option<String> {
    p.parts.last().cloned()
}

fn path_display(p: &PathExpr) -> String {
    p.parts.join(".")
}

fn find_machine_for_path<'a>(
    path: &PathExpr,
    machines: &'a HashMap<String, MachineDecl>,
) -> Option<&'a MachineDecl> {
    let key = path_key(path);
    machines.get(&key).or_else(|| {
        if path.parts.len() == 2 {
            machines.get(&path_key(path))
        } else {
            None
        }
    })
}

fn is_final_target(_path: &PathExpr, _machines: &HashMap<String, MachineDecl>) -> bool {
    false
}

fn rules_may_conflict(a: &RuleDecl, b: &RuleDecl) -> bool {
    let a_sets = set_targets(&a.actions);
    let b_sets = set_targets(&b.actions);
    a_sets.iter().any(|t| b_sets.contains(t))
}

fn set_targets(actions: &[Action]) -> HashSet<String> {
    actions
        .iter()
        .filter_map(|a| {
            if let Action::Set { path, .. } = a {
                Some(path_key(path))
            } else {
                None
            }
        })
        .collect()
}

fn rule_covers(earlier: &RuleDecl, later: &RuleDecl) -> bool {
    matches!(earlier.when, RuleWhen::Otherwise) && matches!(later.when, RuleWhen::Expr(_))
}

fn is_obviously_unreachable(r: &RuleDecl) -> bool {
    matches!(
        r.when,
        RuleWhen::Expr(Expr::Literal(Literal::Bool(false), _))
    )
}

fn find_risky_rule_for_invariant(
    inv: &InvariantDecl,
    handlers: &[HandlerDecl],
) -> Option<(String, Span)> {
    let inv_str = format!("{:?}", inv.expr);
    if !inv_str.contains("approved") && !inv_str.contains("amount") {
        return None;
    }
    for h in handlers {
        for r in &h.rules {
            for a in &r.actions {
                if let Action::Set { path, span, .. } = a {
                    if path_display(path).contains("decision") {
                        if let RuleWhen::Expr(w) = &r.when {
                            let wstr = format!("{:?}", w);
                            if !wstr.contains("order_total") && inv_str.contains("order_total") {
                                return Some((r.name.clone(), span.clone()));
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

pub fn check_parse_only(
    file: &str,
    source: &str,
    spec: Spec,
) -> Result<CheckedProgram, Vec<Diagnostic>> {
    check(spec, file, source)
}
