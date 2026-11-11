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
    Dot,
    Assign,
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
                if matches!(self.peek(), Some('#') | Some('\n') | None) {
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

            if c == '/' && self.chars.get(self.pos + 1) == Some(&'/') {
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
                    "Romit has no braces. Use indentation instead.",
                ));
            }

            if c == ':' {
                return Err(brace_err(
                    file,
                    &self.source,
                    line,
                    col,
                    "P013",
                    "Romit has no colons in code. Put nested lines underneath with indentation.",
                ));
            }

            if c == '"' {
                self.bump();
                let mut s = String::new();
                while let Some(ch) = self.peek() {
                    if ch == '"' {
                        self.bump();
                        break;
                    }
                    if ch == '\n' {
                        return Err(brace_err(
                            file,
                            &self.source,
                            line,
                            col,
                            "P011",
                            "unterminated string literal",
                        ));
                    }
                    s.push(ch);
                    self.bump();
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
                while matches!(self.peek(), Some(ch) if ch.is_ascii_digit() || ch == '.') {
                    let ch = self.bump().unwrap();
                    if ch == '.' {
                        is_dec = true;
                    }
                    num.push(ch);
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
                    TokenKind::Gt
                }
                '+' => {
                    self.bump();
                    TokenKind::Plus
                }
                '-' => {
                    self.bump();
                    TokenKind::Minus
                }
                '*' => {
                    self.bump();
                    TokenKind::Star
                }
                '/' => {
                    self.bump();
                    TokenKind::Slash
                }
                '.' => {
                    self.bump();
                    TokenKind::Dot
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
