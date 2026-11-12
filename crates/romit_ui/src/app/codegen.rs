//! Turn an [`App`] into JavaScript for the Romit app runtime.

use super::ast::*;
use super::expr::{walk_names, Expr, Op, TextPart};
use super::parse::Ctx;
use super::vocab::{did_you_mean, named_color};
use std::collections::{HashMap, HashSet};
use std::fmt::Write;

pub struct Gen<'a, 'b> {
    pub cx: &'b mut Ctx<'a>,
    states: HashSet<String>,
    computed: HashSet<String>,
    actions: HashMap<String, usize>,
    screens: HashMap<String, usize>,
    views: HashMap<String, usize>,
    scopes: Vec<String>,
    /// inside a `where` predicate: unknown bare names are fields of `it`
    where_depth: usize,
    line: usize,
    has_document: bool,
}

pub fn js_str(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '<' => out.push_str("\\u003c"),
            '\u{2028}' => out.push_str("\\u2028"),
            '\u{2029}' => out.push_str("\\u2029"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn num(n: f64) -> String {
    if n.fract() == 0.0 && n.abs() < 1e15 {
        format!("{}", n as i64)
    } else {
        format!("{n}")
    }
}

pub fn humanize(name: &str) -> String {
    let mut out = String::new();
    let chars: Vec<char> = name.chars().collect();
    for (i, c) in chars.iter().enumerate() {
        if *c == '_' {
            out.push(' ');
            continue;
        }
        if i > 0 && c.is_uppercase() && chars[i - 1].is_lowercase() {
            out.push(' ');
        }
        out.push(*c);
    }
    out
}

impl<'a, 'b> Gen<'a, 'b> {
    pub fn new(cx: &'b mut Ctx<'a>, app: &App) -> Self {
        let mut g = Gen {
            cx,
            states: app.states.iter().map(|s| s.name.clone()).collect(),
            computed: app.computed.iter().map(|c| c.name.clone()).collect(),
            actions: app
                .actions
                .iter()
                .map(|a| (a.name.clone(), a.params.len()))
                .collect(),
            screens: app
                .screens
                .iter()
                .map(|s| (s.name.clone(), s.params.len()))
                .collect(),
            views: app
                .views
                .iter()
                .map(|v| (v.name.clone(), v.params.len()))
                .collect(),
            scopes: Vec::new(),
            where_depth: 0,
            line: 1,
            has_document: !app.document.is_empty(),
        };
        let mut seen = HashSet::new();
        for (name, line) in app
            .states
            .iter()
            .map(|s| (&s.name, s.line))
            .chain(app.computed.iter().map(|c| (&c.name, c.line)))
        {
            if !seen.insert(name.clone()) {
                g.err("U111", format!("`{name}` is defined twice"), line, None);
            }
        }
        let mut seen = HashSet::new();
        for s in &app.screens {
            if !seen.insert(s.name.clone()) {
                g.err(
                    "U111",
                    format!("screen `{}` is defined twice", s.name),
                    s.line,
                    None,
                );
            }
        }
        g
    }

    fn err(&mut self, code: &str, msg: impl Into<String>, line: usize, help: Option<String>) {
        self.cx.error(code, msg, line, 1, help);
    }

    fn local(&self, n: &str) -> bool {
        self.scopes.iter().any(|s| s == n)
    }

    fn known_names(&self) -> Vec<String> {
        let mut v: Vec<String> = self.states.iter().cloned().collect();
        v.extend(self.computed.iter().cloned());
        v.extend(self.scopes.iter().cloned());
        v
    }

    fn is_value_name(&self, n: &str) -> bool {
        self.local(n) || self.states.contains(n) || self.computed.contains(n)
    }

    fn name(&mut self, n: &str) -> String {
        if self.local(n) {
            return format!("L_{n}");
        }
        if self.states.contains(n) {
            return format!("S.{n}");
        }
        if self.computed.contains(n) {
            return format!("C.{n}()");
        }
        match n {
            "now" => return "new Date()".into(),
            "today" => return "R.today()".into(),
            "index" if self.local("__index") => return "L___index".into(),
            _ => {}
        }
        if self.where_depth > 0 {
            return format!("R.field(it, {})", js_str(n));
        }
        let known = self.known_names();
        let known_refs: Vec<&str> = known.iter().map(|s| s.as_str()).collect();
        let mut hint = did_you_mean(n, &known_refs);
        if hint.is_empty() {
            hint = did_you_mean(n, super::vocab::MODIFIERS);
        }
        let help = if self.screens.contains_key(n) {
            Some(format!(
                "`{n}` is a screen; use `go to {n}` or `link \"…\" to {n}`"
            ))
        } else if named_color(n).is_some() {
            Some(format!("colors are values for modifiers, e.g. `color {n}`"))
        } else {
            Some(format!(
                "add it to `state` (e.g. `{n} is 0`), or check the spelling"
            ))
        };
        self.err(
            "U101",
            format!("unknown name `{n}`;{hint}")
                .trim_end_matches(';')
                .to_string(),
            self.line,
            help,
        );
        "undefined".into()
    }

    pub fn expr(&mut self, e: &Expr) -> String {
        match e {
            Expr::Num(n) => num(*n),
            Expr::Bool(b) => b.to_string(),
            Expr::Nothing => "null".into(),
            Expr::Color(c) => js_str(c),
            Expr::Text(parts) => self.text(parts),
            Expr::Name(n) => self.name(n),
            Expr::Member(b, f) => {
                let base = self.expr(b);
                match f.as_str() {
                    "count" | "length" | "size" => format!("R.of(\"count\", {base})"),
                    _ => format!("R.field({base}, {})", js_str(f)),
                }
            }
            Expr::Not(b) => format!("!R.truthy({})", self.expr(b)),
            Expr::Neg(b) => format!("(-R.num({}))", self.expr(b)),
            Expr::Bin(op, a, b) => {
                let (a, b) = (self.expr(a), self.expr(b));
                match op {
                    Op::Or => format!("(R.truthy({a}) || R.truthy({b}))"),
                    Op::And => format!("(R.truthy({a}) && R.truthy({b}))"),
                    Op::Eq => format!("R.eq({a}, {b})"),
                    Op::Ne => format!("!R.eq({a}, {b})"),
                    Op::Lt => format!("(R.cmp({a}, {b}) < 0)"),
                    Op::Le => format!("(R.cmp({a}, {b}) <= 0)"),
                    Op::Gt => format!("(R.cmp({a}, {b}) > 0)"),
                    Op::Ge => format!("(R.cmp({a}, {b}) >= 0)"),
                    Op::Add => format!("R.add({a}, {b})"),
                    Op::Sub => format!("R.sub({a}, {b})"),
                    Op::Mul => format!("(R.num({a}) * R.num({b}))"),
                    Op::Div => format!("R.div({a}, {b})"),
                    Op::Mod => format!("R.mod({a}, {b})"),
                    Op::In => format!("R.inn({a}, {b})"),
                }
            }
            Expr::Of(f, b) => format!("R.of({}, {})", js_str(f), self.expr(b)),
            Expr::Where(list, cond) => {
                let l = self.expr(list);
                self.where_depth += 1;
                self.scopes.push("it".into());
                let c = self.expr(cond);
                self.scopes.pop();
                self.where_depth -= 1;
                format!("R.where({l}, (L_it) => {{ const it = L_it; return {c}; }})")
            }
            Expr::SortBy(list, field, desc) => format!(
                "R.sort({}, {}, {})",
                self.expr(list),
                field
                    .as_deref()
                    .map(js_str)
                    .unwrap_or_else(|| "null".into()),
                desc
            ),
            Expr::As(v, f) => format!("R.fmt({}, {})", self.expr(v), js_str(f)),
            Expr::If(c, a, b) => format!(
                "(R.truthy({}) ? {} : {})",
                self.expr(c),
                self.expr(a),
                self.expr(b)
            ),
            Expr::List(items) => {
                let parts: Vec<String> = items.iter().map(|i| self.expr(i)).collect();
                format!("[{}]", parts.join(", "))
            }
            Expr::Record(fields) => {
                let parts: Vec<String> = fields
                    .iter()
                    .map(|(k, v)| format!("{}: {}", js_str(k), self.expr(v)))
                    .collect();
                format!("{{{}}}", parts.join(", "))
            }
            Expr::Random(a, b) => format!("R.rand({}, {})", self.expr(a), self.expr(b)),
            Expr::Item(i, l) => format!("R.item({}, {})", self.expr(i), self.expr(l)),
            Expr::IsEmpty(v, yes) => {
                let x = format!("R.isEmpty({})", self.expr(v));
                if *yes {
                    x
                } else {
                    format!("!{x}")
                }
            }
        }
    }

    fn text(&mut self, parts: &[TextPart]) -> String {
        if parts.len() == 1 {
            if let TextPart::Lit(s) = &parts[0] {
                return js_str(s);
            }
        }
        let mut out = Vec::new();
        for p in parts {
            match p {
                TextPart::Lit(s) => out.push(js_str(s)),
                TextPart::Hole(e) => out.push(format!("R.show({})", self.expr(e))),
            }
        }
        format!("({})", out.join(" + "))
    }

    /// A color value: named colors are literals unless shadowed by a value name.
    fn color(&mut self, e: &Expr) -> String {
        if let Expr::Name(n) = e {
            if !self.is_value_name(n) {
                if let Some(c) = named_color(n) {
                    return js_str(c);
                }
                return js_str(n);
            }
        }
        self.expr(e)
    }

    /// An enum-like word value (`style prominent`, `font title`).
    fn word(&mut self, e: &Expr) -> String {
        if let Expr::Name(n) = e {
            if !self.is_value_name(n) {
                return js_str(n);
            }
        }
        self.expr(e)
    }

    /// The JS left-hand side for something an action changes.
    fn target(&mut self, e: &Expr) -> String {
        match e {
            Expr::Name(n) => {
                if self.states.contains(n) {
                    format!("S.{n}")
                } else if self.computed.contains(n) {
                    self.err(
                        "U112",
                        format!(
                            "`{n}` is computed from other values and can't be changed directly"
                        ),
                        self.line,
                        Some("change the state values it is computed from".into()),
                    );
                    "R.sink".into()
                } else if self.local(n) {
                    self.err("U112", format!("`{n}` is a loop item or parameter; change one of its fields instead (e.g. `{n}.done = true`)"), self.line, None);
                    "R.sink".into()
                } else {
                    self.name(n);
                    "R.sink".into()
                }
            }
            Expr::Member(b, f) => format!("{}[{}]", self.expr(b), js_str(f)),
            _ => {
                self.err(
                    "U112",
                    "only state values and their fields can be changed",
                    self.line,
                    None,
                );
                "R.sink".into()
            }
        }
    }

    // ---------- actions ----------

    pub fn actions(&mut self, acts: &[Action], out: &mut String) {
        for a in acts {
            self.action(a, out);
        }
    }

    fn check_screen(&mut self, name: &str, nargs: usize) {
        match self.screens.get(name) {
            None => {
                let names: Vec<String> = self.screens.keys().cloned().collect();
                let refs: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
                let hint = did_you_mean(name, &refs);
                self.err(
                    "U113",
                    format!("no screen named `{name}`;{hint}")
                        .trim_end_matches(';')
                        .to_string(),
                    self.line,
                    Some(format!("screens: {}", refs.join(", "))),
                );
            }
            Some(&n) if n != nargs => {
                self.err(
                    "U113",
                    format!("screen `{name}` takes {n} value(s), got {nargs}"),
                    self.line,
                    Some(format!("go to {name} with …")),
                );
            }
            _ => {}
        }
    }

    fn args(&mut self, args: &[Expr]) -> String {
        let a: Vec<String> = args.iter().map(|e| self.expr(e)).collect();
        format!("[{}]", a.join(", "))
    }

    fn action(&mut self, a: &Action, out: &mut String) {
        match a {
            Action::At(line, inner) => {
                self.line = *line;
                self.action(inner, out);
            }
            Action::Set(t, v) => {
                let v = self.expr(v);
                let t = self.target(t);
                let _ = writeln!(out, "{t} = R.copy({v});");
            }
            Action::Change(t, op, v) => {
                let v = self.expr(v);
                let t = self.target(t);
                let f = if *op == '+' { "R.add" } else { "R.sub" };
                let _ = writeln!(out, "{t} = {f}({t}, {v});");
            }
            Action::Toggle(t) => {
                let t = self.target(t);
                let _ = writeln!(out, "{t} = !R.truthy({t});");
            }
            Action::Add(v, t) => {
                let v = self.expr(v);
                let t = self.target(t);
                let _ = writeln!(out, "{t} = R.list({t}).concat([R.copy({v})]);");
            }
            Action::Remove(v, t) => {
                let v = self.expr(v);
                let t = self.target(t);
                let _ = writeln!(
                    out,
                    "{{ const __x = {v}; {t} = R.list({t}).filter((y) => !R.same(y, __x)); }}"
                );
            }
            Action::RemoveWhere(t, cond) => {
                let t = self.target(t);
                self.where_depth += 1;
                self.scopes.push("it".into());
                let c = self.expr(cond);
                self.scopes.pop();
                self.where_depth -= 1;
                let _ = writeln!(out, "{t} = R.list({t}).filter((L_it) => {{ const it = L_it; return !R.truthy({c}); }});");
            }
            Action::Clear(t) => {
                let t = self.target(t);
                let _ = writeln!(out, "{t} = R.blank({t});");
            }
            Action::Sort(t, f, desc) => {
                let t = self.target(t);
                let f = f.as_deref().map(js_str).unwrap_or_else(|| "null".into());
                let _ = writeln!(out, "{t} = R.sort({t}, {f}, {desc});");
            }
            Action::Shuffle(t) => {
                let t = self.target(t);
                let _ = writeln!(out, "{t} = R.shuffle({t});");
            }
            Action::GoTo(s, args) => {
                self.check_screen(s, args.len());
                let a = self.args(args);
                let _ = writeln!(out, "R.go({}, {a});", js_str(s));
            }
            Action::Show(s, args, style, locked) => {
                self.check_screen(s, args.len());
                let a = self.args(args);
                let _ = writeln!(
                    out,
                    "R.sheet({}, {a}, {}, {locked});",
                    js_str(s),
                    js_str(style)
                );
            }
            Action::OpenWindow(s, args) => {
                self.check_screen(s, args.len());
                let a = self.args(args);
                let _ = writeln!(out, "R.openWindow({}, {a});", js_str(s));
            }
            Action::OpenFile(t, kind) => {
                let t = self.target(t);
                let _ = writeln!(out, "{t} = await R.openFile({}, {t});", js_str(kind));
            }
            Action::SaveFile(v, name) => {
                let v = self.expr(v);
                let n = self.expr(name);
                let _ = writeln!(out, "R.saveFile({v}, {n});");
            }
            Action::Document(cmd) => {
                if !self.has_document {
                    self.err("U118", "this app has no `document` line", self.line,
                        Some("add at the app level, e.g. `document notes, title named \"Notes.json\"`".into()));
                }
                let _ = writeln!(out, "await R.doc.{cmd}();");
            }
            Action::GoBack => out.push_str("R.back();\n"),
            Action::GoHome => out.push_str("R.home();\n"),
            Action::Close => out.push_str("R.close();\n"),
            Action::Alert(t, m) => {
                let t = self.expr(t);
                let m = m
                    .as_ref()
                    .map(|m| self.expr(m))
                    .unwrap_or_else(|| "null".into());
                let _ = writeln!(out, "R.alert({t}, {m});");
            }
            Action::Ask(t, m, body) => {
                let t = self.expr(t);
                let m = m
                    .as_ref()
                    .map(|m| self.expr(m))
                    .unwrap_or_else(|| "null".into());
                let mut b = String::new();
                self.actions(body, &mut b);
                let _ = writeln!(out, "R.ask({t}, {m}, async () => {{\n{b}}});");
            }
            Action::Toast(v) => {
                let v = self.expr(v);
                let _ = writeln!(out, "R.toast({v});");
            }
            Action::Wait(ms) => {
                let _ = writeln!(out, "await R.wait({});", num(*ms));
            }
            Action::Open(v) => {
                let v = self.expr(v);
                let _ = writeln!(out, "R.open({v});");
            }
            Action::Copy(v) => {
                let v = self.expr(v);
                let _ = writeln!(out, "R.copyText({v});");
            }
            Action::Share(v) => {
                let v = self.expr(v);
                let _ = writeln!(out, "R.share({v});");
            }
            Action::Say(v) => {
                let v = self.expr(v);
                let _ = writeln!(out, "R.say({v});");
            }
            Action::Load(url, t) => {
                let u = self.expr(url);
                let t = self.target(t);
                let _ = writeln!(out, "{t} = await R.load({u});");
            }
            Action::Vibrate => out.push_str("R.vibrate();\n"),
            Action::Run(n, args) => match self.actions.get(n).copied() {
                Some(np) => {
                    if np != args.len() {
                        self.err(
                            "U114",
                            format!("action `{n}` takes {np} value(s), got {}", args.len()),
                            self.line,
                            None,
                        );
                    }
                    let a: Vec<String> = args.iter().map(|e| self.expr(e)).collect();
                    let _ = writeln!(out, "await A.{n}({});", a.join(", "));
                }
                None => {
                    let names: Vec<String> = self.actions.keys().cloned().collect();
                    let refs: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
                    let hint = did_you_mean(n, &refs);
                    self.err(
                        "U114",
                        format!("unknown action `{n}`;{hint}")
                            .trim_end_matches(';')
                            .to_string(),
                        self.line,
                        Some("define it at the app level:\n  action reset\n    count = 0".into()),
                    );
                }
            },
            Action::If(branches) => {
                for (i, (cond, body)) in branches.iter().enumerate() {
                    let mut b = String::new();
                    self.actions(body, &mut b);
                    match (i, cond) {
                        (0, Some(c)) => {
                            let c = self.expr(c);
                            let _ = write!(out, "if (R.truthy({c})) {{\n{b}}}");
                        }
                        (_, Some(c)) => {
                            let c = self.expr(c);
                            let _ = write!(out, " else if (R.truthy({c})) {{\n{b}}}");
                        }
                        (_, None) => {
                            let _ = write!(out, " else {{\n{b}}}");
                        }
                    }
                }
                out.push('\n');
            }
            Action::For(var, iter, body) => {
                let it = self.expr(iter);
                self.scopes.push(var.clone());
                self.scopes.push("__index".into());
                let mut b = String::new();
                self.actions(body, &mut b);
                self.scopes.pop();
                self.scopes.pop();
                let _ = writeln!(out, "{{ let L___index = 0; for (const L_{var} of R.list({it})) {{ L___index++;\n{b}}} }}");
            }
            Action::Animate(body) => {
                out.push_str("R.animate();\n");
                self.actions(body, out);
            }
        }
    }

    fn handler(&mut self, acts: &[Action]) -> String {
        let mut b = String::new();
        self.actions(acts, &mut b);
        format!("R.act(async () => {{\n{b}}})")
    }

    // ---------- views ----------

    pub fn nodes(&mut self, nodes: &[Node]) -> String {
        let parts: Vec<String> = nodes.iter().map(|n| self.node(n)).collect();
        format!("[{}]", parts.join(",\n"))
    }

    fn node(&mut self, n: &Node) -> String {
        match n {
            Node::El(el) => self.element(el),
            Node::If { branches, line } => {
                self.line = *line;
                let mut out = String::new();
                for (cond, body) in branches {
                    let b = self.nodes(body);
                    match cond {
                        Some(c) => {
                            let c = self.expr(c);
                            let _ = write!(out, "R.truthy({c}) ? {b} : ");
                        }
                        None => {
                            let _ = write!(out, "{b}");
                        }
                    }
                }
                if branches.last().is_some_and(|b| b.0.is_some()) {
                    out.push_str("[]");
                }
                format!("({out})")
            }
            Node::For {
                var,
                iter,
                body,
                line,
            } => {
                self.line = *line;
                let it = self.expr(iter);
                self.scopes.push(var.clone());
                self.scopes.push("__index".into());
                let b = self.nodes(body);
                self.scopes.pop();
                self.scopes.pop();
                format!("R.each({it}, (L_{var}, L___index) => {b})")
            }
        }
    }

    fn events(&mut self, evs: &[EventBlock]) -> String {
        let mut parts = Vec::new();
        for ev in evs {
            self.line = ev.line;
            if ev.kind == "every" || (ev.kind == "change" && ev.arg.is_some()) {
                self.err(
                    "U104",
                    format!(
                        "`{}` belongs on a screen or the app, not on an element",
                        if ev.kind == "every" {
                            "every"
                        } else {
                            "on change of"
                        }
                    ),
                    ev.line,
                    None,
                );
                continue;
            }
            let h = self.handler(&ev.actions);
            parts.push(format!("{}: {h}", ev.kind));
        }
        format!("{{{}}}", parts.join(", "))
    }

    fn element(&mut self, el: &Element) -> String {
        self.line = el.line;
        // custom view
        if el.kind.chars().next().is_some_and(|c| c.is_uppercase()) {
            let Some(&np) = self.views.get(&el.kind) else {
                let names: Vec<String> = self.views.keys().cloned().collect();
                let refs: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
                let hint = did_you_mean(&el.kind, &refs);
                let msg = if self.screens.contains_key(&el.kind) {
                    format!(
                        "`{}` is a screen, not a view; use `link \"…\" to {}`",
                        el.kind, el.kind
                    )
                } else {
                    format!("unknown view `{}`;{hint}", el.kind)
                        .trim_end_matches(';')
                        .to_string()
                };
                self.err(
                    "U115",
                    msg,
                    el.line,
                    Some(format!(
                        "define it at the app level:\n  view {} with item\n    text item",
                        el.kind
                    )),
                );
                return "null".into();
            };
            let mut args = el.args.clone();
            if let Some(w) = el.kw("with") {
                args.extend(w.iter().cloned());
            }
            if args.len() != np {
                self.err(
                    "U115",
                    format!("view `{}` takes {np} value(s), got {}", el.kind, args.len()),
                    el.line,
                    None,
                );
            }
            let a: Vec<String> = args.iter().map(|e| self.expr(e)).collect();
            let call = format!("V.{}({})", el.kind, a.join(", "));
            let mods = self.mods(el);
            let evs = self.events(&el.events);
            return format!("R.comp({call}, {mods}, {evs})");
        }
        let mut props: Vec<String> = Vec::new();
        let word_args = matches!(el.kind.as_str(), "grid" | "scroll");
        let args: Vec<String> = el
            .args
            .iter()
            .map(|e| {
                if word_args {
                    self.word(e)
                } else {
                    self.expr(e)
                }
            })
            .collect();
        props.push(format!("a: [{}]", args.join(", ")));
        for (k, vals) in &el.kw {
            let v = match k.as_str() {
                "bind" | "scope" => {
                    let Some(t) = vals.first() else { continue };
                    let get = self.expr(t);
                    let set = self.target(t);
                    format!("{{get: () => {get}, set: (v) => {{ {set} = v; }}}}")
                }
                "to" => {
                    if let Some(Expr::Name(n)) = vals.first() {
                        if self.screens.contains_key(n) && !self.is_value_name(n) {
                            let with = el.kw("with").cloned().unwrap_or_default();
                            self.check_screen(n, with.len());
                            let a = self.args(&with);
                            props.push(format!("nav: {{screen: {}, args: {a}}}", js_str(n)));
                            continue;
                        }
                    }
                    self.expr(&vals[0])
                }
                "with" if el.kw("to").is_some() => continue,
                "options" | "scopes" | "suggest" => {
                    let parts: Vec<String> = vals.iter().map(|e| self.expr(e)).collect();
                    format!("R.opts([{}])", parts.join(", "))
                }
                "icon" => {
                    let v = &vals[0];
                    self.word(v)
                }
                _ => {
                    let parts: Vec<String> = vals.iter().map(|e| self.expr(e)).collect();
                    if parts.len() == 1 {
                        parts[0].clone()
                    } else {
                        format!("[{}]", parts.join(", "))
                    }
                }
            };
            props.push(format!("{}: {v}", js_str(k)));
        }
        self.validate(el);
        let evs = self.events(&el.events);
        props.push(format!("on: {evs}"));
        let kids = self.nodes(&el.children);
        let mods = self.mods(el);
        format!(
            "R.mod(R.el({}, {{{}}}, {kids}), {mods})",
            js_str(&el.kind),
            props.join(", ")
        )
    }

    fn validate(&mut self, el: &Element) {
        let needs_bind = matches!(
            el.kind.as_str(),
            "toggle"
                | "field"
                | "secure"
                | "editor"
                | "slider"
                | "stepper"
                | "picker"
                | "date"
                | "colorpicker"
                | "search"
        );
        if needs_bind && el.kw("bind").is_none() {
            self.err(
                "U116",
                format!(
                    "`{}` needs `bind <state value>` so it can change something",
                    el.kind
                ),
                el.line,
                Some(format!(
                    "e.g. `{} \"Label\" bind name` with `name is \"\"` in state",
                    el.kind
                )),
            );
        }
        if el.kind == "picker" && el.kw("options").is_none() {
            self.err(
                "U116",
                "`picker` needs `options`",
                el.line,
                Some("e.g. `picker \"Size\" bind size options \"S\", \"M\", \"L\"`".into()),
            );
        }
        if el.kind == "link" && el.kw("to").is_none() {
            self.err(
                "U116",
                "`link` needs `to` a screen or web address",
                el.line,
                Some(
                    "`link \"Docs\" to \"https://…\"` or `link \"Details\" to Detail with item`"
                        .into(),
                ),
            );
        }
        if let Some(b) = el.kw1("bind") {
            if !matches!(b, Expr::Name(_) | Expr::Member(_, _)) {
                self.err(
                    "U116",
                    "`bind` needs a state value name like `name` or `todo.done`",
                    el.line,
                    None,
                );
            }
        }
    }

    fn mods(&mut self, el: &Element) -> String {
        let mut parts = Vec::new();
        for m in &el.mods {
            self.line = m.line;
            let vals: Vec<String> = m
                .values
                .iter()
                .map(|v| match m.name.as_str() {
                    "color" | "background" | "tint" | "fill" | "stroke" | "gradient" => {
                        self.color(v)
                    }
                    "border" => self.color(v),
                    "font" | "weight" | "style" | "role" | "align" | "transition" | "aspect"
                    | "keyboard" | "clip" | "padding" => self.word(v),
                    _ => self.expr(v),
                })
                .collect();
            let mut entry = format!("[{}", js_str(&m.name));
            for v in vals {
                entry.push_str(", ");
                entry.push_str(&v);
            }
            entry.push(']');
            if let Some(c) = &m.cond {
                let c = self.expr(c);
                entry = format!("(R.truthy({c}) ? {entry} : null)");
            }
            parts.push(entry);
        }
        format!("[{}]", parts.join(", "))
    }

    // ---------- whole app ----------

    pub fn app(&mut self, app: &App) -> String {
        let mut out = String::new();
        let _ = writeln!(out, "R.name = {};", js_str(&app.name));
        // theme
        let mut theme = Vec::new();
        for t in &app.theme {
            self.line = t.line;
            let v = match t.key.as_str() {
                "appearance" | "dark" => {
                    let v = if t.key == "dark" {
                        format!("(R.truthy({}) ? \"dark\" : \"light\")", self.expr(&t.value))
                    } else {
                        self.word(&t.value)
                    };
                    let _ = writeln!(out, "R.appearance = () => {v};");
                    continue;
                }
                "corner" | "spacing" | "width" => self.expr(&t.value),
                "font" | "titlefont" => self.word(&t.value),
                _ => self.color(&t.value),
            };
            theme.push(format!("{}: {v}", js_str(&t.key)));
        }
        let _ = writeln!(out, "R.theme = {{{}}};", theme.join(", "));
        // state
        out.push_str("R.init = () => {\n");
        let mut seen: Vec<String> = Vec::new();
        for s in &app.states {
            self.line = s.line;
            // earlier state values may be used in later ones
            let saved_states = std::mem::take(&mut self.states);
            self.states = seen.iter().cloned().collect();
            let mut bad = Vec::new();
            walk_names(&s.init, &mut |n, _| {
                if saved_states.contains(n) && !seen.iter().any(|x| x == n) {
                    bad.push(n.to_string());
                }
            });
            for b in bad {
                self.err(
                    "U117",
                    format!("`{}` uses `{b}` before it is defined", s.name),
                    s.line,
                    Some("move it lower in `state`, or make it `computed`".into()),
                );
            }
            let v = self.expr(&s.init);
            self.states = saved_states;
            let _ = writeln!(out, "  S.{} = {v};", s.name);
            seen.push(s.name.clone());
        }
        out.push_str("};\n");
        let saved: Vec<String> = app
            .states
            .iter()
            .filter(|s| s.saved)
            .map(|s| js_str(&s.name))
            .collect();
        let _ = writeln!(out, "R.saved = [{}];", saved.join(", "));
        // computed
        for c in &app.computed {
            self.line = c.line;
            let v = self.expr(&c.value);
            let _ = writeln!(out, "C.{} = () => {v};", c.name);
        }
        // actions
        for a in &app.actions {
            self.line = a.line;
            for p in &a.params {
                self.scopes.push(p.clone());
            }
            let mut b = String::new();
            self.actions(&a.body, &mut b);
            for _ in &a.params {
                self.scopes.pop();
            }
            let params: Vec<String> = a.params.iter().map(|p| format!("L_{p}")).collect();
            let _ = writeln!(
                out,
                "A.{} = async ({}) => {{\n{b}}};",
                a.name,
                params.join(", ")
            );
        }
        // views
        for v in &app.views {
            self.line = v.line;
            for p in &v.params {
                self.scopes.push(p.clone());
            }
            let body = self.nodes(&v.body);
            for _ in &v.params {
                self.scopes.pop();
            }
            let params: Vec<String> = v.params.iter().map(|p| format!("L_{p}")).collect();
            let _ = writeln!(out, "V.{} = ({}) => {body};", v.name, params.join(", "));
        }
        // document (DocumentGroup)
        if !app.document.is_empty() {
            self.line = app.document_line;
            for n in &app.document {
                if !self.states.contains(n) {
                    self.err(
                        "U118",
                        format!("`document` names `{n}`, which is not in `state`"),
                        app.document_line,
                        None,
                    );
                }
            }
            let names: Vec<String> = app.document.iter().map(|n| js_str(n)).collect();
            let file = match &app.document_name {
                Some(e) => self.expr(e),
                None => js_str(&format!("{}.json", app.name)),
            };
            let _ = writeln!(
                out,
                "R.document = {{names: [{}], file: () => {file}}};",
                names.join(", ")
            );
        }
        // screens
        for s in &app.screens {
            self.line = s.line;
            for p in &s.params {
                self.scopes.push(p.clone());
            }
            let params: Vec<String> = s.params.iter().map(|p| format!("L_{p}")).collect();
            let params = params.join(", ");
            let title = match &s.title {
                Some(t) => self.expr(t),
                None => js_str(&humanize(&s.name)),
            };
            let subtitle = match &s.subtitle {
                Some(t) => self.expr(t),
                None => "null".into(),
            };
            let body = self.nodes(&s.body);
            let sidebar = if s.sidebar.is_empty() {
                "null".to_string()
            } else {
                format!("({params}) => {}", self.nodes(&s.sidebar))
            };
            let mut bars = Vec::new();
            for (place, nodes) in &s.toolbar {
                let n = self.nodes(nodes);
                bars.push(format!("{{place: {}, items: {n}}}", js_str(place)));
            }
            let toolbar = format!("[{}]", bars.join(", "));
            let mut evs = Vec::new();
            let mut timers = Vec::new();
            let mut watches = Vec::new();
            for ev in &s.events {
                self.line = ev.line;
                let h = self.handler(&ev.actions);
                match ev.kind.as_str() {
                    "every" => timers.push(format!(
                        "{{ms: {}, run: ({params}) => {h}()}}",
                        num(ev.millis.unwrap_or(1000.0))
                    )),
                    "change" if ev.arg.is_some() => {
                        let w = self.expr(ev.arg.as_ref().unwrap());
                        watches.push(format!(
                            "{{get: ({params}) => {w}, run: ({params}) => {h}()}}"
                        ));
                    }
                    "appear" | "disappear" | "background" | "foreground" => {
                        evs.push(format!("{}: ({params}) => {h}()", ev.kind))
                    }
                    other => self.err(
                        "U104",
                        format!("`on {other}` belongs on an element, not a screen"),
                        ev.line,
                        None,
                    ),
                }
            }
            for _ in &s.params {
                self.scopes.pop();
            }
            let tab = match &s.tab_icon {
                Some(i) => self.word(i),
                None => "null".into(),
            };
            let _ = writeln!(
                out,
                "R.screens[{name}] = {{name: {name}, params: {np}, tab: {tab}, inline: {inline}, settings: {settings}, title: ({params}) => {title}, subtitle: ({params}) => {subtitle}, body: ({params}) => {body}, sidebar: {sidebar}, toolbar: ({params}) => {toolbar}, events: {{{evs}}}, timers: [{timers}], watches: [{watches}]}};",
                name = js_str(&s.name),
                np = s.params.len(),
                inline = s.inline,
                settings = s.settings,
                evs = evs.join(", "),
                timers = timers.join(", "),
                watches = watches.join(", "),
            );
        }
        let _ = writeln!(out, "R.home_screen = {};", js_str(&app.screens[0].name));
        // app events
        for ev in &app.events {
            self.line = ev.line;
            if ev.kind == "url" {
                self.scopes.push("url".into());
                let h = self.handler(&ev.actions);
                self.scopes.pop();
                let _ = writeln!(out, "R.onUrl = (L_url) => {h}();");
                continue;
            }
            let h = self.handler(&ev.actions);
            match ev.kind.as_str() {
                "every" => {
                    let _ = writeln!(
                        out,
                        "R.timers.push({{ms: {}, run: () => {h}()}});",
                        num(ev.millis.unwrap_or(1000.0))
                    );
                }
                "appear" => {
                    let _ = writeln!(out, "R.starts.push(() => {h}());");
                }
                "background" | "foreground" => {
                    let _ = writeln!(
                        out,
                        "R.phases.push({{kind: {}, run: () => {h}()}});",
                        js_str(&ev.kind)
                    );
                }
                "change" if ev.arg.is_some() => {
                    let w = self.expr(ev.arg.as_ref().unwrap());
                    let _ = writeln!(out, "R.watches.push({{get: () => {w}, run: () => {h}()}});");
                }
                other => self.err(
                    "U104",
                    format!("`on {other}` belongs on an element, not the app"),
                    ev.line,
                    None,
                ),
            }
        }
        out
    }
}
