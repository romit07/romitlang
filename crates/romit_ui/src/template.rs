use crate::diag::{snippet_at, Diagnostic};
use crate::parse::{TokenLine, UiNode};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenSet {
    pub values: HashMap<String, String>,
}

pub const VALID_TOKEN_KEYS: &[&str] = &[
    "color-primary",
    "color-text",
    "color-muted",
    "color-background",
    "color-accent",
    "color-button",
    "color-button-text",
    "space-unit",
    "space-section",
    "space-gap",
    "font-family",
    "font-size-base",
    "font-size-title",
    "font-size-hero",
    "radius-button",
    "radius-card",
    "max-width",
];

pub fn valid_tokens_list() -> String {
    VALID_TOKEN_KEYS.join(", ")
}

impl TokenSet {
    pub fn builtin_default() -> Self {
        let mut values = HashMap::new();
        values.insert("color-primary".into(), "#0a7a62".into());
        values.insert("color-text".into(), "#1a1a1a".into());
        values.insert("color-muted".into(), "#5c5c5c".into());
        values.insert("color-background".into(), "#fafafa".into());
        values.insert("color-accent".into(), "#0a7a62".into());
        values.insert("color-button".into(), "#0a7a62".into());
        values.insert("color-button-text".into(), "#ffffff".into());
        values.insert("space-unit".into(), "8px".into());
        values.insert("space-section".into(), "48px".into());
        values.insert("space-gap".into(), "16px".into());
        values.insert("font-family".into(), "system-ui, sans-serif".into());
        values.insert("font-size-base".into(), "1rem".into());
        values.insert("font-size-title".into(), "1.75rem".into());
        values.insert("font-size-hero".into(), "2.5rem".into());
        values.insert("radius-button".into(), "8px".into());
        values.insert("radius-card".into(), "12px".into());
        values.insert("max-width".into(), "960px".into());
        Self { values }
    }

    pub fn merge(&mut self, other: &TokenSet) {
        for (k, v) in &other.values {
            self.values.insert(k.clone(), v.clone());
        }
    }

    pub fn apply_theme_lines(
        &mut self,
        lines: &[TokenLine],
        file: &str,
        source: &str,
    ) -> Result<(), Vec<Diagnostic>> {
        for tl in lines {
            let key = token_key(&tl.category, &tl.name);
            if !VALID_TOKEN_KEYS.contains(&key.as_str()) {
                return Err(vec![Diagnostic::error(
                    "U006",
                    format!("unknown theme token `{}/{}`", tl.category, tl.name),
                    file,
                    tl.line,
                    1,
                    snippet_at(source, tl.line, 1),
                )
                .with_help(format!("valid tokens: {}", valid_tokens_list()))]);
            }
            let value = resolve_color_value(&key, &tl.value, file, source, tl.line)?;
            self.values.insert(key, value);
        }
        Ok(())
    }

    pub fn apply_element_attr(&self, base: &TokenSet, attrs: &[(String, String)]) -> TokenSet {
        let mut out = base.clone();
        for (k, v) in attrs {
            match k.as_str() {
                "color" => {
                    if let Some(resolved) = named_color(v) {
                        out.values.insert("color-text".into(), resolved);
                    } else {
                        out.values.insert("color-text".into(), v.clone());
                    }
                }
                "background" => {
                    out.values.insert(
                        "color-background".into(),
                        named_color(v).unwrap_or_else(|| v.clone()),
                    );
                }
                "button-color" | "button" => {
                    out.values.insert(
                        "color-button".into(),
                        named_color(v).unwrap_or_else(|| v.clone()),
                    );
                }
                _ => {}
            }
        }
        out
    }
}

pub fn token_key(category: &str, name: &str) -> String {
    format!("{}-{}", category, name)
}

pub fn resolve_for_page(
    template_name: &str,
    base_dir: Option<&Path>,
    page: &UiNode,
    page_source: &str,
    file: &str,
) -> Result<TokenSet, Vec<Diagnostic>> {
    let mut tokens = TokenSet::builtin_default();

    if let Some(dir) = base_dir {
        let path = dir.join("templates").join(format!("{template_name}.rmt"));
        if path.exists() {
            let src = std::fs::read_to_string(&path).map_err(|e| {
                vec![Diagnostic::error(
                    "U007",
                    format!("cannot read template {}: {e}", path.display()),
                    file,
                    1,
                    1,
                    "",
                )]
            })?;
            let loaded = parse_template_file(&src, path.to_string_lossy().as_ref())?;
            tokens.merge(&loaded);
        }
    }

    let page_theme: Vec<TokenLine> = page
        .children
        .iter()
        .filter(|c| c.kind == "theme")
        .flat_map(|c| c.theme_tokens.clone())
        .collect();
    tokens.apply_theme_lines(&page_theme, file, page_source)?;

    Ok(tokens)
}

fn parse_template_file(src: &str, template_path: &str) -> Result<TokenSet, Vec<Diagnostic>> {
    let mut set = TokenSet {
        values: HashMap::new(),
    };
    let lines: Vec<_> = src
        .lines()
        .enumerate()
        .filter(|(_, l)| {
            let t = l.trim();
            !t.is_empty() && !t.starts_with("note ")
        })
        .collect();
    if lines.is_empty() {
        return Ok(set);
    }
    let first = lines[0].1.trim();
    let words: Vec<&str> = first.split_whitespace().collect();
    if words.len() >= 2 && words[0] == "template" {
        // header ok
    } else {
        return Err(vec![Diagnostic::error(
            "U007",
            "template file must start with `template <name>`",
            template_path,
            1,
            1,
            snippet_at(src, 1, 1),
        )]);
    }
    for (i, line) in lines.iter().skip(1) {
        let line_no = i + 1;
        let indent = line.len() - line.trim_start().len();
        if indent < 2 {
            continue;
        }
        let words = line.trim().split_whitespace().collect::<Vec<_>>();
        if words.len() < 3 {
            continue;
        }
        let tl = TokenLine {
            category: words[0].to_string(),
            name: words[1].to_string(),
            value: words[2..].join(" ").trim_matches('"').to_string(),
            line: line_no,
        };
        set.apply_theme_lines(&[tl], template_path, src)?;
    }
    Ok(set)
}

fn resolve_color_value(
    key: &str,
    value: &str,
    file: &str,
    source: &str,
    line: usize,
) -> Result<String, Vec<Diagnostic>> {
    let v = value.trim();
    if v.starts_with('#') || v.chars().all(|c| c.is_ascii_hexdigit() || c == '#') {
        return Ok(v.to_string());
    }
    if let Some(hex) = named_color(v) {
        return Ok(hex);
    }
    if key.starts_with("color-")
        || key.starts_with("font-")
        || key.contains("radius")
        || key.contains("space")
        || key == "max-width"
    {
        // allow raw css values like 1rem, 48px
        if v.contains("px") || v.contains("rem") || v.contains('%') || v.contains(',') {
            return Ok(v.to_string());
        }
    }
    Err(vec![Diagnostic::error(
        "U006",
        format!("unknown color or value `{v}`"),
        file,
        line,
        1,
        snippet_at(source, line, 1),
    )
    .with_help(format!("valid tokens: {}", valid_tokens_list()))])
}

pub fn named_color(name: &str) -> Option<String> {
    match name.to_lowercase().as_str() {
        "blue" => Some("#2563eb".into()),
        "red" => Some("#dc2626".into()),
        "green" => Some("#16a34a".into()),
        "white" => Some("#ffffff".into()),
        "black" => Some("#000000".into()),
        "gray" | "grey" => Some("#6b7280".into()),
        _ => None,
    }
}
