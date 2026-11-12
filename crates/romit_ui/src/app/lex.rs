//! Line tokenizer for Romit apps (`use ui` + `app Name`).

#[derive(Debug, Clone, PartialEq)]
pub enum Tok {
    Word(String),
    Str(String),
    Num(String),
    Color(String),
    /// Operators and punctuation: `= += -= == != < > <= >= + - * / % , .`
    Sym(&'static str),
}

impl Tok {
    pub fn word(&self) -> Option<&str> {
        match self {
            Tok::Word(w) => Some(w.as_str()),
            _ => None,
        }
    }
    pub fn is_sym(&self, s: &str) -> bool {
        matches!(self, Tok::Sym(x) if *x == s)
    }
    pub fn describe(&self) -> String {
        match self {
            Tok::Word(w) => format!("`{w}`"),
            Tok::Str(s) => format!("\"{s}\""),
            Tok::Num(n) => n.clone(),
            Tok::Color(c) => c.clone(),
            Tok::Sym(s) => format!("`{s}`"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Line {
    pub indent: usize,
    pub line_no: usize,
    pub toks: Vec<Tok>,
    /// 1-based column of each token (for diagnostics).
    pub cols: Vec<usize>,
    pub children: Vec<Line>,
}

#[derive(Debug, Clone)]
pub struct LexError {
    pub code: &'static str,
    pub message: String,
    pub help: Option<String>,
    pub line: usize,
    pub column: usize,
}

fn err(code: &'static str, message: impl Into<String>, line: usize, column: usize) -> LexError {
    LexError {
        code,
        message: message.into(),
        help: None,
        line,
        column,
    }
}

pub fn tokenize_line(
    text: &str,
    line_no: usize,
    col0: usize,
) -> Result<(Vec<Tok>, Vec<usize>), LexError> {
    let chars: Vec<char> = text.chars().collect();
    let mut toks = Vec::new();
    let mut cols = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let col = col0 + i + 1;
        if c == ' ' {
            i += 1;
            continue;
        }
        if c == '\t' {
            return Err(err(
                "P001",
                "tabs are not allowed; use 2 spaces per indent level",
                line_no,
                col,
            ));
        }
        // comments: `# …`, `// …`
        if (c == '#' && matches!(chars.get(i + 1), None | Some(' ') | Some('#')))
            || (c == '/' && chars.get(i + 1) == Some(&'/'))
        {
            break;
        }
        if c == '"' || c == '\u{201c}' {
            let close = if c == '"' { '"' } else { '\u{201d}' };
            let mut s = String::new();
            i += 1;
            let mut closed = false;
            while i < chars.len() {
                let ch = chars[i];
                if ch == close {
                    closed = true;
                    i += 1;
                    break;
                }
                if ch == '\\' && i + 1 < chars.len() {
                    let n = chars[i + 1];
                    s.push(match n {
                        'n' => '\n',
                        't' => '\t',
                        other => other,
                    });
                    i += 2;
                    continue;
                }
                s.push(ch);
                i += 1;
            }
            if !closed {
                let mut e = err("P011", "unterminated string", line_no, col);
                e.help = Some("close the text with a matching `\"` on the same line".into());
                return Err(e);
            }
            toks.push(Tok::Str(s));
            cols.push(col);
            continue;
        }
        if c == '#' {
            let mut h = String::from("#");
            i += 1;
            while i < chars.len() && chars[i].is_ascii_hexdigit() {
                h.push(chars[i]);
                i += 1;
            }
            toks.push(Tok::Color(h));
            cols.push(col);
            continue;
        }
        if c.is_ascii_digit() {
            let mut n = String::new();
            let mut dot = false;
            while i < chars.len() {
                let ch = chars[i];
                let next_digit = chars.get(i + 1).is_some_and(|d| d.is_ascii_digit());
                if ch.is_ascii_digit() {
                    n.push(ch);
                } else if ch == '_' && next_digit {
                } else if ch == '.' && !dot && next_digit {
                    dot = true;
                    n.push(ch);
                } else {
                    break;
                }
                i += 1;
            }
            toks.push(Tok::Num(n));
            cols.push(col);
            continue;
        }
        if c.is_alphabetic() || c == '_' {
            let mut w = String::new();
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                w.push(chars[i]);
                i += 1;
            }
            toks.push(Tok::Word(w));
            cols.push(col);
            continue;
        }
        let two: String = chars[i..chars.len().min(i + 2)].iter().collect();
        let sym2: Option<&'static str> = match two.as_str() {
            "+=" => Some("+="),
            "-=" => Some("-="),
            "==" => Some("=="),
            "!=" => Some("!="),
            "<=" => Some("<="),
            ">=" => Some(">="),
            _ => None,
        };
        if let Some(s) = sym2 {
            toks.push(Tok::Sym(s));
            cols.push(col);
            i += 2;
            continue;
        }
        let sym1: Option<&'static str> = match c {
            '=' => Some("="),
            '<' => Some("<"),
            '>' => Some(">"),
            '+' => Some("+"),
            '-' => Some("-"),
            '*' => Some("*"),
            '/' => Some("/"),
            '%' => Some("%"),
            ',' => Some(","),
            '.' => Some("."),
            ':' => Some(":"),
            _ => None,
        };
        if let Some(s) = sym1 {
            toks.push(Tok::Sym(s));
            cols.push(col);
            i += 1;
            continue;
        }
        if matches!(c, '(' | ')' | '[' | ']' | '{' | '}') {
            let mut e = err(
                "P013",
                "Romit has no braces, brackets, or parentheses — use indentation and words",
                line_no,
                col,
            );
            e.help = Some(
                "put children on indented lines; use `{name}` only inside text, e.g. \"Hi {name}\""
                    .into(),
            );
            return Err(e);
        }
        if !c.is_ascii() {
            // Emoji and other symbols outside strings: treat as a word so `icon 🔥` works.
            let mut w = String::new();
            while i < chars.len() && chars[i] != ' ' {
                w.push(chars[i]);
                i += 1;
            }
            toks.push(Tok::Str(w));
            cols.push(col);
            continue;
        }
        return Err(err(
            "P010",
            format!("unexpected character {c:?}"),
            line_no,
            col,
        ));
    }
    Ok((toks, cols))
}

/// Split source into an indentation tree of tokenized lines.
pub fn lex_tree(source: &str) -> Result<Vec<Line>, LexError> {
    let mut flat: Vec<Line> = Vec::new();
    for (i, raw) in source.lines().enumerate() {
        let line_no = i + 1;
        let indent = raw.chars().take_while(|c| *c == ' ').count();
        let rest = &raw[indent..];
        if rest.starts_with('\t') {
            return Err(err(
                "P001",
                "tabs are not allowed; use 2 spaces per indent level",
                line_no,
                indent + 1,
            ));
        }
        let trimmed = rest.trim_end();
        if trimmed.is_empty() || trimmed == "note" || trimmed.starts_with("note ") {
            continue;
        }
        if indent % 2 != 0 {
            return Err(err(
                "P002",
                "indent must be a multiple of two spaces",
                line_no,
                indent + 1,
            ));
        }
        let (toks, cols) = tokenize_line(trimmed, line_no, indent)?;
        if toks.is_empty() {
            continue;
        }
        flat.push(Line {
            indent,
            line_no,
            toks,
            cols,
            children: Vec::new(),
        });
    }
    let mut pos = 0;
    build(&flat, &mut pos, 0)
}

fn build(flat: &[Line], pos: &mut usize, indent: usize) -> Result<Vec<Line>, LexError> {
    let mut out: Vec<Line> = Vec::new();
    while *pos < flat.len() {
        let l = &flat[*pos];
        if l.indent < indent {
            break;
        }
        if l.indent > indent {
            if out.is_empty() || l.indent != indent + 2 {
                return Err(err(
                    "P002",
                    "expected indent two spaces deeper than the line above",
                    l.line_no,
                    l.indent + 1,
                ));
            }
            let kids = build(flat, pos, indent + 2)?;
            out.last_mut().unwrap().children = kids;
            continue;
        }
        out.push(l.clone());
        *pos += 1;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_and_tree() {
        let t = lex_tree("app A\n  state\n    n is 0 # count\n").unwrap();
        assert_eq!(t.len(), 1);
        assert_eq!(t[0].children[0].children[0].toks.len(), 3);
    }

    #[test]
    fn rejects_parens() {
        assert_eq!(lex_tree("text (1)\n").unwrap_err().code, "P013");
    }
}
