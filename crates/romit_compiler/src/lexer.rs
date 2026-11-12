use crate::diag::{snippet_at, Diagnostic};

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Newline,
    Indent,
    Dedent,
    Eof,
    Ident(String),
    Int(i64),
    Decimal(String),
    String(String),
    Color(String),
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Dot,
    Assign,
    PlusAssign,
    MinusAssign,
    Comma,
    Colon,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub line: usize,
    pub column: usize,
}

pub struct Lexer {
    source: String,
    chars: Vec<char>,
    pos: usize,
    line: usize,
    column: usize,
    indent_stack: Vec<usize>,
    pending_dedents: usize,
}

impl Lexer {
    pub fn new(source: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            chars: Vec::new(),
            pos: 0,
            line: 1,
            column: 1,
            indent_stack: vec![0],
            pending_dedents: 0,
        }
    }

    /// `# comment` — a hash followed by a space, another hash, or end of line.
    /// `#fff` stays a color literal.
    fn at_comment_start(&self) -> bool {
        self.peek() == Some('#')
            && matches!(
                self.chars.get(self.pos + 1),
                None | Some(' ') | Some('#') | Some('\n') | Some('!')
            )
    }

    /// `note anything` at the start of a line is a comment (same as Romit UI files).
    fn at_note_line(&self) -> bool {
        let rest: String = self.chars[self.pos..].iter().take(5).collect();
        rest == "note " || rest == "note\n" || (rest == "note" && self.pos + 4 == self.chars.len())
    }

    fn ensure_chars(&mut self) {
        if self.chars.is_empty() {
            self.chars = self.source.chars().collect();
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.chars.get(self.pos).copied()?;
        self.pos += 1;
        if c == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        Some(c)
    }

    pub fn tokenize(&mut self, file: &str) -> Result<Vec<Token>, Diagnostic> {
        self.ensure_chars();
        let mut tokens = Vec::new();
        let mut at_line_start = true;
        loop {
            if at_line_start {
                let line = self.line;
                let col = self.column;
                let mut indent = 0usize;
                while self.peek() == Some(' ') {
                    indent += 1;
                    self.bump();
                }
                if self.peek() == Some('\t') {
                    return Err(brace_err(
                        file,
                        &self.source,
                        line,
                        col,
                        "P001",
                        "tabs are not allowed; use 2 spaces per indent level",
                    ));
                }
                if indent % 2 != 0 {
                    return Err(brace_err(
                        file,
                        &self.source,
                        line,
                        col,
                        "P002",
                        "indentation must use multiples of 2 spaces",
                    ));
                }
                let top = *self.indent_stack.last().unwrap_or(&0);
                if self.at_comment_start() || self.at_note_line() {
                    while matches!(self.peek(), Some(ch) if ch != '\n') {
                        self.bump();
                    }
                }
                if matches!(self.peek(), Some('\n') | None) {
                    // blank or comment-only line
                } else if indent > top {
                    self.indent_stack.push(indent);
                    tokens.push(Token {
                        kind: TokenKind::Indent,
                        line,
                        column: col,
                    });
                } else if indent < top {
                    while self.indent_stack.last().copied().unwrap_or(0) > indent {
                        if self.indent_stack.len() <= 1 {
                            return Err(brace_err(
                                file,
                                &self.source,
                                line,
                                col,
                                "P003",
                                "dedent goes past the outermost block",
                            ));
                        }
                        self.indent_stack.pop();
                        tokens.push(Token {
                            kind: TokenKind::Dedent,
                            line,
                            column: col,
                        });
                    }
                    if *self.indent_stack.last().unwrap_or(&0) != indent {
                        return Err(brace_err(
                            file,
                            &self.source,
                            line,
                            col,
                            "P004",
                            "indentation does not match any enclosing block",
                        ));
                    }
                }
                at_line_start = false;

                if matches!(self.peek(), Some('\n') | None) {
                    if self.peek() == Some('\n') {
                        self.bump();
                        tokens.push(Token {
                            kind: TokenKind::Newline,
                            line,
                            column: col,
                        });
                    }
                    at_line_start = true;
                    if self.peek().is_none() {
                        flush_eof(
                            &mut tokens,
                            &mut self.pending_dedents,
                            &mut self.indent_stack,
                            self.line,
                            self.column,
                        );
                        tokens.push(Token {
                            kind: TokenKind::Eof,
                            line: self.line,
                            column: self.column,
                        });
                        break;
                    }
                    continue;
                }
            }

            while matches!(self.peek(), Some(' ') | Some('\t')) {
                self.bump();
            }

            let line = self.line;
            let col = self.column;

            let Some(c) = self.peek() else {
                flush_eof(
                    &mut tokens,
                    &mut self.pending_dedents,
                    &mut self.indent_stack,
                    line,
                    col,
                );
                tokens.push(Token {
                    kind: TokenKind::Eof,
                    line,
                    column: col,
                });
                break;
            };

            if c == '\n' {
                self.bump();
                tokens.push(Token {
                    kind: TokenKind::Newline,
                    line,
                    column: col,
                });
                at_line_start = true;
                continue;
            }

            if (c == '/' && self.chars.get(self.pos + 1) == Some(&'/')) || self.at_comment_start() {
                while matches!(self.peek(), Some(ch) if ch != '\n') {
                    self.bump();
                }
                continue;
            }

            if matches!(c, '(' | ')' | '[' | ']' | '{' | '}') {
                return Err(brace_err(
                    file,
                    &self.source,
                    line,
                    col,
                    "P013",
                    "Romit has no braces, brackets, or parentheses — use indentation for blocks",
                ));
            }

            if c == '"'
                && self.chars.get(self.pos + 1) == Some(&'"')
                && self.chars.get(self.pos + 2) == Some(&'"')
            {
                // multi-line text: """ … """ (common indentation is removed)
                for _ in 0..3 {
                    self.bump();
                }
                let mut raw = String::new();
                let mut closed = false;
                while self.peek().is_some() {
                    if self.peek() == Some('"')
                        && self.chars.get(self.pos + 1) == Some(&'"')
                        && self.chars.get(self.pos + 2) == Some(&'"')
                    {
                        for _ in 0..3 {
                            self.bump();
                        }
                        closed = true;
                        break;
                    }
                    raw.push(self.bump().unwrap());
                }
                if !closed {
                    return Err(brace_err(
                        file,
                        &self.source,
                        line,
                        col,
                        "P011",
                        "multi-line text needs a closing \"\"\"",
                    ));
                }
                tokens.push(Token {
                    kind: TokenKind::String(dedent_block(&raw)),
                    line,
                    column: col,
                });
                continue;
            }

            if c == '"' {
                self.bump();
                let mut s = String::new();
                // quotes inside a `{…}` hole belong to the hole: "Hi {name or else "friend"}"
                let mut depth = 0usize;
                let mut closed = false;
                while let Some(ch) = self.peek() {
                    if ch == '{' {
                        if self.chars.get(self.pos + 1) == Some(&'{') && depth == 0 {
                            s.push_str("{{");
                            self.bump();
                            self.bump();
                            continue;
                        }
                        depth += 1;
                    } else if ch == '}' && depth > 0 {
                        depth -= 1;
                    }
                    if ch == '"' && depth == 0 {
                        self.bump();
                        closed = true;
                        break;
                    }
                    if ch == '\n' {
                        break;
                    }
                    if ch == '\\' && depth == 0 {
                        // escapes: \n new line, \t tab, \" quote, \\ backslash
                        if let Some(&n) = self.chars.get(self.pos + 1) {
                            let mapped = match n {
                                'n' => Some('\n'),
                                't' => Some('\t'),
                                '"' => Some('"'),
                                '\\' => Some('\\'),
                                _ => None,
                            };
                            if let Some(m) = mapped {
                                s.push(m);
                                self.bump();
                                self.bump();
                                continue;
                            }
                        }
                    }
                    s.push(ch);
                    self.bump();
                }
                if !closed {
                    return Err(brace_err(
                        file,
                        &self.source,
                        line,
                        col,
                        "P011",
                        "unterminated string literal",
                    )
                    .with_help("close the text with `\"` on the same line, or use \"\"\" … \"\"\" for several lines"));
                }
                tokens.push(Token {
                    kind: TokenKind::String(s),
                    line,
                    column: col,
                });
                continue;
            }

            if c == '#' {
                self.bump();
                let mut h = String::from("#");
                while matches!(self.peek(), Some(ch) if ch.is_ascii_hexdigit()) {
                    h.push(self.bump().unwrap());
                }
                tokens.push(Token {
                    kind: TokenKind::Color(h),
                    line,
                    column: col,
                });
                continue;
            }

            if c.is_ascii_digit() {
                let mut num = String::new();
                let mut is_dec = false;
                while let Some(ch) = self.peek() {
                    let next_is_digit = self
                        .chars
                        .get(self.pos + 1)
                        .is_some_and(|d| d.is_ascii_digit());
                    if ch.is_ascii_digit() || (ch == '_' && next_is_digit) {
                        self.bump();
                        if ch != '_' {
                            num.push(ch);
                        }
                    } else if ch == '.' && !is_dec && next_is_digit {
                        self.bump();
                        is_dec = true;
                        num.push(ch);
                    } else {
                        break;
                    }
                }
                tokens.push(Token {
                    kind: if is_dec {
                        TokenKind::Decimal(num)
                    } else {
                        TokenKind::Int(num.parse().unwrap_or(0))
                    },
                    line,
                    column: col,
                });
                continue;
            }

            if c.is_ascii_alphabetic() || c == '_' {
                let mut w = String::new();
                while matches!(self.peek(), Some(ch) if ch.is_ascii_alphanumeric() || ch == '_' || ch == '/')
                {
                    w.push(self.bump().unwrap());
                }
                tokens.push(Token {
                    kind: TokenKind::Ident(w),
                    line,
                    column: col,
                });
                continue;
            }

            let op = match c {
                '=' => {
                    self.bump();
                    if self.peek() == Some('=') {
                        self.bump();
                        TokenKind::Eq
                    } else if self.peek() == Some('>') {
                        self.bump();
                        TokenKind::Ge
                    } else {
                        TokenKind::Assign
                    }
                }
                '!' => {
                    self.bump();
                    if self.peek() == Some('=') {
                        self.bump();
                        TokenKind::Ne
                    } else {
                        return Err(brace_err(
                            file,
                            &self.source,
                            line,
                            col,
                            "P010",
                            "unexpected !",
                        ));
                    }
                }
                '<' => {
                    self.bump();
                    if self.peek() == Some('=') {
                        self.bump();
                        TokenKind::Le
                    } else {
                        TokenKind::Lt
                    }
                }
                '>' => {
                    self.bump();
                    if self.peek() == Some('=') {
                        self.bump();
                        TokenKind::Ge
                    } else {
                        TokenKind::Gt
                    }
                }
                '+' => {
                    self.bump();
                    if self.peek() == Some('=') {
                        self.bump();
                        TokenKind::PlusAssign
                    } else {
                        TokenKind::Plus
                    }
                }
                '-' => {
                    self.bump();
                    if self.peek() == Some('=') {
                        self.bump();
                        TokenKind::MinusAssign
                    } else {
                        TokenKind::Minus
                    }
                }
                '*' => {
                    self.bump();
                    TokenKind::Star
                }
                '/' => {
                    self.bump();
                    TokenKind::Slash
                }
                '%' => {
                    self.bump();
                    TokenKind::Percent
                }
                '.' => {
                    self.bump();
                    TokenKind::Dot
                }
                ',' => {
                    self.bump();
                    TokenKind::Comma
                }
                ':' => {
                    self.bump();
                    TokenKind::Colon
                }
                _ => {
                    return Err(brace_err(
                        file,
                        &self.source,
                        line,
                        col,
                        "P010",
                        format!("unexpected character {:?}", c),
                    ));
                }
            };
            tokens.push(Token {
                kind: op,
                line,
                column: col,
            });
        }
        let _ = file;
        Ok(tokens)
    }
}

fn flush_eof(
    tokens: &mut Vec<Token>,
    pending: &mut usize,
    stack: &mut Vec<usize>,
    line: usize,
    col: usize,
) {
    while stack.len() > 1 {
        stack.pop();
        *pending += 1;
    }
    while *pending > 0 {
        *pending -= 1;
        tokens.push(Token {
            kind: TokenKind::Dedent,
            line,
            column: col,
        });
    }
}

fn brace_err(
    file: &str,
    source: &str,
    line: usize,
    col: usize,
    code: &str,
    message: impl Into<String>,
) -> Diagnostic {
    Diagnostic::error(
        code,
        message,
        file,
        line,
        col,
        snippet_at(source, line, col, "here"),
    )
}

/// Text between """ marks: drop the first and last blank lines and the shared indentation.
fn dedent_block(raw: &str) -> String {
    let mut lines: Vec<&str> = raw.split('\n').collect();
    if lines.first().is_some_and(|l| l.trim().is_empty()) {
        lines.remove(0);
    }
    if lines.last().is_some_and(|l| l.trim().is_empty()) {
        lines.pop();
    }
    let indent = lines
        .iter()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.len() - l.trim_start_matches(' ').len())
        .min()
        .unwrap_or(0);
    lines
        .iter()
        .map(|l| {
            if l.len() >= indent {
                &l[indent..]
            } else {
                l.trim_start()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}
