//! Expressions inside Romit apps: `count of todos where not done`, `"Hi {name}"`, `price as money`.

use super::lex::{tokenize_line, Tok};

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Num(f64),
    /// Text with `{expr}` holes.
    Text(Vec<TextPart>),
    Bool(bool),
    Nothing,
    Color(String),
    Name(String),
    Member(Box<Expr>, String),
    Bin(Op, Box<Expr>, Box<Expr>),
    Not(Box<Expr>),
    Neg(Box<Expr>),
    /// `count of x`, `upper of name`
    Of(String, Box<Expr>),
    /// `todos where done`
    Where(Box<Expr>, Box<Expr>),
    /// `todos sorted by title`, `names sorted`
    SortBy(Box<Expr>, Option<String>, bool),
    /// `price as money`
    As(Box<Expr>, String),
    If(Box<Expr>, Box<Expr>, Box<Expr>),
    List(Vec<Expr>),
    Record(Vec<(String, Expr)>),
    /// `random 1 to 6`
    Random(Box<Expr>, Box<Expr>),
    /// `item 2 of names`
    Item(Box<Expr>, Box<Expr>),
    IsEmpty(Box<Expr>, bool),
}

#[derive(Debug, Clone, PartialEq)]
pub enum TextPart {
    Lit(String),
    Hole(Expr),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Op {
    Or,
    And,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    In,
}

pub const OF_FUNCS: &[&str] = &[
    "count",
    "size",
    "length",
    "sum",
    "total",
    "average",
    "min",
    "max",
    "first",
    "last",
    "upper",
    "uppercase",
    "lower",
    "lowercase",
    "trim",
    "round",
    "floor",
    "ceil",
    "abs",
    "sqrt",
    "reverse",
    "reversed",
    "sorted",
    "unique",
    "number",
    "text",
    "words",
    "lines",
    "shuffled",
    "keys",
];

pub const AS_FORMATS: &[&str] = &[
    "money", "dollars", "euros", "percent", "number", "date", "time", "datetime", "text", "whole",
    "decimal", "ago",
];

/// Words that continue an expression when they appear after a value.
pub fn is_operator_word(w: &str) -> bool {
    matches!(
        w,
        "plus"
            | "minus"
            | "times"
            | "divided"
            | "remainder"
            | "mod"
            | "is"
            | "isnt"
            | "and"
            | "or"
            | "contains"
            | "has"
            | "in"
            | "above"
            | "below"
            | "where"
            | "sorted"
            | "as"
            | "equals"
    )
}

/// Words that can never be a value name.
pub fn is_reserved_word(w: &str) -> bool {
    is_operator_word(w)
        || matches!(
            w,
            "not"
                | "if"
                | "then"
                | "else"
                | "otherwise"
                | "true"
                | "false"
                | "yes"
                | "no"
                | "nothing"
                | "list"
                | "with"
                | "random"
                | "of"
                | "do"
                | "to"
                | "by"
                | "each"
                | "for"
                | "on"
                | "bind"
        )
}

#[derive(Debug, Clone)]
pub struct ExprError {
    pub message: String,
    pub help: Option<String>,
    pub tok: usize,
}

pub struct ExprParser<'a> {
    pub toks: &'a [Tok],
    pub pos: usize,
    /// Words that end the expression when seen where a new value or operator would start.
    pub stop: &'a dyn Fn(&str) -> bool,
    /// Line number, used to re-lex `{…}` holes.
    pub line_no: usize,
    /// When true, `,` ends the expression (record fields, argument lists).
    pub comma_stops: bool,
    /// Inside a `where` condition: a later `where` / `sorted` belongs to the outer list.
    in_where: usize,
}

type R<T> = Result<T, ExprError>;

impl<'a> ExprParser<'a> {
    pub fn new(
        toks: &'a [Tok],
        pos: usize,
        stop: &'a dyn Fn(&str) -> bool,
        line_no: usize,
    ) -> Self {
        ExprParser {
            toks,
            pos,
            stop,
            line_no,
            comma_stops: true,
            in_where: 0,
        }
    }

    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.pos)
    }
    fn peek_at(&self, n: usize) -> Option<&Tok> {
        self.toks.get(self.pos + n)
    }
    fn peek_word(&self) -> Option<&str> {
        self.peek().and_then(|t| t.word())
    }
    fn err<T>(&self, msg: impl Into<String>) -> R<T> {
        Err(ExprError {
            message: msg.into(),
            help: None,
            tok: self.pos,
        })
    }
    fn eat_word(&mut self, w: &str) -> bool {
        if self.peek_word() == Some(w) {
            self.pos += 1;
            true
        } else {
            false
        }
    }
    fn eat_sym(&mut self, s: &str) -> bool {
        if self.peek().is_some_and(|t| t.is_sym(s)) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    pub fn at_end(&self) -> bool {
        match self.peek() {
            None => true,
            Some(Tok::Sym(",")) => self.comma_stops,
            Some(Tok::Word(w)) => (self.stop)(w) && !is_operator_word(w),
            _ => false,
        }
    }

    /// True when the next token could begin a new value (used for juxtaposed text parts).
    pub fn starts_value(&self) -> bool {
        match self.peek() {
            None => false,
            Some(Tok::Word(w)) => {
                !(self.stop)(w)
                    && !is_operator_word(w)
                    && !matches!(w.as_str(), "then" | "else" | "to" | "by" | "of" | "do")
            }
            Some(Tok::Sym("-")) => false,
            Some(Tok::Sym(_)) => false,
            _ => true,
        }
    }

    pub fn parse(&mut self) -> R<Expr> {
        if self.at_end() {
            return self.err("expected a value");
        }
        self.parse_or()
    }

    fn parse_or(&mut self) -> R<Expr> {
        let mut e = self.parse_and()?;
        while self.peek_word() == Some("or") {
            self.pos += 1;
            let r = self.parse_and()?;
            e = Expr::Bin(Op::Or, Box::new(e), Box::new(r));
        }
        Ok(e)
    }

    fn parse_and(&mut self) -> R<Expr> {
        let mut e = self.parse_not()?;
        while self.peek_word() == Some("and") {
            self.pos += 1;
            let r = self.parse_not()?;
            e = Expr::Bin(Op::And, Box::new(e), Box::new(r));
        }
        Ok(e)
    }

    fn parse_not(&mut self) -> R<Expr> {
        if self.eat_word("not") {
            let e = self.parse_not()?;
            return Ok(Expr::Not(Box::new(e)));
        }
        self.parse_cmp()
    }

    fn parse_cmp(&mut self) -> R<Expr> {
        let mut e = self.parse_add()?;
        loop {
            let op = match self.peek() {
                Some(Tok::Word(w)) if w == "is" || w == "isnt" => {
                    let neg_word = w == "isnt";
                    self.pos += 1;
                    let negate = neg_word || self.eat_word("not");
                    if self.eat_word("empty") {
                        e = Expr::IsEmpty(Box::new(e), !negate);
                        continue;
                    }
                    if self.eat_word("in") {
                        let r = self.parse_add()?;
                        let inn = Expr::Bin(Op::In, Box::new(e), Box::new(r));
                        e = if negate {
                            Expr::Not(Box::new(inn))
                        } else {
                            inn
                        };
                        continue;
                    }
                    let op = if self.eat_word("above") || self.eat_word("over") {
                        Op::Gt
                    } else if self.eat_word("below") || self.eat_word("under") {
                        Op::Lt
                    } else if self.eat_word("more") || self.eat_word("greater") {
                        self.eat_word("than");
                        Op::Gt
                    } else if self.eat_word("less") || self.eat_word("fewer") {
                        self.eat_word("than");
                        Op::Lt
                    } else if self.peek_word() == Some("at") {
                        self.pos += 1;
                        if self.eat_word("least") {
                            Op::Ge
                        } else if self.eat_word("most") {
                            Op::Le
                        } else {
                            return self.err("expected `least` or `most` after `is at`");
                        }
                    } else {
                        Op::Eq
                    };
                    let op = if negate {
                        match op {
                            Op::Eq => Op::Ne,
                            Op::Gt => Op::Le,
                            Op::Lt => Op::Ge,
                            Op::Ge => Op::Lt,
                            Op::Le => Op::Gt,
                            o => o,
                        }
                    } else {
                        op
                    };
                    let r = self.parse_add()?;
                    e = Expr::Bin(op, Box::new(e), Box::new(r));
                    continue;
                }
                Some(Tok::Word(w)) if w == "equals" => Op::Eq,
                Some(Tok::Word(w)) if w == "above" => Op::Gt,
                Some(Tok::Word(w)) if w == "below" => Op::Lt,
                Some(Tok::Word(w)) if w == "contains" || w == "has" => {
                    self.pos += 1;
                    let r = self.parse_add()?;
                    e = Expr::Bin(Op::In, Box::new(r), Box::new(e));
                    continue;
                }
                Some(Tok::Word(w)) if w == "in" => Op::In,
                Some(Tok::Sym("==")) => Op::Eq,
                Some(Tok::Sym("!=")) => Op::Ne,
                Some(Tok::Sym("<")) => Op::Lt,
                Some(Tok::Sym("<=")) => Op::Le,
                Some(Tok::Sym(">")) => Op::Gt,
                Some(Tok::Sym(">=")) => Op::Ge,
                Some(Tok::Sym("=")) => {
                    let mut er: R<Expr> =
                        self.err("use `is` or `==` to compare; `=` changes a value");
                    if let Err(ref mut x) = er {
                        x.help = Some("in conditions write `if count is 3`".into());
                    }
                    return er;
                }
                _ => break,
            };
            self.pos += 1;
            let r = self.parse_add()?;
            e = Expr::Bin(op, Box::new(e), Box::new(r));
        }
        Ok(e)
    }

    pub fn parse_sum(&mut self) -> R<Expr> {
        self.parse_add()
    }

    fn parse_add(&mut self) -> R<Expr> {
        let mut e = self.parse_mul()?;
        loop {
            let op = match self.peek() {
                Some(Tok::Sym("+")) => Op::Add,
                Some(Tok::Sym("-")) => Op::Sub,
                Some(Tok::Word(w)) if w == "plus" => Op::Add,
                Some(Tok::Word(w)) if w == "minus" => Op::Sub,
                _ => break,
            };
            self.pos += 1;
            let r = self.parse_mul()?;
            e = Expr::Bin(op, Box::new(e), Box::new(r));
        }
        Ok(e)
    }

    fn parse_mul(&mut self) -> R<Expr> {
        let mut e = self.parse_unary()?;
        loop {
            let op = match self.peek() {
                Some(Tok::Sym("*")) => Op::Mul,
                Some(Tok::Sym("/")) => Op::Div,
                Some(Tok::Sym("%")) => Op::Mod,
                Some(Tok::Word(w)) if w == "times" => Op::Mul,
                Some(Tok::Word(w)) if w == "remainder" || w == "mod" => Op::Mod,
                Some(Tok::Word(w)) if w == "divided" => {
                    self.pos += 1;
                    if !self.eat_word("by") {
                        return self.err("expected `by` after `divided`");
                    }
                    let r = self.parse_unary()?;
                    e = Expr::Bin(Op::Div, Box::new(e), Box::new(r));
                    continue;
                }
                _ => break,
            };
            self.pos += 1;
            let r = self.parse_unary()?;
            e = Expr::Bin(op, Box::new(e), Box::new(r));
        }
        Ok(e)
    }

    fn parse_unary(&mut self) -> R<Expr> {
        if self.eat_sym("-") {
            let e = self.parse_unary()?;
            return Ok(Expr::Neg(Box::new(e)));
        }
        self.parse_postfix()
    }

    fn parse_postfix(&mut self) -> R<Expr> {
        let mut e = self.parse_primary()?;
        loop {
            if self.peek().is_some_and(|t| t.is_sym(".")) {
                if let Some(Tok::Word(f)) = self.peek_at(1) {
                    let f = f.clone();
                    self.pos += 2;
                    e = Expr::Member(Box::new(e), f);
                    continue;
                }
                return self.err("expected a field name after `.`");
            }
            match self.peek_word() {
                Some("where") | Some("sorted") if self.in_where > 0 => break,
                Some("where") => {
                    self.pos += 1;
                    self.in_where += 1;
                    let cond = self.parse_or();
                    self.in_where -= 1;
                    e = Expr::Where(Box::new(e), Box::new(cond?));
                }
                Some("sorted") => {
                    self.pos += 1;
                    let mut field = None;
                    if self.eat_word("by") {
                        match self.peek() {
                            Some(Tok::Word(f)) => {
                                field = Some(f.clone());
                                self.pos += 1;
                            }
                            _ => return self.err("expected a field name after `sorted by`"),
                        }
                    }
                    let desc = self.eat_word("descending") || self.eat_word("reversed");
                    self.eat_word("ascending");
                    e = Expr::SortBy(Box::new(e), field, desc);
                }
                Some("as") => {
                    self.pos += 1;
                    match self.peek_word() {
                        Some(f) if AS_FORMATS.contains(&f) => {
                            let f = f.to_string();
                            self.pos += 1;
                            e = Expr::As(Box::new(e), f);
                        }
                        _ => {
                            let mut er: R<Expr> = self.err("unknown format after `as`");
                            if let Err(ref mut x) = er {
                                x.help = Some(format!("formats: {}", AS_FORMATS.join(", ")));
                            }
                            return er;
                        }
                    }
                }
                _ => break,
            }
        }
        Ok(e)
    }

    fn parse_primary(&mut self) -> R<Expr> {
        let Some(t) = self.peek().cloned() else {
            return self.err("expected a value");
        };
        match t {
            Tok::Num(n) => {
                self.pos += 1;
                Ok(Expr::Num(n.parse().unwrap_or(0.0)))
            }
            Tok::Str(s) => {
                self.pos += 1;
                Ok(Expr::Text(parse_text(&s, self.line_no)?))
            }
            Tok::Color(c) => {
                self.pos += 1;
                Ok(Expr::Color(c))
            }
            Tok::Sym(s) => self.err(format!("unexpected `{s}`")),
            Tok::Word(w) => {
                if (self.stop)(&w) {
                    return self.err(format!("expected a value before `{w}`"));
                }
                match w.as_str() {
                    "true" | "yes" | "on" => {
                        self.pos += 1;
                        Ok(Expr::Bool(true))
                    }
                    "false" | "no" | "off" => {
                        self.pos += 1;
                        Ok(Expr::Bool(false))
                    }
                    "nothing" | "none" => {
                        self.pos += 1;
                        Ok(Expr::Nothing)
                    }
                    "if" => {
                        self.pos += 1;
                        let c = self.parse_or()?;
                        if !self.eat_word("then") {
                            return self.err("expected `then` in `if … then … else …`");
                        }
                        let a = self.parse_or()?;
                        if !(self.eat_word("else") || self.eat_word("otherwise")) {
                            return self.err("expected `else` in `if … then … else …`");
                        }
                        let b = self.parse_or()?;
                        Ok(Expr::If(Box::new(c), Box::new(a), Box::new(b)))
                    }
                    "list" | "empty"
                        if w == "list"
                            || self.peek_at(1).and_then(|t| t.word()) == Some("list") =>
                    {
                        self.pos += 1;
                        if w == "empty" {
                            self.pos += 1;
                            return Ok(Expr::List(Vec::new()));
                        }
                        let mut items = Vec::new();
                        let prev = self.comma_stops;
                        self.comma_stops = true;
                        while !self.at_end() && self.starts_value_or_neg() {
                            items.push(self.parse_or()?);
                            if !self.eat_sym(",") {
                                break;
                            }
                        }
                        self.comma_stops = prev;
                        Ok(Expr::List(items))
                    }
                    "with" => {
                        self.pos += 1;
                        let mut fields = Vec::new();
                        let prev = self.comma_stops;
                        self.comma_stops = true;
                        loop {
                            let Some(Tok::Word(f)) = self.peek().cloned() else {
                                return self.err("expected a field name after `with`");
                            };
                            self.pos += 1;
                            let v = self.parse_or()?;
                            fields.push((f, v));
                            if !self.eat_sym(",") {
                                break;
                            }
                        }
                        self.comma_stops = prev;
                        Ok(Expr::Record(fields))
                    }
                    "random" => {
                        self.pos += 1;
                        let a = self.parse_add()?;
                        if !self.eat_word("to") {
                            return self.err("expected `to` in `random 1 to 6`");
                        }
                        let b = self.parse_add()?;
                        Ok(Expr::Random(Box::new(a), Box::new(b)))
                    }
                    "item" if self.peek_at(2).and_then(|t| t.word()) == Some("of") => {
                        self.pos += 1;
                        let idx = self.parse_primary()?;
                        self.pos += 1;
                        let list = self.parse_unary()?;
                        Ok(Expr::Item(Box::new(idx), Box::new(list)))
                    }
                    f if OF_FUNCS.contains(&f)
                        && self.peek_at(1).and_then(|t| t.word()) == Some("of") =>
                    {
                        self.pos += 2;
                        // `floor of seconds / 60` means floor(seconds / 60)
                        let arg = if matches!(
                            f,
                            "round" | "floor" | "ceil" | "abs" | "sqrt" | "number"
                        ) {
                            self.parse_mul()?
                        } else {
                            self.parse_unary()?
                        };
                        Ok(Expr::Of(f.to_string(), Box::new(arg)))
                    }
                    _ => {
                        if is_reserved_word(&w) {
                            return self.err(format!("expected a value, found `{w}`"));
                        }
                        self.pos += 1;
                        Ok(Expr::Name(w))
                    }
                }
            }
        }
    }

    fn starts_value_or_neg(&self) -> bool {
        self.starts_value() || self.peek().is_some_and(|t| t.is_sym("-"))
    }
}

/// Split `"Hi {name}!"` into literal and expression parts.
pub fn parse_text(s: &str, line_no: usize) -> R<Vec<TextPart>> {
    let mut parts = Vec::new();
    let mut lit = String::new();
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '{' {
            if chars.get(i + 1) == Some(&'{') {
                lit.push('{');
                i += 2;
                continue;
            }
            let Some(end) = chars[i + 1..].iter().position(|c| *c == '}') else {
                return Err(ExprError {
                    message: "`{` in text needs a matching `}`".into(),
                    help: Some("write \"Hi {name}\", or `{{` for a literal brace".into()),
                    tok: 0,
                });
            };
            let inner: String = chars[i + 1..i + 1 + end].iter().collect();
            if !lit.is_empty() {
                parts.push(TextPart::Lit(std::mem::take(&mut lit)));
            }
            let (toks, _) = tokenize_line(inner.trim(), line_no, 0).map_err(|e| ExprError {
                message: e.message,
                help: e.help,
                tok: 0,
            })?;
            let never = |_: &str| false;
            let mut p = ExprParser::new(&toks, 0, &never, line_no);
            p.comma_stops = false;
            let e = p.parse().map_err(|mut e| {
                e.message = format!("in `{{{inner}}}`: {}", e.message);
                e.tok = 0;
                e
            })?;
            if p.pos < toks.len() {
                return Err(ExprError {
                    message: format!("in `{{{inner}}}`: unexpected {}", toks[p.pos].describe()),
                    help: None,
                    tok: 0,
                });
            }
            parts.push(TextPart::Hole(e));
            i += end + 2;
            continue;
        }
        if c == '}' && chars.get(i + 1) == Some(&'}') {
            lit.push('}');
            i += 2;
            continue;
        }
        lit.push(c);
        i += 1;
    }
    if !lit.is_empty() || parts.is_empty() {
        parts.push(TextPart::Lit(lit));
    }
    Ok(parts)
}

/// Visit every name an expression reads (for checking).
pub fn walk_names<'e>(e: &'e Expr, f: &mut dyn FnMut(&'e str, bool)) {
    // the bool is "inside a `where` predicate" — bare names may be item fields there
    fn go<'e>(e: &'e Expr, inside_where: bool, f: &mut dyn FnMut(&'e str, bool)) {
        match e {
            Expr::Name(n) => f(n, inside_where),
            Expr::Text(parts) => {
                for p in parts {
                    if let TextPart::Hole(h) = p {
                        go(h, inside_where, f);
                    }
                }
            }
            Expr::Member(b, _)
            | Expr::Not(b)
            | Expr::Neg(b)
            | Expr::Of(_, b)
            | Expr::As(b, _)
            | Expr::IsEmpty(b, _) => go(b, inside_where, f),
            Expr::SortBy(b, _, _) => go(b, inside_where, f),
            Expr::Bin(_, a, b) | Expr::Random(a, b) | Expr::Item(a, b) => {
                go(a, inside_where, f);
                go(b, inside_where, f);
            }
            Expr::Where(a, b) => {
                go(a, inside_where, f);
                go(b, true, f);
            }
            Expr::If(a, b, c) => {
                go(a, inside_where, f);
                go(b, inside_where, f);
                go(c, inside_where, f);
            }
            Expr::List(items) => items.iter().for_each(|i| go(i, inside_where, f)),
            Expr::Record(fs) => fs.iter().for_each(|(_, v)| go(v, inside_where, f)),
            Expr::Num(_) | Expr::Bool(_) | Expr::Nothing | Expr::Color(_) => {}
        }
    }
    go(e, false, f)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(src: &str) -> Expr {
        let (t, _) = tokenize_line(src, 1, 0).unwrap();
        let never = |_: &str| false;
        let mut ep = ExprParser::new(&t, 0, &never, 1);
        let e = ep.parse().unwrap();
        assert_eq!(ep.pos, t.len(), "leftover tokens in {src}");
        e
    }

    #[test]
    fn precedence() {
        assert!(matches!(p("1 + 2 * 3"), Expr::Bin(Op::Add, _, _)));
        assert!(matches!(p("a is above 2 and b"), Expr::Bin(Op::And, _, _)));
        assert!(matches!(p("count of todos where not done"), Expr::Of(_, _)));
        assert!(matches!(p("todos where done"), Expr::Where(_, _)));
        // chained filters apply to the list, not to the last value in the condition
        match p("todos where folder is box where done sorted by title") {
            Expr::SortBy(inner, _, _) => match *inner {
                Expr::Where(list, cond) => {
                    assert!(matches!(*list, Expr::Where(_, _)));
                    assert!(matches!(*cond, Expr::Name(_)));
                }
                other => panic!("{other:?}"),
            },
            other => panic!("{other:?}"),
        }
        assert!(matches!(p("price * 2 as money"), Expr::Bin(Op::Mul, _, _)));
        assert!(matches!(p("x is not empty"), Expr::IsEmpty(_, false)));
        assert!(matches!(p("list 1, 2, 3"), Expr::List(v) if v.len() == 3));
        assert!(matches!(p("with title \"a\", done false"), Expr::Record(v) if v.len() == 2));
    }

    #[test]
    fn text_holes() {
        let parts = parse_text("Hi {name}, {count of items} left", 1).unwrap();
        assert_eq!(parts.len(), 5);
    }
}
