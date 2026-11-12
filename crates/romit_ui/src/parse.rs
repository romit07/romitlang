use crate::diag::{snippet_at, Diagnostic};
use crate::parts::{allowed_child, is_valid_part, valid_parts_list};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenLine {
    pub category: String,
    pub name: String,
    pub value: String,
    pub line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiNode {
    pub kind: String,
    pub line: usize,
    pub strings: Vec<String>,
    pub attrs: Vec<(String, String)>,
    pub theme_tokens: Vec<TokenLine>,
    pub children: Vec<UiNode>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiFile {
    pub uses_ui: bool,
    pub template_name: Option<String>,
    pub page: Option<UiNode>,
}

struct RawLine {
    indent: usize,
    line_no: usize,
    text: String,
}

pub fn is_ui_source(source: &str) -> bool {
    for line in source.lines() {
        let t = line.trim();
        if t.is_empty()
            || t == "note"
            || t.starts_with("note ")
            || t.starts_with("# ")
            || t == "#"
            || t.starts_with("//")
        {
            continue;
        }
        return t == "use ui" || t.starts_with("use ui ");
    }
    false
}

pub fn parse(source: &str, file: &str) -> Result<UiFile, Vec<Diagnostic>> {
    let mut diags = Vec::new();
    let raw = match collect_lines(source, file) {
        Ok(r) => r,
        Err(d) => return Err(vec![d]),
    };
    if raw.is_empty() {
        return Err(vec![Diagnostic::error(
            "U001",
            "empty UI file",
            file,
            1,
            1,
            "",
        )]);
    }

    let mut uses_ui = false;
    let mut template_name = None;
    let mut page_start: Option<usize> = None;

    for (i, rl) in raw.iter().enumerate() {
        if rl.indent != 0 {
            continue;
        }
        let words = split_words(&rl.text);
        if words.is_empty() {
            continue;
        }
        match words[0].as_str() {
            "use" if words.len() >= 2 && words[1] == "ui" => uses_ui = true,
            "use" if words.len() >= 3 && words[1] == "template" => {
                template_name = Some(words[2].clone());
            }
            "use" => diags.push(unknown_use(file, source, rl.line_no, &words[1])),
            "page" => {
                if page_start.is_some() {
                    diags.push(Diagnostic::error(
                        "U003",
                        "only one page per file is supported in this MVP",
                        file,
                        rl.line_no,
                        1,
                        snippet_at(source, rl.line_no, 1),
                    ));
                } else {
                    page_start = Some(i);
                }
            }
            w if is_valid_part(w) => {}
            other => diags.push(unknown_part(file, source, rl.line_no, other)),
        }
    }

    if !uses_ui {
        diags.push(
            Diagnostic::error(
                "U002",
                "UI files must start with `use ui`",
                file,
                1,
                1,
                snippet_at(source, 1, 1),
            )
            .with_help("add a top-level line: use ui"),
        );
    }

    if !diags.is_empty() {
        return Err(diags);
    }

    let template_name = template_name.or_else(|| Some("default".to_string()));

    let page = if let Some(idx) = page_start {
        match build_tree(&raw, idx, file, source, None) {
            Ok(node) => Some(node),
            Err(mut d) => {
                diags.append(&mut d);
                return Err(diags);
            }
        }
    } else {
        diags.push(Diagnostic::error(
            "U003",
            "missing `page` declaration",
            file,
            1,
            1,
            snippet_at(source, 1, 1),
        ));
        return Err(diags);
    };

    Ok(UiFile {
        uses_ui,
        template_name,
        page,
    })
}

fn collect_lines(source: &str, file: &str) -> Result<Vec<RawLine>, Diagnostic> {
    let mut out = Vec::new();
    for (i, line) in source.lines().enumerate() {
        let line_no = i + 1;
        if line.contains('\t') {
            return Err(Diagnostic::error(
                "P001",
                "tabs are not allowed; use 2 spaces per indent level",
                file,
                line_no,
                1,
                snippet_at(source, line_no, 1),
            ));
        }
        let mut indent = 0usize;
        for ch in line.chars() {
            if ch == ' ' {
                indent += 1;
            } else {
                break;
            }
        }
        if indent % 2 != 0 {
            return Err(Diagnostic::error(
                "P002",
                "indent must be a multiple of two spaces",
                file,
                line_no,
                indent,
                snippet_at(source, line_no, indent),
            ));
        }
        let text = line[indent..].trim();
        if text.is_empty() || text.starts_with("note ") {
            continue;
        }
        out.push(RawLine {
            indent,
            line_no,
            text: text.to_string(),
        });
    }
    Ok(out)
}

fn build_tree(
    raw: &[RawLine],
    idx: usize,
    file: &str,
    source: &str,
    parent: Option<&str>,
) -> Result<UiNode, Vec<Diagnostic>> {
    let rl = &raw[idx];
    let words = split_words(&rl.text);
    if words.is_empty() {
        return Err(vec![Diagnostic::error(
            "U001",
            "expected a part name",
            file,
            rl.line_no,
            1,
            snippet_at(source, rl.line_no, 1),
        )]);
    }
    let kind = words[0].clone();
    if !is_valid_part(&kind) {
        return Err(vec![unknown_part(file, source, rl.line_no, &kind)]);
    }
    if let Some(p) = parent {
        if !allowed_child(p, &kind) && kind != "theme" {
            return Err(vec![Diagnostic::error(
                "U001",
                format!("`{kind}` is not allowed inside `{p}`"),
                file,
                rl.line_no,
                1,
                snippet_at(source, rl.line_no, 1),
            )
            .with_help(format!(
                "valid parts at this level depend on `{p}`; top-level parts: page"
            ))]);
        }
    }

    let (strings, attrs, theme_tokens) = if kind == "theme" {
        (
            Vec::new(),
            Vec::new(),
            parse_theme_body(raw, idx, file, source)?,
        )
    } else {
        let (s, a) = parse_line_tail(&words[1..], file, source, rl.line_no)?;
        (s, a, Vec::new())
    };

    let base_indent = rl.indent;
    let mut children = Vec::new();
    let mut i = idx + 1;
    if kind == "theme" {
        while i < raw.len() && raw[i].indent > base_indent {
            i += 1;
        }
    } else {
        while i < raw.len() {
            let next = &raw[i];
            if next.indent <= base_indent {
                break;
            }
            if next.indent != base_indent + 2 {
                return Err(vec![Diagnostic::error(
                    "P002",
                    "expected indent two spaces deeper than parent",
                    file,
                    next.line_no,
                    1,
                    snippet_at(source, next.line_no, 1),
                )]);
            }
            let child = build_tree(raw, i, file, source, Some(&kind))?;
            i = skip_subtree(raw, i);
            children.push(child);
        }
    }

    Ok(UiNode {
        kind,
        line: rl.line_no,
        strings,
        attrs,
        theme_tokens,
        children,
    })
}

fn skip_subtree(raw: &[RawLine], start: usize) -> usize {
    let base = raw[start].indent;
    let mut i = start + 1;
    while i < raw.len() && raw[i].indent > base {
        i += 1;
    }
    i
}

fn parse_theme_body(
    raw: &[RawLine],
    idx: usize,
    file: &str,
    source: &str,
) -> Result<Vec<TokenLine>, Vec<Diagnostic>> {
    let base = raw[idx].indent;
    let mut tokens = Vec::new();
    let mut i = idx + 1;
    while i < raw.len() {
        let rl = &raw[i];
        if rl.indent <= base {
            break;
        }
        if rl.indent != base + 2 {
            return Err(vec![Diagnostic::error(
                "P002",
                "theme entries must be indented two spaces under `theme`",
                file,
                rl.line_no,
                1,
                snippet_at(source, rl.line_no, 1),
            )]);
        }
        let words = split_words(&rl.text);
        if words.len() < 3 {
            return Err(vec![Diagnostic::error(
                "U005",
                "theme line must be: <category> <name> <value>",
                file,
                rl.line_no,
                1,
                snippet_at(source, rl.line_no, 1),
            )]);
        }
        tokens.push(TokenLine {
            category: words[0].clone(),
            name: words[1].clone(),
            value: unquote(&words[2..].join(" ")),
            line: rl.line_no,
        });
        i += 1;
    }
    Ok(tokens)
}

fn is_attr_key(word: &str) -> bool {
    matches!(word, "color" | "background" | "button-color" | "button")
}

fn parse_line_tail(
    words: &[String],
    _file: &str,
    _source: &str,
    _line: usize,
) -> Result<(Vec<String>, Vec<(String, String)>), Vec<Diagnostic>> {
    let mut strings = Vec::new();
    let mut attrs = Vec::new();
    let mut i = 0;
    while i < words.len() {
        let w = &words[i];
        if w.starts_with('"') {
            match parse_string_token(w, &words[i..]) {
                Ok((s, consumed)) => {
                    strings.push(s);
                    i += consumed;
                    continue;
                }
                Err(d) => return Err(vec![d]),
            }
        }
        if is_attr_key(w) && i + 1 < words.len() {
            attrs.push((w.clone(), words[i + 1].clone()));
            i += 2;
            continue;
        }
        strings.push(w.clone());
        i += 1;
    }
    Ok((strings, attrs))
}

fn parse_string_token(first: &str, rest: &[String]) -> Result<(String, usize), Diagnostic> {
    if let Some(end) = first.strip_prefix('"') {
        if let Some(inner) = end.strip_suffix('"') {
            return Ok((inner.to_string(), 1));
        }
    }
    let mut combined = first.to_string();
    let mut n = 1usize;
    while !combined.ends_with('"') && n < rest.len() {
        combined.push(' ');
        combined.push_str(&rest[n]);
        n += 1;
    }
    if combined.starts_with('"') && combined.ends_with('"') && combined.len() >= 2 {
        Ok((combined[1..combined.len() - 1].to_string(), n))
    } else {
        Err(Diagnostic::error(
            "P021",
            "unterminated string",
            "",
            0,
            0,
            "",
        ))
    }
}

fn split_words(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut in_str = false;
    for ch in text.chars() {
        if ch == '"' {
            in_str = !in_str;
            cur.push(ch);
            if !in_str && !cur.is_empty() {
                out.push(cur.clone());
                cur.clear();
            }
            continue;
        }
        if !in_str && ch.is_whitespace() {
            if !cur.is_empty() {
                out.push(cur.clone());
                cur.clear();
            }
            continue;
        }
        cur.push(ch);
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

fn unquote(s: &str) -> String {
    let t = s.trim();
    if t.len() >= 2 && t.starts_with('"') && t.ends_with('"') {
        t[1..t.len() - 1].to_string()
    } else {
        t.to_string()
    }
}

fn unknown_part(file: &str, source: &str, line: usize, word: &str) -> Diagnostic {
    Diagnostic::error(
        "U001",
        format!("unknown UI part `{word}`"),
        file,
        line,
        1,
        snippet_at(source, line, 1),
    )
    .with_help(format!("valid parts: {}", valid_parts_list()))
}

fn unknown_use(file: &str, source: &str, line: usize, what: &str) -> Diagnostic {
    Diagnostic::error(
        "U002",
        format!("unknown import `{what}`"),
        file,
        line,
        1,
        snippet_at(source, line, 1),
    )
    .with_help("use `use ui` or `use template <name>`")
}

pub fn attrs_map(attrs: &[(String, String)]) -> HashMap<String, String> {
    attrs.iter().cloned().collect()
}
