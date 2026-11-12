//! Parse a Romit app (`use ui` + `app Name`) into an [`App`].

use super::ast::*;
use super::expr::{is_operator_word, Expr, ExprError, ExprParser};
use super::lex::{lex_tree, Line, Tok};
use super::vocab::*;
use crate::diag::{snippet_at, Diagnostic};

pub struct Ctx<'a> {
    pub file: &'a str,
    pub source: &'a str,
    pub diags: Vec<Diagnostic>,
}

impl<'a> Ctx<'a> {
    pub fn error(
        &mut self,
        code: &str,
        msg: impl Into<String>,
        line: usize,
        col: usize,
        help: Option<String>,
    ) {
        let mut d = Diagnostic::error(
            code,
            msg,
            self.file,
            line,
            col,
            snippet_at(self.source, line, col),
        );
        if let Some(h) = help {
            d = d.with_help(h);
        }
        self.diags.push(d);
    }
    fn at(
        &mut self,
        l: &Line,
        tok: usize,
        code: &str,
        msg: impl Into<String>,
        help: Option<String>,
    ) {
        let col = l
            .cols
            .get(tok)
            .copied()
            .unwrap_or_else(|| l.cols.last().map(|c| c + 1).unwrap_or(1));
        self.error(code, msg, l.line_no, col, help);
    }
    fn expr_err(&mut self, l: &Line, e: ExprError) {
        self.at(l, e.tok, "U102", e.message, e.help);
    }
}

pub fn parse_app(source: &str, file: &str) -> Result<App, Vec<Diagnostic>> {
    let mut cx = Ctx {
        file,
        source,
        diags: Vec::new(),
    };
    let tree = match lex_tree(source) {
        Ok(t) => t,
        Err(e) => {
            cx.error(e.code, e.message, e.line, e.column, e.help);
            return Err(cx.diags);
        }
    };
    let mut app = App {
        name: "App".into(),
        ..Default::default()
    };
    for l in &tree {
        match l.toks[0].word() {
            Some("use") => {}
            Some("app") => {
                match l.toks.get(1) {
                    Some(Tok::Word(n)) | Some(Tok::Str(n)) => app.name = n.clone(),
                    _ => cx.at(l, 1, "U003", "expected an app name, e.g. `app Todos`", None),
                }
                for c in &l.children {
                    app_item(&mut cx, &mut app, c);
                }
            }
            _ => app_item(&mut cx, &mut app, l),
        }
    }
    if app.screens.is_empty() && cx.diags.is_empty() {
        cx.error(
            "U003",
            "an app needs at least one `screen`",
            1,
            1,
            Some("add:\n  screen Home\n    text \"Hello\"".into()),
        );
    }
    if cx.diags.is_empty() {
        Ok(app)
    } else {
        Err(cx.diags)
    }
}

fn app_item(cx: &mut Ctx, app: &mut App, l: &Line) {
    let head = l.toks[0].word().unwrap_or("");
    match head {
        "theme" => {
            if l.toks.len() > 1 {
                // `theme dark` shorthand
                let e = one_expr(cx, l, 1, &never);
                if let Some(e) = e {
                    app.theme.push(ThemeEntry {
                        key: "appearance".into(),
                        value: e,
                        line: l.line_no,
                    });
                }
            }
            for c in &l.children {
                let Some(key) = c.toks[0].word() else {
                    cx.at(c, 0, "U006", "theme lines look like `accent purple`", None);
                    continue;
                };
                if !THEME_KEYS.contains(&key) {
                    cx.at(
                        c,
                        0,
                        "U006",
                        format!("unknown theme setting `{key}`"),
                        Some(format!(
                            "valid settings: {}{}",
                            THEME_KEYS.join(", "),
                            did_you_mean(key, THEME_KEYS)
                        )),
                    );
                    continue;
                }
                let mut start = 1;
                if key == "dark" && c.toks.get(1).and_then(|t| t.word()) == Some("when") {
                    start = 2;
                }
                if c.toks.len() <= start {
                    if key == "dark" {
                        app.theme.push(ThemeEntry {
                            key: "appearance".into(),
                            value: Expr::Text(vec![super::expr::TextPart::Lit("dark".into())]),
                            line: c.line_no,
                        });
                        continue;
                    }
                    cx.at(c, start, "U006", format!("`{key}` needs a value"), None);
                    continue;
                }
                if let Some(e) = one_expr(cx, c, start, &never) {
                    app.theme.push(ThemeEntry {
                        key: key.to_string(),
                        value: e,
                        line: c.line_no,
                    });
                }
            }
        }
        "state" | "data" | "remember" => {
            if l.toks.len() > 1 {
                // `state count is 0` single-line form
                state_line(cx, app, l, 1, head == "remember");
            }
            for c in &l.children {
                state_line(cx, app, c, 0, head == "remember");
            }
        }
        "computed" | "derived" => {
            for c in &l.children {
                computed_line(cx, app, c, 0);
            }
        }
        "let" => computed_line(cx, app, l, 1),
        "action" | "func" | "function" | "fn" => {
            let Some(Tok::Word(name)) = l.toks.get(1) else {
                cx.at(
                    l,
                    1,
                    "U003",
                    "expected an action name, e.g. `action reset`",
                    None,
                );
                return;
            };
            let params = parse_params(cx, l, 2);
            let body = parse_actions(cx, &l.children);
            app.actions.push(ActionDef {
                name: name.clone(),
                params,
                body,
                line: l.line_no,
            });
        }
        "screen" | "page" => {
            if let Some(s) = parse_screen(cx, l) {
                app.screens.push(s);
            }
        }
        "view" | "component" => {
            let Some(Tok::Word(name)) = l.toks.get(1) else {
                cx.at(
                    l,
                    1,
                    "U003",
                    "expected a view name, e.g. `view TodoRow with todo`",
                    None,
                );
                return;
            };
            if !name.chars().next().is_some_and(|c| c.is_uppercase()) {
                cx.at(
                    l,
                    1,
                    "U003",
                    format!(
                        "view names start with a capital letter: `{}`",
                        capitalize(name)
                    ),
                    None,
                );
            }
            let params = parse_params(cx, l, 2);
            let body = parse_nodes(cx, &l.children);
            app.views.push(Component {
                name: name.clone(),
                params,
                body,
                line: l.line_no,
            });
        }
        "on" | "every" => {
            if let Some(ev) = parse_event(cx, l) {
                app.events.push(ev);
            }
        }
        "document" => {
            // document notes, title [named "Notes.json"]
            app.document_line = l.line_no;
            let mut i = 1;
            while i < l.toks.len() {
                match &l.toks[i] {
                    Tok::Word(w) if w == "named" || w == "called" => {
                        app.document_name = one_expr(cx, l, i + 1, &never);
                        break;
                    }
                    Tok::Word(w) if w == "and" => {}
                    Tok::Word(w) => app.document.push(w.clone()),
                    Tok::Sym(",") => {}
                    t => cx.at(
                        l,
                        i,
                        "U003",
                        format!("expected a state name, found {}", t.describe()),
                        None,
                    ),
                }
                i += 1;
            }
            if app.document.is_empty() {
                cx.at(
                    l,
                    1,
                    "U003",
                    "`document` needs the state values it saves",
                    Some("e.g. `document notes, title named \"Notes.json\"`".into()),
                );
            }
        }
        _ => cx.at(
            l,
            0,
            "U003",
            format!("`{head}` cannot go at the app level"),
            Some(
                "app-level blocks: theme, state, computed, action, screen, view, on start, every"
                    .into(),
            ),
        ),
    }
}

fn never(_: &str) -> bool {
    false
}

fn one_expr(cx: &mut Ctx, l: &Line, start: usize, stop: &dyn Fn(&str) -> bool) -> Option<Expr> {
    let mut p = ExprParser::new(&l.toks, start, stop, l.line_no);
    p.comma_stops = false;
    match p.parse() {
        Ok(e) => {
            if p.pos < l.toks.len() && !stop(l.toks[p.pos].word().unwrap_or("\u{0}")) {
                cx.at(
                    l,
                    p.pos,
                    "U102",
                    format!("unexpected {}", l.toks[p.pos].describe()),
                    None,
                );
                return None;
            }
            Some(e)
        }
        Err(e) => {
            cx.expr_err(l, e);
            None
        }
    }
}

fn state_line(cx: &mut Ctx, app: &mut App, l: &Line, start: usize, force_saved: bool) {
    let Some(Tok::Word(name)) = l.toks.get(start) else {
        cx.at(l, start, "U103", "state lines look like `count is 0`", None);
        return;
    };
    if let Some(why) = bad_value_name(name) {
        cx.at(
            l,
            start,
            "U110",
            format!("`{name}` can't be a state name: {why}"),
            Some(format!(
                "use a more specific name, like `selected{}` or `my{}`",
                capitalize(name),
                capitalize(name)
            )),
        );
        return;
    }
    let sep = l.toks.get(start + 1);
    let has_sep = matches!(sep, Some(Tok::Word(w)) if w == "is" || w == "starts" || w == "=")
        || matches!(sep, Some(Tok::Sym("=")));
    if !has_sep {
        cx.at(
            l,
            start + 1,
            "U103",
            format!("expected `is` after `{name}`"),
            Some(format!("write `{name} is 0` or `{name} is \"\"`")),
        );
        return;
    }
    let mut vstart = start + 2;
    if l.toks.get(start + 1).and_then(|t| t.word()) == Some("starts")
        && l.toks.get(vstart).and_then(|t| t.word()) == Some("at")
    {
        vstart += 1;
    }
    let mut end = l.toks.len();
    let mut saved = force_saved;
    if l.toks
        .last()
        .and_then(|t| t.word())
        .is_some_and(|w| w == "saved" || w == "remembered" || w == "stored")
    {
        saved = true;
        end -= 1;
    }
    let toks = &l.toks[..end];
    let mut p = ExprParser::new(toks, vstart, &never, l.line_no);
    p.comma_stops = false;
    let mut init = match p.parse() {
        Ok(e) => e,
        Err(e) => {
            cx.expr_err(l, e);
            return;
        }
    };
    if p.pos < toks.len() {
        cx.at(
            l,
            p.pos,
            "U103",
            format!("unexpected {} in state value", toks[p.pos].describe()),
            None,
        );
        return;
    }
    // Child lines add list items: records (`title "Milk", done false`) or plain values.
    if !l.children.is_empty() {
        let mut items = match init {
            Expr::List(v) => v,
            _ => {
                cx.at(
                    l,
                    vstart,
                    "U103",
                    "only a `list` can have indented items",
                    None,
                );
                return;
            }
        };
        for c in &l.children {
            let is_record = matches!(c.toks.first(), Some(Tok::Word(w)) if w != "with" && !is_operator_word(w))
                && c.toks.len() > 1
                && !matches!(c.toks.get(1), Some(Tok::Sym(_)));
            let mut toks = c.toks.clone();
            if is_record {
                toks.insert(0, Tok::Word("with".into()));
            }
            let mut p = ExprParser::new(&toks, 0, &never, c.line_no);
            p.comma_stops = false;
            match p.parse() {
                Ok(e) => items.push(e),
                Err(e) => cx.expr_err(c, e),
            }
        }
        init = Expr::List(items);
    }
    app.states.push(StateVar {
        name: name.clone(),
        init,
        saved,
        line: l.line_no,
    });
}

fn computed_line(cx: &mut Ctx, app: &mut App, l: &Line, start: usize) {
    let Some(Tok::Word(name)) = l.toks.get(start) else {
        cx.at(
            l,
            start,
            "U103",
            "computed lines look like `left is count of todos where not done`",
            None,
        );
        return;
    };
    if let Some(why) = bad_value_name(name) {
        cx.at(
            l,
            start,
            "U110",
            format!("`{name}` can't be a computed name: {why}"),
            None,
        );
        return;
    }
    let ok = matches!(l.toks.get(start + 1), Some(Tok::Word(w)) if w == "is" || w == "means")
        || matches!(l.toks.get(start + 1), Some(Tok::Sym("=")));
    if !ok {
        cx.at(
            l,
            start + 1,
            "U103",
            format!("expected `is` after `{name}`"),
            None,
        );
        return;
    }
    if let Some(e) = one_expr(cx, l, start + 2, &never) {
        app.computed.push(Computed {
            name: name.clone(),
            value: e,
            line: l.line_no,
        });
    }
}

fn parse_params(cx: &mut Ctx, l: &Line, mut i: usize) -> Vec<String> {
    let mut out = Vec::new();
    if l.toks.get(i).and_then(|t| t.word()) == Some("with") {
        i += 1;
    }
    while i < l.toks.len() {
        match &l.toks[i] {
            Tok::Sym(",") => {}
            Tok::Word(w) if w == "and" => {}
            Tok::Word(w) => {
                if let Some(why) = bad_value_name(w) {
                    cx.at(
                        l,
                        i,
                        "U110",
                        format!("`{w}` can't be a parameter name: {why}"),
                        None,
                    );
                }
                out.push(w.clone());
            }
            t => cx.at(
                l,
                i,
                "U003",
                format!("expected a parameter name, found {}", t.describe()),
                None,
            ),
        }
        i += 1;
    }
    out
}

const SCREEN_WORDS: &[&str] = &[
    "with", "tab", "icon", "titled", "title", "subtitle", "inline", "settings",
];

fn screen_stop(w: &str) -> bool {
    SCREEN_WORDS.contains(&w)
}

fn parse_screen(cx: &mut Ctx, l: &Line) -> Option<Screen> {
    let Some(Tok::Word(name)) = l.toks.get(1) else {
        cx.at(
            l,
            1,
            "U003",
            "expected a screen name, e.g. `screen Home`",
            None,
        );
        return None;
    };
    let mut s = Screen {
        name: name.clone(),
        title: None,
        subtitle: None,
        params: Vec::new(),
        tab_icon: None,
        inline: false,
        settings: false,
        toolbar: Vec::new(),
        sidebar: Vec::new(),
        body: Vec::new(),
        events: Vec::new(),
        line: l.line_no,
    };
    let mut i = 2;
    while i < l.toks.len() {
        match &l.toks[i] {
            Tok::Str(_) => {
                let mut p = ExprParser::new(&l.toks, i, &screen_stop, l.line_no);
                match p.parse() {
                    Ok(e) => s.title = Some(e),
                    Err(e) => {
                        cx.expr_err(l, e);
                        return None;
                    }
                }
                i = p.pos;
            }
            Tok::Word(w) if w == "titled" || w == "title" || w == "subtitle" => {
                let is_sub = w == "subtitle";
                let mut p = ExprParser::new(&l.toks, i + 1, &screen_stop, l.line_no);
                match p.parse() {
                    Ok(e) if is_sub => s.subtitle = Some(e),
                    Ok(e) => s.title = Some(e),
                    Err(e) => {
                        cx.expr_err(l, e);
                        return None;
                    }
                }
                i = p.pos;
            }
            Tok::Word(w) if w == "inline" => {
                s.inline = true;
                i += 1;
            }
            Tok::Word(w) if w == "settings" => {
                s.settings = true;
                i += 1;
            }
            Tok::Word(w) if w == "tab" || w == "icon" => {
                i += 1;
                if l.toks.get(i).and_then(|t| t.word()) == Some("icon") {
                    i += 1;
                }
                let icon = match l.toks.get(i) {
                    Some(Tok::Str(icon)) => {
                        i += 1;
                        icon.clone()
                    }
                    _ => String::new(),
                };
                s.tab_icon = Some(Expr::Text(vec![super::expr::TextPart::Lit(icon)]));
            }
            Tok::Word(w) if w == "with" => {
                let mut j = i + 1;
                while j < l.toks.len() {
                    match &l.toks[j] {
                        Tok::Word(w) if screen_stop(w) && w != "with" => break,
                        Tok::Str(_) => break,
                        Tok::Word(w) if w == "and" => {}
                        Tok::Word(w) => s.params.push(w.clone()),
                        Tok::Sym(",") => {}
                        t => cx.at(
                            l,
                            j,
                            "U003",
                            format!("expected a parameter name, found {}", t.describe()),
                            None,
                        ),
                    }
                    j += 1;
                }
                i = j;
            }
            t => {
                cx.at(l, i, "U003", format!("unexpected {} on screen line", t.describe()),
                    Some("screen lines look like `screen Home \"My title\" tab \"house\"`, `screen Detail with todo titled todo.title`, or `screen Prefs settings`".into()));
                return None;
            }
        }
    }
    let mut body_lines: Vec<Line> = Vec::new();
    for c in &l.children {
        match c.toks[0].word() {
            Some("toolbar") => {
                let place = match c.toks.get(1).and_then(|t| t.word()) {
                    None => "trailing",
                    Some("trailing") | Some("right") | Some("top") => "trailing",
                    Some("leading") | Some("left") => "leading",
                    Some("bottom") => "bottom",
                    Some("title") | Some("principal") | Some("center") => "title",
                    Some(other) => {
                        cx.at(
                            c,
                            1,
                            "U003",
                            format!("unknown toolbar place `{other}`"),
                            Some("toolbar places: leading, trailing, title, bottom".into()),
                        );
                        continue;
                    }
                };
                let nodes = parse_nodes(cx, &c.children);
                s.toolbar.push((place.to_string(), nodes));
            }
            Some("sidebar") => {
                if c.toks.len() > 1 {
                    cx.at(c, 1, "U003", "`sidebar` takes indented lines only", None);
                }
                s.sidebar.extend(parse_nodes(cx, &c.children));
            }
            Some("subtitle") if c.children.is_empty() => {
                if let Some(e) = one_expr(cx, c, 1, &never) {
                    s.subtitle = Some(e);
                }
            }
            Some("on") | Some("every") => {
                if let Some(ev) = parse_event(cx, c) {
                    s.events.push(ev);
                }
            }
            _ => body_lines.push(c.clone()),
        }
    }
    s.body = parse_nodes(cx, &body_lines);
    Some(s)
}

/// Parse sibling view lines (handles `if` / `otherwise` chains).
pub fn parse_nodes(cx: &mut Ctx, lines: &[Line]) -> Vec<Node> {
    let mut out: Vec<Node> = Vec::new();
    for l in lines {
        let head = l.toks[0].word().unwrap_or("");
        match head {
            "if" => {
                let cond = one_expr(cx, l, 1, &never);
                let body = parse_nodes(cx, &l.children);
                out.push(Node::If {
                    branches: vec![(cond, body)],
                    line: l.line_no,
                });
            }
            "otherwise" | "else" => {
                let (cond, body) = if l.toks.get(1).and_then(|t| t.word()) == Some("if") {
                    (one_expr(cx, l, 2, &never), parse_nodes(cx, &l.children))
                } else {
                    (None, parse_nodes(cx, &l.children))
                };
                match out.last_mut() {
                    Some(Node::If { branches, .. })
                        if branches.last().is_some_and(|b| b.0.is_some()) =>
                    {
                        branches.push((cond, body));
                    }
                    _ => cx.at(
                        l,
                        0,
                        "P024",
                        "`otherwise` must follow an `if` block at the same indent",
                        None,
                    ),
                }
            }
            "for" => {
                if let Some((var, iter)) = parse_for_head(cx, l) {
                    let body = parse_nodes(cx, &l.children);
                    out.push(Node::For {
                        var,
                        iter,
                        body,
                        line: l.line_no,
                    });
                }
            }
            "on" | "every" => cx.at(
                l,
                0,
                "U104",
                format!("`{head}` must be indented under the element it belongs to"),
                None,
            ),
            _ if is_modifier(head) => cx.at(
                l,
                0,
                "U105",
                format!("modifier `{head}` must be indented under an element"),
                Some(format!("e.g.\n  text \"Hi\"\n    {head} …")),
            ),
            _ => {
                if let Some(el) = parse_element(cx, l) {
                    out.push(Node::El(el));
                }
            }
        }
    }
    out
}

fn parse_for_head(cx: &mut Ctx, l: &Line) -> Option<(String, Expr)> {
    let mut i = 1;
    if l.toks.get(i).and_then(|t| t.word()) == Some("each") {
        i += 1;
    }
    let Some(Tok::Word(var)) = l.toks.get(i) else {
        cx.at(l, i, "U106", "expected `for each item in items`", None);
        return None;
    };
    if l.toks.get(i + 1).and_then(|t| t.word()) != Some("in") {
        cx.at(
            l,
            i + 1,
            "U106",
            "expected `in` after the loop name",
            Some(format!("for each {var} in items")),
        );
        return None;
    }
    let start = i + 2;
    // `for each n in 1 to 5`
    if let Some(to) = l.toks.iter().position(|t| t.word() == Some("to")) {
        if to > start {
            let a_toks = &l.toks[start..to];
            let b_toks = &l.toks[to + 1..];
            let mut pa = ExprParser::new(a_toks, 0, &never, l.line_no);
            let mut pb = ExprParser::new(b_toks, 0, &never, l.line_no);
            return match (pa.parse(), pb.parse()) {
                (Ok(a), Ok(b)) => Some((
                    var.clone(),
                    Expr::Of("__range".into(), Box::new(Expr::List(vec![a, b]))),
                )),
                (Err(e), _) | (_, Err(e)) => {
                    cx.expr_err(l, e);
                    None
                }
            };
        }
    }
    one_expr(cx, l, start, &never).map(|e| (var.clone(), e))
}

fn tail_stop_for(kind: &str) -> impl Fn(&str) -> bool + '_ {
    move |w: &str| {
        is_modifier(w)
            || element_kwargs(kind).contains(&canonical_kwarg(w))
            || w == "do"
            || w == "if"
    }
}

pub fn parse_element(cx: &mut Ctx, l: &Line) -> Option<Element> {
    let kind = match &l.toks[0] {
        Tok::Word(w) => w.clone(),
        t => {
            cx.at(
                l,
                0,
                "U001",
                format!("expected an element name, found {}", t.describe()),
                Some(format!("e.g. `text {}`", t.describe())),
            );
            return None;
        }
    };
    let is_component = kind.chars().next().is_some_and(|c| c.is_uppercase());
    if !is_component && !is_element(&kind) {
        let hint = did_you_mean(&kind, ELEMENTS);
        cx.at(
            l,
            0,
            "U001",
            format!(
                "unknown element `{kind}`{}",
                if hint.is_empty() {
                    String::new()
                } else {
                    format!(";{hint}")
                }
            ),
            Some(format!("elements: {}", ELEMENTS.join(", "))),
        );
        return None;
    }
    let mut el = Element {
        kind,
        args: Vec::new(),
        kw: Vec::new(),
        mods: Vec::new(),
        events: Vec::new(),
        children: Vec::new(),
        line: l.line_no,
    };
    parse_tail(cx, l, 1, &mut el);
    let mut kid_lines: Vec<Line> = Vec::new();
    for c in &l.children {
        let head = c.toks[0].word().unwrap_or("");
        if head == "on" || head == "every" {
            if let Some(ev) = parse_event(cx, c) {
                el.events.push(ev);
            }
        } else if is_modifier(head) {
            parse_tail(cx, c, 0, &mut el);
            if !c.children.is_empty() {
                cx.at(c, 0, "U105", "modifiers cannot have indented lines", None);
            }
        } else if element_kwargs(&el.kind).contains(&canonical_kwarg(head)) && head != "to" {
            parse_tail(cx, c, 0, &mut el);
        } else {
            kid_lines.push(c.clone());
        }
    }
    el.children = parse_nodes(cx, &kid_lines);
    Some(el)
}

/// Parse `args… kw value… modifiers… do action` from token `i`.
fn parse_tail(cx: &mut Ctx, l: &Line, mut i: usize, el: &mut Element) {
    let kind = el.kind.clone();
    let tail_stop = tail_stop_for(&kind);
    let kwargs = element_kwargs(&kind);
    while i < l.toks.len() {
        let t = &l.toks[i];
        let w = t.word().unwrap_or("");
        if w == "do" {
            let actions = parse_inline_actions(cx, l, i + 1);
            el.events.push(EventBlock {
                kind: "tap".into(),
                arg: None,
                millis: None,
                actions,
                line: l.line_no,
            });
            return;
        }
        if t.is_sym(",") {
            i += 1;
            continue;
        }
        if is_modifier(w) {
            i = parse_modifier(cx, l, i, el);
            continue;
        }
        if kwargs.contains(&canonical_kwarg(w)) {
            let name = canonical_kwarg(w).to_string();
            i += 1;
            if name == "bind" || name == "scope" {
                // a value name, not an expression: `bind size`, `bind todo.done`
                match path_expr(cx, l, i, &never) {
                    Some((e, j)) => {
                        el.kw.push((name, vec![e]));
                        i = j;
                        continue;
                    }
                    None => return,
                }
            }
            let mut vals = Vec::new();
            loop {
                let mut p = ExprParser::new(&l.toks, i, &tail_stop, l.line_no);
                match p.parse() {
                    Ok(e) => vals.push(e),
                    Err(e) => {
                        cx.expr_err(l, e);
                        return;
                    }
                }
                i = p.pos;
                // `options "S", "M", "L"` / `with a, b`
                if matches!(name.as_str(), "options" | "with" | "suggest" | "scopes")
                    && l.toks.get(i).is_some_and(|t| t.is_sym(","))
                {
                    i += 1;
                    continue;
                }
                break;
            }
            el.kw.push((name, vals));
            continue;
        }
        let mut p = ExprParser::new(&l.toks, i, &tail_stop, l.line_no);
        match p.parse() {
            Ok(e) => el.args.push(e),
            Err(e) => {
                let mut e = e;
                if let Some(Tok::Word(bad)) = l.toks.get(e.tok) {
                    if p.pos == i && !is_operator_word(bad) {
                        let hint = did_you_mean(bad, MODIFIERS);
                        e.message = format!("`{bad}` is not a modifier or value here{hint}");
                        e.help = Some(format!("modifiers: {}", MODIFIERS.join(", ")));
                    }
                }
                cx.expr_err(l, e);
                return;
            }
        }
        if p.pos == i {
            cx.at(l, i, "U102", format!("unexpected {}", t.describe()), None);
            return;
        }
        i = p.pos;
    }
}

fn parse_modifier(cx: &mut Ctx, l: &Line, start: usize, el: &mut Element) -> usize {
    let raw = l.toks[start].word().unwrap().to_string();
    let name = canonical_modifier(&raw).to_string();
    let mut i = start + 1;
    let mut values = Vec::new();
    let (min, max) = modifier_arity(&name);
    // `padding horizontal 20`, `border red 2`, `offset 4 8`
    while values.len() < max && i < l.toks.len() {
        let t = &l.toks[i];
        if let Some(w) = t.word() {
            if w == "if"
                || w == "do"
                || w == "unless"
                || (is_modifier(w) && !modifier_takes_word(&name, w))
                || element_kwargs(&el.kind).contains(&canonical_kwarg(w))
            {
                break;
            }
        }
        if t.is_sym(",") {
            break;
        }
        let kind = el.kind.clone();
        let base = tail_stop_for(&kind);
        let stop = |w: &str| base(w) || w == "unless";
        let mut p = ExprParser::new(&l.toks, i, &stop, l.line_no);
        match p.parse_value_only() {
            Ok(e) => values.push(e),
            Err(e) => {
                cx.expr_err(l, e);
                return l.toks.len();
            }
        }
        i = p.pos;
    }
    if values.len() < min {
        cx.at(
            l,
            i,
            "U107",
            format!("`{raw}` needs a value"),
            Some(modifier_help(&name)),
        );
    }
    let mut cond = None;
    if let Some(w) = l.toks.get(i).and_then(|t| t.word()) {
        if w == "if" || w == "when" || w == "unless" {
            let neg = w == "unless";
            let kind = el.kind.clone();
            let tail_stop = tail_stop_for(&kind);
            let mut p = ExprParser::new(&l.toks, i + 1, &tail_stop, l.line_no);
            match p.parse() {
                Ok(e) => cond = Some(if neg { Expr::Not(Box::new(e)) } else { e }),
                Err(e) => {
                    cx.expr_err(l, e);
                    return l.toks.len();
                }
            }
            i = p.pos;
        }
    }
    el.mods.push(Modifier {
        name,
        values,
        cond,
        line: l.line_no,
    });
    i
}

impl<'a> ExprParser<'a> {
    /// A modifier value: a single additive expression (no comparisons, so `if` can follow).
    pub fn parse_value_only(&mut self) -> Result<Expr, ExprError> {
        if self.at_end() {
            return Err(ExprError {
                message: "expected a value".into(),
                help: None,
                tok: self.pos,
            });
        }
        // Reuse the full parser but values rarely need more than `a + b`.
        let save = self.pos;
        match self.peek_tok().cloned() {
            Some(Tok::Word(w))
                if w != "random" && w != "if" && !super::expr::OF_FUNCS.contains(&w.as_str()) =>
            {
                // plain word (color name, enum, or a state name) possibly with `.field`
                self.pos += 1;
                let mut e = Expr::Name(w.clone());
                while self.peek_tok().is_some_and(|t| t.is_sym(".")) {
                    if let Some(Tok::Word(f)) = self.toks.get(self.pos + 1) {
                        e = Expr::Member(Box::new(e), f.clone());
                        self.pos += 2;
                    } else {
                        break;
                    }
                }
                let _ = save;
                Ok(e)
            }
            _ => self.parse_sum(),
        }
    }
    fn peek_tok(&self) -> Option<&Tok> {
        self.toks.get(self.pos)
    }
}

fn parse_event(cx: &mut Ctx, l: &Line) -> Option<EventBlock> {
    let head = l.toks[0].word().unwrap_or("");
    let mut ev = EventBlock {
        kind: String::new(),
        arg: None,
        millis: None,
        actions: Vec::new(),
        line: l.line_no,
    };
    let mut i;
    if head == "every" {
        // every 2 seconds / every second
        let (ms, next) = parse_duration(l, 1);
        let Some(ms) = ms else {
            cx.at(
                l,
                1,
                "U108",
                "expected a time like `every 1 second` or `every 500 ms`",
                None,
            );
            return None;
        };
        ev.kind = "every".into();
        ev.millis = Some(ms);
        i = next;
    } else {
        let words: Vec<&str> = l.toks.iter().skip(1).filter_map(|t| t.word()).collect();
        let (kind, used) = match words.as_slice() {
            ["double", "tap", ..] | ["double", "click", ..] => ("doubletap", 2),
            ["long", "press", ..] => ("longpress", 2),
            ["tap", ..] | ["click", ..] | ["press", ..] => ("tap", 1),
            ["hover", ..] => ("hover", 1),
            ["appear", ..] | ["start", ..] | ["load", ..] | ["open", ..] => ("appear", 1),
            ["change", ..] => ("change", 1),
            ["submit", ..] | ["enter", ..] | ["return", ..] => ("submit", 1),
            ["delete", ..] | ["swipe", ..] => ("delete", 1),
            ["disappear", ..] | ["leave", ..] => ("disappear", 1),
            ["background", ..] | ["hidden", ..] | ["inactive", ..] => ("background", 1),
            ["foreground", ..] | ["active", ..] | ["visible", ..] | ["resume", ..] => {
                ("foreground", 1)
            }
            ["url", ..] | ["link", ..] => ("url", 1),
            _ => {
                cx.at(l, 1, "U104", "unknown event",
                    Some("events: on tap, on double tap, on long press, on hover, on appear, on change of x, on submit, on delete, on background, on foreground, on url, every 1 second".into()));
                return None;
            }
        };
        ev.kind = kind.into();
        i = 1 + used;
        if kind == "change" {
            if l.toks.get(i).and_then(|t| t.word()) == Some("of") {
                i += 1;
            }
            let stop = |w: &str| w == "do";
            let mut p = ExprParser::new(&l.toks, i, &stop, l.line_no);
            match p.parse() {
                Ok(e) => ev.arg = Some(e),
                Err(e) => {
                    cx.expr_err(l, e);
                    return None;
                }
            }
            i = p.pos;
        }
    }
    if l.toks.get(i).and_then(|t| t.word()) == Some("do") {
        ev.actions = parse_inline_actions(cx, l, i + 1);
    } else if i < l.toks.len() {
        cx.at(
            l,
            i,
            "U104",
            format!("unexpected {} after event", l.toks[i].describe()),
            Some("put actions on indented lines, or after `do`".into()),
        );
    }
    ev.actions.extend(parse_actions(cx, &l.children));
    if ev.actions.is_empty() {
        cx.at(
            l,
            0,
            "U104",
            "this event has no actions",
            Some("add indented actions, e.g.\n  on tap\n    count += 1".into()),
        );
    }
    Some(ev)
}

fn parse_duration(l: &Line, i: usize) -> (Option<f64>, usize) {
    let (n, mut j) = match l.toks.get(i) {
        Some(Tok::Num(n)) => (n.parse::<f64>().unwrap_or(1.0), i + 1),
        _ => (1.0, i),
    };
    let unit = l.toks.get(j).and_then(|t| t.word()).unwrap_or("");
    let ms = match unit {
        "ms" | "millisecond" | "milliseconds" => n,
        "s" | "sec" | "second" | "seconds" => n * 1000.0,
        "minute" | "minutes" | "min" => n * 60_000.0,
        "hour" | "hours" => n * 3_600_000.0,
        _ => return (None, j),
    };
    j += 1;
    (Some(ms), j)
}

fn parse_inline_actions(cx: &mut Ctx, l: &Line, start: usize) -> Vec<Action> {
    if start >= l.toks.len() {
        cx.at(
            l,
            start,
            "U109",
            "expected an action after `do`",
            Some("e.g. `button \"+\" do count += 1`".into()),
        );
        return Vec::new();
    }
    // `do a, then b` / `do a then b`
    let mut out = Vec::new();
    let mut seg_start = start;
    let mut i = start;
    let mut depth_if = 0;
    while i <= l.toks.len() {
        let at_split = i == l.toks.len()
            || (l.toks[i].word() == Some("then") && depth_if == 0)
            || (l.toks[i].is_sym(",") && l.toks.get(i + 1).and_then(|t| t.word()) == Some("then"));
        if i < l.toks.len() {
            match l.toks[i].word() {
                Some("if") => depth_if += 1,
                Some("else") | Some("otherwise") if depth_if > 0 => depth_if -= 1,
                _ => {}
            }
        }
        if at_split {
            if i > seg_start {
                let seg = Line {
                    indent: l.indent,
                    line_no: l.line_no,
                    toks: l.toks[seg_start..i].to_vec(),
                    cols: l.cols[seg_start..i].to_vec(),
                    children: Vec::new(),
                };
                if let Some(a) = parse_action(cx, &seg, &[]) {
                    out.push(a);
                }
            }
            if i < l.toks.len() && l.toks[i].is_sym(",") {
                i += 1;
            }
            seg_start = i + 1;
        }
        i += 1;
    }
    out
}

pub fn parse_actions(cx: &mut Ctx, lines: &[Line]) -> Vec<Action> {
    let mut out: Vec<Action> = Vec::new();
    for l in lines {
        let head = l.toks[0].word().unwrap_or("");
        if head == "otherwise" || head == "else" {
            let cond = if l.toks.get(1).and_then(|t| t.word()) == Some("if") {
                one_expr(cx, l, 2, &never)
            } else {
                None
            };
            let body = parse_actions(cx, &l.children);
            if let Some(Action::At(_, b)) = out.last_mut() {
                if let Action::If(branches) = b.as_mut() {
                    if branches.last().is_some_and(|b| b.0.is_some()) {
                        branches.push((cond, body));
                        continue;
                    }
                }
            }
            cx.at(
                l,
                0,
                "P024",
                "`otherwise` must follow an `if` at the same indent",
                None,
            );
            continue;
        }
        if let Some(a) = parse_action(cx, l, &l.children) {
            out.push(a);
        }
    }
    out
}

fn path_expr(
    cx: &mut Ctx,
    l: &Line,
    i: usize,
    stop: &dyn Fn(&str) -> bool,
) -> Option<(Expr, usize)> {
    let Some(Tok::Word(name)) = l.toks.get(i) else {
        cx.at(l, i, "U109", "expected a value name", None);
        return None;
    };
    let _ = stop;
    let mut e = Expr::Name(name.clone());
    let mut j = i + 1;
    while l.toks.get(j).is_some_and(|t| t.is_sym(".")) {
        if let Some(Tok::Word(f)) = l.toks.get(j + 1) {
            e = Expr::Member(Box::new(e), f.clone());
            j += 2;
        } else {
            cx.at(l, j, "U109", "expected a field name after `.`", None);
            return None;
        }
    }
    Some((e, j))
}

fn rest_expr(
    cx: &mut Ctx,
    l: &Line,
    i: usize,
    stop: &dyn Fn(&str) -> bool,
) -> Option<(Expr, usize)> {
    let mut p = ExprParser::new(&l.toks, i, stop, l.line_no);
    p.comma_stops = false;
    match p.parse() {
        Ok(e) => Some((e, p.pos)),
        Err(e) => {
            cx.expr_err(l, e);
            None
        }
    }
}

fn expect_end(cx: &mut Ctx, l: &Line, i: usize) -> bool {
    if i < l.toks.len() {
        cx.at(
            l,
            i,
            "U109",
            format!("unexpected {} at end of action", l.toks[i].describe()),
            None,
        );
        return false;
    }
    true
}

/// `with a, b` at token `i` (or nothing), then end of line.
fn parse_with_args(cx: &mut Ctx, l: &Line, i: usize) -> Option<Vec<Expr>> {
    let mut args = Vec::new();
    if l.toks.get(i).and_then(|t| t.word()) == Some("with") {
        let mut j = i + 1;
        loop {
            let mut p = ExprParser::new(&l.toks, j, &never, l.line_no);
            match p.parse() {
                Ok(e) => args.push(e),
                Err(e) => {
                    cx.expr_err(l, e);
                    return None;
                }
            }
            j = p.pos;
            if l.toks.get(j).is_some_and(|t| t.is_sym(",")) {
                j += 1;
                continue;
            }
            break;
        }
        if !expect_end(cx, l, j) {
            return None;
        }
    } else if !expect_end(cx, l, i) {
        return None;
    }
    Some(args)
}

/// Presentation styles for `show Screen as …` (SwiftUI sheet / detents / cover / popover / inspector).
const SHOW_STYLES: &[&str] = &[
    "sheet",
    "half",
    "full",
    "cover",
    "popover",
    "inspector",
    "large",
    "medium",
];

fn parse_action(cx: &mut Ctx, l: &Line, children: &[Line]) -> Option<Action> {
    let a = parse_action_inner(cx, l, children)?;
    Some(Action::At(l.line_no, Box::new(a)))
}

fn parse_action_inner(cx: &mut Ctx, l: &Line, children: &[Line]) -> Option<Action> {
    let head = match &l.toks[0] {
        Tok::Word(w) => w.as_str(),
        t => {
            cx.at(
                l,
                0,
                "U109",
                format!("expected an action, found {}", t.describe()),
                Some(ACTION_HELP.into()),
            );
            return None;
        }
    };
    let word = |i: usize| l.toks.get(i).and_then(|t| t.word());
    let with_args = |cx: &mut Ctx, i: usize| -> Option<Vec<Expr>> { parse_with_args(cx, l, i) };
    let screen_name = |cx: &mut Ctx, i: usize| -> Option<String> {
        match l.toks.get(i) {
            Some(Tok::Word(n)) => Some(n.clone()),
            _ => {
                cx.at(l, i, "U109", "expected a screen name", None);
                None
            }
        }
    };
    if !children.is_empty()
        && !matches!(
            head,
            "if" | "for" | "ask" | "confirm" | "animate" | "repeat" | "otherwise" | "else"
        )
    {
        cx.at(
            l,
            0,
            "U109",
            format!("`{head}` cannot have indented lines"),
            None,
        );
    }
    Some(match head {
        "set" => {
            let (target, i) = path_expr(cx, l, 1, &never)?;
            if word(i) != Some("to") {
                cx.at(l, i, "U109", "expected `to` in `set x to value`", None);
                return None;
            }
            let (v, j) = rest_expr(cx, l, i + 1, &never)?;
            expect_end(cx, l, j);
            Action::Set(target, v)
        }
        "toggle" | "flip" => {
            let (target, i) = path_expr(cx, l, 1, &never)?;
            expect_end(cx, l, i);
            Action::Toggle(target)
        }
        "increase" | "decrease" | "increment" | "decrement" => {
            let (target, i) = path_expr(cx, l, 1, &never)?;
            let amount = if word(i) == Some("by") {
                let (v, j) = rest_expr(cx, l, i + 1, &never)?;
                expect_end(cx, l, j);
                v
            } else {
                expect_end(cx, l, i);
                Expr::Num(1.0)
            };
            Action::Change(
                target,
                if head.starts_with("inc") { '+' } else { '-' },
                amount,
            )
        }
        "add" | "append" | "insert" => {
            if word(1) == Some("to") {
                // add to todos with title draft, done false
                let (target, i) = path_expr(cx, l, 2, &never)?;
                let (v, j) = rest_expr(cx, l, i, &never)?;
                expect_end(cx, l, j);
                return Some(Action::Add(v, target));
            }
            let stop = |w: &str| w == "to";
            let (v, i) = rest_expr(cx, l, 1, &stop)?;
            if word(i) != Some("to") {
                cx.at(l, i, "U109", "expected `to` in `add item to list`", None);
                return None;
            }
            let (target, j) = path_expr(cx, l, i + 1, &never)?;
            expect_end(cx, l, j);
            Action::Add(v, target)
        }
        "remove" | "delete" => {
            if word(1) == Some("from") {
                let (target, i) = path_expr(cx, l, 2, &never)?;
                if word(i) != Some("where") {
                    cx.at(
                        l,
                        i,
                        "U109",
                        "expected `where` in `remove from todos where done`",
                        None,
                    );
                    return None;
                }
                let (cond, j) = rest_expr(cx, l, i + 1, &never)?;
                expect_end(cx, l, j);
                return Some(Action::RemoveWhere(target, cond));
            }
            let stop = |w: &str| w == "from";
            let (v, i) = rest_expr(cx, l, 1, &stop)?;
            if word(i) != Some("from") {
                cx.at(
                    l,
                    i,
                    "U109",
                    "expected `from` in `remove item from list`",
                    None,
                );
                return None;
            }
            let (target, j) = path_expr(cx, l, i + 1, &never)?;
            if word(j) == Some("where") {
                cx.at(l, j, "U109", "write `remove from list where …`", None);
                return None;
            }
            expect_end(cx, l, j);
            Action::Remove(v, target)
        }
        "clear" | "reset" | "empty" => {
            let (target, i) = path_expr(cx, l, 1, &never)?;
            expect_end(cx, l, i);
            Action::Clear(target)
        }
        "sort" => {
            let (target, mut i) = path_expr(cx, l, 1, &never)?;
            let mut field = None;
            if word(i) == Some("by") {
                field = word(i + 1).map(String::from);
                i += 2;
            }
            let desc = matches!(word(i), Some("descending") | Some("reversed"));
            if desc || word(i) == Some("ascending") {
                i += 1;
            }
            expect_end(cx, l, i);
            Action::Sort(target, field, desc)
        }
        "shuffle" => {
            let (target, i) = path_expr(cx, l, 1, &never)?;
            expect_end(cx, l, i);
            Action::Shuffle(target)
        }
        "go" | "navigate" | "push" => {
            let mut i = 1;
            match word(i) {
                Some("back") => {
                    expect_end(cx, l, 2);
                    return Some(Action::GoBack);
                }
                Some("home") => {
                    expect_end(cx, l, 2);
                    return Some(Action::GoHome);
                }
                Some("to") => i += 1,
                _ => {}
            }
            let name = screen_name(cx, i)?;
            let args = with_args(cx, i + 1)?;
            Action::GoTo(name, args)
        }
        "back" | "dismiss" if l.toks.len() == 1 => Action::GoBack,
        "show" | "present" | "sheet" | "cover" | "popover" | "inspector" => {
            let name = screen_name(cx, 1)?;
            // trailing `as half`, `as popover`, `locked`
            let mut end = l.toks.len();
            let mut locked = false;
            let mut style = match head {
                "cover" => "full",
                "popover" => "popover",
                "inspector" => "inspector",
                _ => "sheet",
            }
            .to_string();
            if word(end - 1) == Some("locked") {
                locked = true;
                end -= 1;
            }
            if end >= 2 && word(end - 2) == Some("as") {
                match word(end - 1) {
                    Some(st) if SHOW_STYLES.contains(&st) => {
                        style = match st {
                            "cover" => "full",
                            "medium" => "half",
                            "large" => "sheet",
                            other => other,
                        }
                        .to_string();
                        end -= 2;
                    }
                    _ => {
                        cx.at(l, end - 1, "U109", "unknown presentation style",
                            Some("styles: sheet, half, full, popover, inspector (add `locked` to stop swipe-to-close)".into()));
                        return None;
                    }
                }
            }
            if word(end - 1) == Some("locked") {
                locked = true;
                end -= 1;
            }
            let trimmed = Line {
                indent: l.indent,
                line_no: l.line_no,
                toks: l.toks[..end].to_vec(),
                cols: l.cols[..end].to_vec(),
                children: Vec::new(),
            };
            let args = parse_with_args(cx, &trimmed, 2)?;
            Action::Show(name, args, style, locked)
        }
        "close" | "hide" if l.toks.len() == 1 => Action::Close,
        "alert" | "ask" | "confirm" => {
            let stop = |w: &str| w == "message" || w == "saying";
            let (title, mut i) = rest_expr(cx, l, 1, &stop)?;
            let mut msg = None;
            if matches!(word(i), Some("message") | Some("saying")) {
                let (m, j) = rest_expr(cx, l, i + 1, &never)?;
                msg = Some(m);
                i = j;
            }
            expect_end(cx, l, i);
            if head == "alert" {
                Action::Alert(title, msg)
            } else {
                let body = parse_actions(cx, children);
                if body.is_empty() {
                    cx.at(
                        l,
                        0,
                        "U109",
                        format!("`{head}` needs indented actions to run when confirmed"),
                        None,
                    );
                }
                Action::Ask(title, msg, body)
            }
        }
        "toast" | "notify" => {
            let (v, i) = rest_expr(cx, l, 1, &never)?;
            expect_end(cx, l, i);
            Action::Toast(v)
        }
        "wait" | "pause" | "sleep" => {
            let mut i = 1;
            if word(i) == Some("for") {
                i += 1;
            }
            let (ms, j) = parse_duration(l, i);
            let Some(ms) = ms else {
                cx.at(
                    l,
                    i,
                    "U109",
                    "expected a time like `wait 1 second` or `wait 300 ms`",
                    None,
                );
                return None;
            };
            expect_end(cx, l, j);
            Action::Wait(ms)
        }
        "new" | "open" | "save" if word(1) == Some("document") && l.toks.len() == 2 => {
            Action::Document(head.to_string())
        }
        "open" if word(1) == Some("window") => {
            let name = screen_name(cx, 2)?;
            let args = with_args(cx, 3)?;
            Action::OpenWindow(name, args)
        }
        "open" | "import" if word(1) == Some("file") || head == "import" => {
            // open file into notes [as text|json|image]
            let mut i = if word(1) == Some("file") { 2 } else { 1 };
            if word(i) != Some("into") {
                cx.at(
                    l,
                    i,
                    "U109",
                    "expected `into`, e.g. `open file into notes`",
                    None,
                );
                return None;
            }
            i += 1;
            let (target, mut j) = path_expr(cx, l, i, &never)?;
            let mut kind = "auto".to_string();
            if word(j) == Some("as") {
                match word(j + 1) {
                    Some(k @ ("text" | "json" | "image" | "data")) => kind = k.to_string(),
                    _ => {
                        cx.at(l, j + 1, "U109", "file kinds: text, json, image", None);
                        return None;
                    }
                }
                j += 2;
            }
            expect_end(cx, l, j);
            Action::OpenFile(target, kind)
        }
        "save" | "export" | "download" if l.toks.len() > 1 && word(1) != Some("with") => {
            // save notes to file "notes.json"
            let stop = |w: &str| w == "to" || w == "as";
            let (v, mut i) = rest_expr(cx, l, 1, &stop)?;
            if matches!(word(i), Some("to") | Some("as")) {
                i += 1;
            }
            if word(i) == Some("file") {
                i += 1;
            }
            let (name, j) = if i < l.toks.len() {
                rest_expr(cx, l, i, &never)?
            } else {
                (
                    Expr::Text(vec![super::expr::TextPart::Lit("data.json".into())]),
                    i,
                )
            };
            expect_end(cx, l, j);
            Action::SaveFile(v, name)
        }
        "open" | "visit" => {
            let (v, i) = rest_expr(cx, l, 1, &never)?;
            expect_end(cx, l, i);
            Action::Open(v)
        }
        "copy" => {
            let (v, i) = rest_expr(cx, l, 1, &never)?;
            expect_end(cx, l, i);
            Action::Copy(v)
        }
        "share" => {
            let (v, i) = rest_expr(cx, l, 1, &never)?;
            expect_end(cx, l, i);
            Action::Share(v)
        }
        "say" | "print" | "log" => {
            let (v, i) = rest_expr(cx, l, 1, &never)?;
            expect_end(cx, l, i);
            Action::Say(v)
        }
        "load" | "fetch" => {
            let stop = |w: &str| w == "into";
            let (url, i) = rest_expr(cx, l, 1, &stop)?;
            if word(i) != Some("into") {
                cx.at(
                    l,
                    i,
                    "U109",
                    "expected `into` in `load \"https://…\" into posts`",
                    None,
                );
                return None;
            }
            let (target, j) = path_expr(cx, l, i + 1, &never)?;
            expect_end(cx, l, j);
            Action::Load(url, target)
        }
        "vibrate" | "buzz" | "haptic" => Action::Vibrate,
        "run" | "call" => {
            let Some(Tok::Word(n)) = l.toks.get(1) else {
                cx.at(l, 1, "U109", "expected an action name after `run`", None);
                return None;
            };
            let args = with_args(cx, 2)?;
            Action::Run(n.clone(), args)
        }
        "if" => {
            let cond = one_expr(cx, l, 1, &never)?;
            Action::If(vec![(Some(cond), parse_actions(cx, children))])
        }
        "for" => {
            let (var, iter) = parse_for_head(cx, l)?;
            Action::For(var, iter, parse_actions(cx, children))
        }
        "repeat" => {
            let stop = |w: &str| w == "times";
            let (n, i) = rest_expr(cx, l, 1, &stop)?;
            if word(i) == Some("times") {
                expect_end(cx, l, i + 1);
            } else {
                expect_end(cx, l, i);
            }
            Action::For(
                "__i".into(),
                Expr::Of(
                    "__range".into(),
                    Box::new(Expr::List(vec![Expr::Num(1.0), n])),
                ),
                parse_actions(cx, children),
            )
        }
        "animate" | "animated" | "smoothly" => {
            let mut body = parse_actions(cx, children);
            if l.toks.len() > 1 {
                body.extend(parse_inline_actions(cx, l, 1));
            }
            Action::Animate(body)
        }
        _ => {
            // assignment forms: x = v, x += v, x -= v, todo.done = true
            let (target, i) = path_expr(cx, l, 0, &never)?;
            match l.toks.get(i) {
                Some(Tok::Sym("=")) => {
                    let (v, j) = rest_expr(cx, l, i + 1, &never)?;
                    expect_end(cx, l, j);
                    Action::Set(target, v)
                }
                Some(Tok::Sym(op @ ("+=" | "-="))) => {
                    let (v, j) = rest_expr(cx, l, i + 1, &never)?;
                    expect_end(cx, l, j);
                    Action::Change(target, if *op == "+=" { '+' } else { '-' }, v)
                }
                None if matches!(target, Expr::Name(_)) => {
                    // bare action name: `reset`
                    let Expr::Name(n) = target else {
                        unreachable!()
                    };
                    Action::Run(n, Vec::new())
                }
                Some(Tok::Word(w)) if w == "with" && matches!(target, Expr::Name(_)) => {
                    let Expr::Name(n) = target else {
                        unreachable!()
                    };
                    let args = with_args(cx, i)?;
                    Action::Run(n, args)
                }
                _ => {
                    cx.at(
                        l,
                        0,
                        "U109",
                        format!("`{head}` is not an action"),
                        Some(ACTION_HELP.into()),
                    );
                    return None;
                }
            }
        }
    })
}

const ACTION_HELP: &str = "actions: x = 5, x += 1, toggle x, add x to list, remove x from list, clear x, go to Screen, go back, show Screen, close, alert \"…\", ask \"…\", toast \"…\", wait 1 second, open \"url\", copy x, load \"url\" into x, run name, if …, for each …";

pub fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}
