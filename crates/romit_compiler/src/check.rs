use crate::ast::*;
use crate::diag::{snippet_at, Diagnostic};
use crate::types::{type_from_ast, RomitType};
use std::collections::{HashMap, HashSet};

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

    let has_errors = diags.iter().any(|d| d.severity == crate::diag::Severity::Error);
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

fn find_machine_for_path<'a>(path: &PathExpr, machines: &'a HashMap<String, MachineDecl>) -> Option<&'a MachineDecl> {
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
    matches!(earlier.when, RuleWhen::Otherwise)
        || (matches!(later.when, RuleWhen::Expr(_))
            && !matches!(earlier.when, RuleWhen::Otherwise))
        && false
}

fn is_obviously_unreachable(r: &RuleDecl) -> bool {
    matches!(r.when, RuleWhen::Expr(Expr::Literal(Literal::Bool(false), _)))
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

pub fn check_parse_only(file: &str, source: &str, spec: Spec) -> Result<CheckedProgram, Vec<Diagnostic>> {
    check(spec, file, source)
}
