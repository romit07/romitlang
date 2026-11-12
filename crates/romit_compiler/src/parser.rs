use crate::ast::*;
use crate::diag::{snippet_at, Diagnostic};
use crate::lexer::{Lexer, Token, TokenKind};
use std::collections::BTreeMap;

pub struct Parser {
    file: String,
    source: String,
    tokens: Vec<Token>,
    pos: usize,
    errors: Vec<Diagnostic>,
    /// When true, a comma ends a list item the same way newline does (params, args).
    list_item_mode: bool,
    /// When set, a matching word ends expression parsing (e.g. `times` after `repeat N`).
    expr_stop_word: Option<String>,
}

impl Parser {
    pub fn parse_file(file: &str, source: &str) -> Result<Program, Vec<Diagnostic>> {
        let mut lexer = Lexer::new(source.to_string());
        let tokens = match lexer.tokenize(file) {
            Ok(t) => t,
            Err(mut d) => {
                d.file = file.to_string();
                return Err(vec![d]);
            }
        };
        let mut p = Parser {
            file: file.to_string(),
            source: source.to_string(),
            tokens,
            pos: 0,
            errors: Vec::new(),
            list_item_mode: false,
            expr_stop_word: None,
        };
        p.skip_newlines();
        let is_spec = matches!(p.peek().kind, TokenKind::Ident(ref s) if s == "spec");
        let result = if is_spec {
            p.parse_spec().map(Program::Spec)
        } else {
            p.parse_script().map(Program::Script)
        };
        match result {
            Ok(prog) if p.errors.is_empty() => Ok(prog),
            Ok(_) => Err(p.errors),
            Err(e) => {
                p.errors.push(e);
                Err(p.errors)
            }
        }
    }

    fn parse_script(&mut self) -> Result<Script, Diagnostic> {
        let span = self.span();
        let mut items = Vec::new();
        while !matches!(self.peek().kind, TokenKind::Eof) {
            self.skip_top_level();
            if matches!(self.peek().kind, TokenKind::Eof) {
                break;
            }
            if let TokenKind::Ident(ref w) = self.peek().kind {
                if script_conflicts_with_spec(w) {
                    return Err(Diagnostic::error(
                        "E005",
                        "a script cannot mix top-level statements with spec declarations (use one style per file)",
                        &self.file,
                        self.peek().line,
                        self.peek().column,
                        snippet_at(
                            &self.source,
                            self.peek().line,
                            self.peek().column,
                            w,
                        ),
                    )
                    .with_help(
                        "use `say`, `print`, `set`, functions (`name:`, `fn`, `function`), and control flow, or start with `spec`",
                    ));
                }
            }
            if self.peek_script_fn_def() {
                items.push(ScriptItem::Fn(self.parse_script_fn()?));
            } else if matches!(self.peek_word().as_deref(), Some("fn") | Some("function")) {
                return Err(self.err_fn_keyword_not_definition());
            } else {
                items.push(ScriptItem::Stmt(self.parse_script_stmt(false)?));
            }
            self.skip_newlines();
        }
        Ok(Script { items, span })
    }

    fn peek_script_fn_def(&self) -> bool {
        match self.peek_word().as_deref() {
            Some("fn") | Some("function") => true,
            _ => {
                matches!(self.peek().kind, TokenKind::Ident(_))
                    && matches!(
                        self.tokens.get(self.pos + 1).map(|t| &t.kind),
                        Some(TokenKind::Colon)
                    )
            }
        }
    }

    fn peek_word_at(&self, ahead: usize) -> Option<String> {
        let mut i = self.pos;
        let mut skipped = 0usize;
        while i < self.tokens.len() && skipped < ahead {
            if !matches!(self.tokens[i].kind, TokenKind::Newline) {
                skipped += 1;
            }
            i += 1;
        }
        match self.tokens.get(i).map(|t| &t.kind) {
            Some(TokenKind::Ident(s)) => Some(s.clone()),
            _ => None,
        }
    }

    fn err_fn_keyword_not_definition(&self) -> Diagnostic {
        let t = self.peek();
        Diagnostic::error(
            "P026",
            "`fn` and `function` start a definition only at the beginning of a line",
            &self.file,
            t.line,
            t.column,
            snippet_at(&self.source, t.line, t.column, "fn"),
        )
        .with_help("write `fn name …` or `name: …` with an indented body; calls stay `name 3, 4`")
    }

    fn parse_script_fn(&mut self) -> Result<ScriptFn, Diagnostic> {
        let span = self.span();
        let name = if self.peek_word() == Some("fn".into()) {
            self.bump();
            self.parse_fn_name_after_keyword("fn")?
        } else if self.peek_word() == Some("function".into()) {
            self.bump();
            self.parse_fn_name_after_keyword("function")?
        } else {
            let name = self.word()?;
            if !matches!(self.bump().kind, TokenKind::Colon) {
                return Err(Diagnostic::error(
                    "P027",
                    "calls never use `:`; only definitions use `name: …`",
                    &self.file,
                    self.peek().line,
                    self.peek().column,
                    snippet_at(&self.source, self.peek().line, self.peek().column, ":"),
                )
                .with_help("call with `name 3, 4` or define with `name: a, b` / `fn name a, b`"));
            }
            name
        };
        let params = self.parse_fn_param_list()?;
        self.skip_newlines();
        if !matches!(self.peek().kind, TokenKind::Indent) {
            return Err(Diagnostic::error(
                "E118",
                format!("function `{name}` needs an indented body after its header line"),
                &self.file,
                self.peek().line,
                self.peek().column,
                snippet_at(&self.source, self.peek().line, self.peek().column, "here"),
            )
            .with_help(
                "add indented lines under the header (`give` to return); try `name: …`, `fn name …`, or `function name …`",
            ));
        }
        self.expect_indent()?;
        let body = self.parse_script_block(true)?;
        self.expect_dedent()?;
        Ok(ScriptFn {
            name,
            params,
            body,
            span,
        })
    }

    fn parse_fn_name_after_keyword(&mut self, keyword: &str) -> Result<String, Diagnostic> {
        if self.at_newline() || self.peek_word().is_none() {
            let t = self.peek();
            return Err(Diagnostic::error(
                "E119",
                format!("expected a function name after `{keyword}`"),
                &self.file,
                t.line,
                t.column,
                snippet_at(&self.source, t.line, t.column, keyword),
            )
            .with_help("write `fn sum a, b` or `function hello` with an indented body"));
        }
        self.word()
    }

    fn parse_fn_param_list(&mut self) -> Result<Vec<ScriptParam>, Diagnostic> {
        let mut params = Vec::new();
        let mut expect_param = false;
        while !self.at_newline() {
            self.skip_space_commas_before_list_item(&mut expect_param)?;
            if self.at_newline() {
                break;
            }
            let ps = self.span();
            let pname = self.word()?;
            let default = if self.peek_word() == Some("is".into()) {
                self.bump();
                Some(self.parse_expr_on_fn_header_line()?)
            } else {
                None
            };
            params.push(ScriptParam {
                name: pname,
                default,
                span: ps,
            });
            expect_param = false;
            if matches!(self.peek().kind, TokenKind::Comma) {
                self.bump();
                expect_param = true;
                if self.at_newline() {
                    return Err(self.comma_err_trailing());
                }
            } else if !self.at_newline() && self.peek_word().is_some() {
                expect_param = true;
            }
        }
        Ok(params)
    }

    fn parse_script_block(&mut self, in_fn: bool) -> Result<Vec<ScriptStmt>, Diagnostic> {
        let mut stmts = Vec::new();
        while !matches!(self.peek().kind, TokenKind::Dedent | TokenKind::Eof) {
            self.skip_newlines();
            if matches!(self.peek().kind, TokenKind::Dedent | TokenKind::Eof) {
                break;
            }
            stmts.push(self.parse_script_stmt(in_fn)?);
            self.skip_newlines();
        }
        Ok(stmts)
    }

    fn parse_script_stmt(&mut self, in_fn: bool) -> Result<ScriptStmt, Diagnostic> {
        let stmt_span = self.span();
        let head = self
            .peek_word()
            .ok_or_else(|| self.err_here("a script statement"))?;
        match head.as_str() {
            "say" | "print" => {
                self.bump();
                let parts = self.parse_say_parts()?;
                Ok(ScriptStmt::Say {
                    parts,
                    span: stmt_span,
                })
            }
            "set" => {
                self.bump();
                let name = self.word()?;
                if self.word()? != "to" {
                    return Err(self.err_here("'to' after variable name"));
                }
                let value = self.parse_expr_until_line()?;
                Ok(ScriptStmt::Set {
                    name,
                    value,
                    span: stmt_span,
                })
            }
            "give" => {
                if !in_fn {
                    return Err(Diagnostic::error(
                        "P022",
                        "`give` is only allowed inside a `to` function body",
                        &self.file,
                        self.peek().line,
                        self.peek().column,
                        snippet_at(&self.source, self.peek().line, self.peek().column, "give"),
                    )
                    .with_help(
                        "define with `name: …`, `fn name …`, or `function name …`, then `give <value>` to return",
                    ));
                }
                self.bump();
                let value = self.parse_expr_until_line()?;
                Ok(ScriptStmt::Give {
                    value,
                    span: stmt_span,
                })
            }
            "if" => self.parse_script_if_chain(stmt_span, in_fn),
            "otherwise" => Err(Diagnostic::error(
                "P024",
                "`otherwise` must follow an `if` or `otherwise if` block",
                &self.file,
                self.peek().line,
                self.peek().column,
                snippet_at(
                    &self.source,
                    self.peek().line,
                    self.peek().column,
                    "otherwise",
                ),
            )
            .with_help(
                "use `if <condition>` first, then `otherwise if` or `otherwise` at the same indent",
            )),
            "repeat" => {
                self.bump();
                self.expr_stop_word = Some("times".into());
                let count = self.parse_expr_until_line()?;
                self.expr_stop_word = None;
                if self.word()? != "times" {
                    return Err(self.err_here("'times' after repeat count"));
                }
                self.skip_newlines();
                self.expect_indent()?;
                let body = self.parse_script_block(in_fn)?;
                self.expect_dedent()?;
                Ok(ScriptStmt::Repeat {
                    count,
                    body,
                    span: stmt_span,
                })
            }
            "for" => {
                self.bump();
                if self.word()? != "each" {
                    return Err(self.err_here("'each' after for"));
                }
                let var = self.word()?;
                if self.word()? != "in" {
                    return Err(self.err_here("'in' after loop variable"));
                }
                let from = self.parse_range_endpoint()?;
                if self.at_line_end() {
                    self.skip_newlines();
                    self.expect_indent()?;
                    let body = self.parse_script_block(in_fn)?;
                    self.expect_dedent()?;
                    return Ok(ScriptStmt::ForEachList {
                        var,
                        list: from,
                        body,
                        span: stmt_span,
                    });
                }
                if self.word()? != "to" {
                    return Err(
                        self.err_here("'to' after range start (e.g. `1 to 5`), or a list name")
                    );
                }
                let to = self.parse_range_endpoint()?;
                self.skip_newlines();
                self.expect_indent()?;
                let body = self.parse_script_block(in_fn)?;
                self.expect_dedent()?;
                Ok(ScriptStmt::ForEachRange {
                    var,
                    from,
                    to,
                    body,
                    span: stmt_span,
                })
            }
            "while" => {
                self.bump();
                let cond = self.parse_expr_until_line()?;
                self.skip_newlines();
                self.expect_indent()?;
                let body = self.parse_script_block(in_fn)?;
                self.expect_dedent()?;
                Ok(ScriptStmt::While {
                    cond,
                    body,
                    span: stmt_span,
                })
            }
            "stop" | "skip" if self.next_is_line_end(1) => {
                self.bump();
                Ok(if head == "stop" {
                    ScriptStmt::Stop { span: stmt_span }
                } else {
                    ScriptStmt::Skip { span: stmt_span }
                })
            }
            // `increase score by 10` / `decrease lives by 1`
            "increase" | "decrease"
                if matches!(
                    self.tokens.get(self.pos + 1).map(|t| &t.kind),
                    Some(TokenKind::Ident(_))
                ) =>
            {
                self.bump();
                let name = self.word()?;
                let amount = if self.peek_word().as_deref() == Some("by") {
                    self.bump();
                    self.parse_expr_until_line()?
                } else {
                    Expr::Literal(Literal::Int(1), stmt_span.clone())
                };
                let op = if head == "increase" {
                    BinaryOp::Add
                } else {
                    BinaryOp::Sub
                };
                Ok(self.compound_set(name, op, amount, stmt_span))
            }
            // `add "milk" to groceries` / `remove "milk" from groceries`
            "add" | "remove" if self.line_has_word(if head == "add" { "to" } else { "from" }) => {
                self.bump();
                let joiner = if head == "add" { "to" } else { "from" };
                self.expr_stop_word = Some(joiner.into());
                let item = self.parse_expr_until_line();
                self.expr_stop_word = None;
                let item = item?;
                if self.word()? != joiner {
                    return Err(self.err_here(&format!("'{joiner}' before the list name")));
                }
                let name = self.word()?;
                let list = Expr::Path(
                    PathExpr {
                        parts: vec![name.clone()],
                        span: stmt_span.clone(),
                    },
                    stmt_span.clone(),
                );
                let value = Expr::Call {
                    name: if head == "add" {
                        "__append".into()
                    } else {
                        "__remove".into()
                    },
                    args: vec![list, item],
                    named: Vec::new(),
                    span: stmt_span.clone(),
                };
                Ok(ScriptStmt::Set {
                    name,
                    value,
                    span: stmt_span,
                })
            }
            _ => {
                let next = self.tokens.get(self.pos + 1).map(|t| t.kind.clone());
                if matches!(
                    next,
                    Some(TokenKind::PlusAssign) | Some(TokenKind::MinusAssign)
                ) {
                    self.bump();
                    self.bump();
                    let amount = self.parse_expr_until_line()?;
                    let op = if matches!(next, Some(TokenKind::PlusAssign)) {
                        BinaryOp::Add
                    } else {
                        BinaryOp::Sub
                    };
                    return Ok(self.compound_set(head, op, amount, stmt_span));
                }
                let assign = matches!(next, Some(TokenKind::Assign));
                if !assign {
                    return Err(self
                        .err_here("a script statement")
                        .with_help(
                            "try `say`, `x = 5`, `if`, `fn name a, b`, `repeat N times`, `for each x in 1 to 5`, `for each item in items`, `add x to items`, or `while`",
                        ));
                }
                let name = head;
                self.bump();
                self.bump();
                let value = self.parse_expr_until_line()?;
                Ok(ScriptStmt::Set {
                    name,
                    value,
                    span: stmt_span,
                })
            }
        }
    }

    fn compound_set(&self, name: String, op: BinaryOp, amount: Expr, span: Span) -> ScriptStmt {
        let current = Expr::Path(
            PathExpr {
                parts: vec![name.clone()],
                span: span.clone(),
            },
            span.clone(),
        );
        ScriptStmt::Set {
            name,
            value: Expr::Binary {
                op,
                left: Box::new(current),
                right: Box::new(amount),
                span: span.clone(),
            },
            span,
        }
    }

    fn next_is_line_end(&self, ahead: usize) -> bool {
        matches!(
            self.tokens.get(self.pos + ahead).map(|t| &t.kind),
            None | Some(TokenKind::Newline)
                | Some(TokenKind::Dedent)
                | Some(TokenKind::Eof)
                | Some(TokenKind::Indent)
        )
    }

    fn line_has_word(&self, word: &str) -> bool {
        let mut i = self.pos;
        while let Some(t) = self.tokens.get(i) {
            match &t.kind {
                TokenKind::Newline | TokenKind::Eof | TokenKind::Indent | TokenKind::Dedent => {
                    return false
                }
                TokenKind::Ident(w) if w == word => return true,
                _ => i += 1,
            }
        }
        false
    }

    fn parse_script_if_chain(
        &mut self,
        if_span: Span,
        in_fn: bool,
    ) -> Result<ScriptStmt, Diagnostic> {
        self.bump();
        if self.peek_word() == Some("then".into()) {
            return Err(Diagnostic::error(
                "P023",
                "script `if` uses an indented block, not `then`",
                &self.file,
                self.peek().line,
                self.peek().column,
                snippet_at(&self.source, self.peek().line, self.peek().column, "then"),
            )
            .with_help("write `if <condition>` on one line, then indent the next lines"));
        }
        let cond = self.parse_expr_until_line()?;
        self.skip_newlines();
        self.expect_indent()?;
        let body = self.parse_script_block(in_fn)?;
        self.expect_dedent()?;
        let mut elif = Vec::new();
        let mut else_body = None;
        self.skip_newlines();
        while self.peek_word() == Some("otherwise".into()) {
            let branch_span = self.span();
            self.bump();
            if self.peek_word() == Some("if".into()) {
                self.bump();
                let cond = self.parse_expr_until_line()?;
                self.skip_newlines();
                self.expect_indent()?;
                let body = self.parse_script_block(in_fn)?;
                self.expect_dedent()?;
                elif.push(ScriptElif {
                    cond,
                    body,
                    span: branch_span,
                });
                self.skip_newlines();
                continue;
            }
            self.skip_newlines();
            self.expect_indent()?;
            let body = self.parse_script_block(in_fn)?;
            self.expect_dedent()?;
            else_body = Some(body);
            break;
        }
        Ok(ScriptStmt::If {
            cond,
            body,
            elif,
            else_body,
            span: if_span,
        })
    }

    fn parse_range_endpoint(&mut self) -> Result<Expr, Diagnostic> {
        self.parse_add_line()
    }

    fn parse_say_parts(&mut self) -> Result<Vec<Expr>, Diagnostic> {
        let mut parts = Vec::new();
        while !self.at_line_end() {
            if matches!(self.peek().kind, TokenKind::Comma) {
                return Err(self.comma_err_doubled());
            }
            let rest_of_say_line = !self.say_has_following_print_part();
            parts.push(self.parse_say_part_expr(rest_of_say_line)?);
            if self.at_line_end() {
                break;
            }
            if matches!(self.peek().kind, TokenKind::Comma) {
                self.bump();
                if self.at_line_end() {
                    return Err(self.comma_err_trailing());
                }
            }
        }
        if parts.is_empty() {
            return Err(self.err_here("a value after `say` or `print`"));
        }
        Ok(parts)
    }

    /// Another `say` part follows when comma is then a callable name (e.g. `say sum 1, sum 2, 3`).
    fn say_has_following_print_part(&self) -> bool {
        let mut i = self.pos;
        while i < self.tokens.len() {
            match &self.tokens[i].kind {
                TokenKind::Newline | TokenKind::Eof | TokenKind::Dedent | TokenKind::Indent => {
                    return false;
                }
                TokenKind::Comma => {
                    let mut j = i + 1;
                    while j < self.tokens.len() {
                        match &self.tokens[j].kind {
                            TokenKind::Newline
                            | TokenKind::Eof
                            | TokenKind::Dedent
                            | TokenKind::Indent => return false,
                            TokenKind::Ident(name) => {
                                return user_callable_name(name);
                            }
                            _ => {
                                j += 1;
                            }
                        }
                    }
                    return false;
                }
                _ => i += 1,
            }
        }
        false
    }

    fn parse_say_part_expr(&mut self, rest_of_line: bool) -> Result<Expr, Diagnostic> {
        let item_of = matches!(
            self.tokens.get(self.pos + 2).map(|t| &t.kind),
            Some(TokenKind::Ident(ref of)) if of == "of"
        ) && self.peek_word().as_deref() == Some("item");
        if let Some(w) = self.peek_word().filter(|_| !item_of) {
            if user_callable_name(&w) {
                let saved_pos = self.pos;
                let name = self.word()?;
                let span = self.span();
                if matches!(self.peek().kind, TokenKind::Colon) {
                    return Err(Diagnostic::error(
                        "P027",
                        "calls never use `:`; use `name 3, 4`",
                        &self.file,
                        self.peek().line,
                        self.peek().column,
                        snippet_at(&self.source, self.peek().line, self.peek().column, ":"),
                    )
                    .with_help("define with `name: …`, `fn name …`, or `function name …`"));
                }
                if self.looks_like_call_suffix() {
                    let (args, named) =
                        self.parse_call_args_line(CallArgMode::SayPart { rest_of_line })?;
                    return Ok(Expr::Call {
                        name,
                        args,
                        named,
                        span,
                    });
                }
                // `say a + b` / `say a remainder b` — infix, not a call.
                self.pos = saved_pos;
            }
        }
        let prev = self.list_item_mode;
        self.list_item_mode = true;
        let e = self.parse_or_line()?;
        self.list_item_mode = prev;
        Ok(e)
    }

    fn parse_expr_on_fn_header_line(&mut self) -> Result<Expr, Diagnostic> {
        let prev = self.list_item_mode;
        self.list_item_mode = true;
        let e = self.parse_add_line()?;
        self.list_item_mode = prev;
        Ok(e)
    }

    fn skip_space_commas_before_list_item(
        &mut self,
        need_item: &mut bool,
    ) -> Result<(), Diagnostic> {
        while matches!(self.peek().kind, TokenKind::Comma) {
            if !*need_item {
                return Err(self.comma_err_doubled());
            }
            self.bump();
            *need_item = false;
            if self.at_newline() {
                return Err(self.comma_err_trailing());
            }
        }
        Ok(())
    }

    fn comma_err_trailing(&self) -> Diagnostic {
        let t = self.peek();
        Diagnostic::error(
            "P014",
            "trailing comma",
            &self.file,
            t.line,
            t.column,
            snippet_at(&self.source, t.line, t.column, ","),
        )
        .with_help("remove the comma after the last item or add another value")
    }

    fn comma_err_doubled(&self) -> Diagnostic {
        let t = self.peek();
        Diagnostic::error(
            "P015",
            "unexpected comma right after another comma",
            &self.file,
            t.line,
            t.column,
            snippet_at(&self.source, t.line, t.column, ","),
        )
        .with_help("use a single comma between items")
    }

    fn skip_optional_commas_between_args(&mut self) -> Result<(), Diagnostic> {
        while matches!(self.peek().kind, TokenKind::Comma) {
            self.bump();
            if self.at_line_end() {
                return Err(self.comma_err_trailing());
            }
            if matches!(self.peek().kind, TokenKind::Comma) {
                return Err(self.comma_err_doubled());
            }
        }
        Ok(())
    }

    fn span(&self) -> Span {
        let t = self.peek();
        Span {
            line: t.line,
            column: t.column,
            end_line: t.line,
            end_column: t.column,
        }
    }

    fn peek(&self) -> &Token {
        self.tokens
            .get(self.pos)
            .unwrap_or_else(|| self.tokens.last().unwrap())
    }

    fn bump(&mut self) -> Token {
        let t = self.peek().clone();
        if !matches!(t.kind, TokenKind::Eof) {
            self.pos += 1;
        }
        t
    }

    fn at_newline(&self) -> bool {
        matches!(self.peek().kind, TokenKind::Newline)
    }

    fn skip_newlines(&mut self) {
        while matches!(self.peek().kind, TokenKind::Newline) {
            self.bump();
        }
    }

    fn skip_top_level(&mut self) {
        while matches!(self.peek().kind, TokenKind::Newline | TokenKind::Dedent) {
            self.bump();
        }
    }

    fn expect_indent(&mut self) -> Result<(), Diagnostic> {
        self.skip_newlines();
        if matches!(self.peek().kind, TokenKind::Indent) {
            self.bump();
            Ok(())
        } else {
            Err(self.err_here("indented block"))
        }
    }

    fn expect_dedent(&mut self) -> Result<(), Diagnostic> {
        self.skip_newlines();
        if matches!(self.peek().kind, TokenKind::Dedent) {
            self.bump();
        }
        Ok(())
    }

    fn err_here(&self, msg: &str) -> Diagnostic {
        let t = self.peek();
        Diagnostic::error(
            "P020",
            format!("expected {}", msg),
            &self.file,
            t.line,
            t.column,
            snippet_at(&self.source, t.line, t.column, "here"),
        )
    }

    fn err_lone_equals(&self) -> Diagnostic {
        let t = self.peek();
        Diagnostic::error(
            "P010",
            "unexpected `=`; use `is` or `==` for comparison",
            &self.file,
            t.line,
            t.column,
            snippet_at(&self.source, t.line, t.column, "="),
        )
        .with_help("assign at the start of a line with `x = value` or `set x to value`")
    }

    fn word(&mut self) -> Result<String, Diagnostic> {
        match self.bump().kind {
            TokenKind::Ident(s) => Ok(s),
            other => Err(Diagnostic::error(
                "P021",
                format!("expected a word, found {:?}", other),
                &self.file,
                self.peek().line,
                self.peek().column,
                snippet_at(&self.source, self.peek().line, self.peek().column, "word"),
            )),
        }
    }

    fn literal(&mut self) -> Result<Literal, Diagnostic> {
        match self.bump().kind {
            TokenKind::Int(n) => Ok(Literal::Int(n)),
            TokenKind::Decimal(s) => Ok(Literal::Decimal(s)),
            TokenKind::String(s) => Ok(Literal::Text(s)),
            TokenKind::Ident(s) if s == "true" => Ok(Literal::Bool(true)),
            TokenKind::Ident(s) if s == "false" => Ok(Literal::Bool(false)),
            TokenKind::Ident(s) => Ok(Literal::Enum(s)),
            _ => Err(self.err_here("literal")),
        }
    }

    fn parse_spec(&mut self) -> Result<Spec, Diagnostic> {
        let span = self.span();
        self.skip_newlines();
        if self.word()? != "spec" {
            return Err(self.err_here("'spec'"));
        }
        let name = self.word()?;
        if self.word()? != "version" {
            return Err(self.err_here("'version'"));
        }
        let version = match self.bump().kind {
            TokenKind::Int(v) => v,
            _ => return Err(self.err_here("version number")),
        };
        self.skip_newlines();
        let mut about = None;
        if let TokenKind::Ident(ref s) = self.peek().kind {
            if s == "about" {
                self.bump();
                about = Some(match self.bump().kind {
                    TokenKind::String(s) => s,
                    _ => return Err(self.err_here("about text")),
                });
                self.skip_newlines();
            }
        }
        let mut decls = Vec::new();
        while !matches!(self.peek().kind, TokenKind::Eof) {
            self.skip_top_level();
            if matches!(self.peek().kind, TokenKind::Eof) {
                break;
            }
            match self.parse_decl() {
                Ok(d) => decls.push(d),
                Err(e) => {
                    self.errors.push(e);
                    self.resync();
                }
            }
        }
        Ok(Spec {
            name,
            version,
            about,
            decls,
            span,
        })
    }

    fn resync(&mut self) {
        while !matches!(self.peek().kind, TokenKind::Eof) {
            if let TokenKind::Ident(ref s) = self.peek().kind {
                if matches!(
                    s.as_str(),
                    "enum"
                        | "config"
                        | "state"
                        | "entity"
                        | "event"
                        | "output"
                        | "define"
                        | "machine"
                        | "on"
                        | "invariant"
                        | "limit"
                        | "test"
                ) {
                    break;
                }
            }
            self.bump();
        }
    }

    fn parse_decl(&mut self) -> Result<Decl, Diagnostic> {
        self.skip_top_level();
        let head = self.word()?;
        match head.as_str() {
            "enum" => self.parse_enum(),
            "config" => {
                let span = self.span();
                self.skip_newlines();
                if !matches!(self.peek().kind, TokenKind::Indent) {
                    return Err(self.err_here("indented fields after config"));
                }
                self.bump();
                let mut fields = Vec::new();
                while !matches!(self.peek().kind, TokenKind::Dedent | TokenKind::Eof) {
                    self.skip_newlines();
                    if matches!(self.peek().kind, TokenKind::Dedent) {
                        break;
                    }
                    fields.push(self.parse_field()?);
                }
                if matches!(self.peek().kind, TokenKind::Dedent) {
                    self.bump();
                }
                Ok(Decl::Config(ConfigDecl { fields, span }))
            }
            "state" => {
                let span = self.span();
                self.skip_newlines();
                if !matches!(self.peek().kind, TokenKind::Indent) {
                    return Err(self.err_here("indented fields after state"));
                }
                self.bump();
                let mut fields = Vec::new();
                while !matches!(self.peek().kind, TokenKind::Dedent | TokenKind::Eof) {
                    self.skip_newlines();
                    if matches!(self.peek().kind, TokenKind::Dedent) {
                        break;
                    }
                    fields.push(self.parse_field()?);
                }
                if matches!(self.peek().kind, TokenKind::Dedent) {
                    self.bump();
                }
                Ok(Decl::State(StateDecl { fields, span }))
            }
            "entity" => self.parse_entity(),
            "event" => self.parse_event(),
            "output" => self.parse_output(),
            "define" => self.parse_define(),
            "machine" => self.parse_machine(),
            "on" => self.parse_handler(),
            "invariant" => self.parse_invariant(),
            "limit" => self.parse_limit(),
            "test" => self.parse_test(),
            _ => Err(self.err_here("declaration")),
        }
    }

    fn parse_enum(&mut self) -> Result<Decl, Diagnostic> {
        let span = self.span();
        let name = self.word()?;
        let mut variants = Vec::new();
        while !self.at_newline()
            && !matches!(
                self.peek().kind,
                TokenKind::Eof | TokenKind::Dedent | TokenKind::Indent
            )
        {
            let w = self.word()?;
            if w == "=" || w == "|" || w == "or" {
                continue;
            }
            variants.push(w);
        }
        self.skip_newlines();
        Ok(Decl::Enum(EnumDecl {
            name,
            variants,
            span,
        }))
    }

    fn parse_field(&mut self) -> Result<Field, Diagnostic> {
        let span = self.span();
        let name = self.word()?;
        let ty = Type::Named(self.word()?);
        let mut compare = None;
        if let TokenKind::Ident(ref w) = self.peek().kind {
            if w == "above" || w == "below" || w == "at" {
                let op = self.word()?;
                if op == "at" {
                    let w2 = self.word()?;
                    if w2 != "least" {
                        self.pos -= 1;
                    }
                }
                let lit = self.literal()?;
                compare = Some((map_compare(&op), lit));
            }
        }
        let mut default = None;
        if let TokenKind::Ident(ref w) = self.peek().kind {
            if w == "=" || w == "is" {
                self.bump();
                default = Some(self.literal()?);
            }
        }
        Ok(Field {
            name,
            ty,
            compare,
            default,
            span,
        })
    }

    fn parse_entity(&mut self) -> Result<Decl, Diagnostic> {
        let span = self.span();
        let name = self.word()?;
        if self.word()? != "keyed" || self.word()? != "by" {
            return Err(self.err_here("'keyed by'"));
        }
        let key = self.word()?;
        self.skip_newlines();
        self.expect_indent()?;
        let mut fields = Vec::new();
        while !matches!(self.peek().kind, TokenKind::Dedent | TokenKind::Eof) {
            self.skip_newlines();
            if matches!(self.peek().kind, TokenKind::Dedent) {
                break;
            }
            fields.push(self.parse_field()?);
            self.skip_newlines();
        }
        self.expect_dedent()?;
        Ok(Decl::Entity(EntityDecl {
            name,
            key,
            fields,
            span,
        }))
    }

    fn parse_event(&mut self) -> Result<Decl, Diagnostic> {
        let span = self.span();
        let name = self.word()?;
        let mut params = Vec::new();
        let mut stores = StoreMode::None;
        if self.peek_word() == Some("with".into()) {
            self.bump();
            while !self.at_newline() {
                if let TokenKind::Ident(ref bw) = self.peek().kind {
                    if bw == "and" {
                        self.bump();
                        continue;
                    }
                    if bw == "stores" {
                        break;
                    }
                }
                let pname = self.word()?;
                let pty = Type::Named(self.word()?);
                params.push(Param {
                    name: pname,
                    ty: pty,
                    span: self.span(),
                });
            }
        }
        let inline_stores = self.peek_word().as_deref() == Some("stores");
        if !inline_stores {
            self.skip_newlines();
        }
        if !inline_stores && matches!(self.peek().kind, TokenKind::Indent) {
            self.bump();
            while !matches!(self.peek().kind, TokenKind::Dedent | TokenKind::Eof) {
                self.skip_newlines();
                if matches!(self.peek().kind, TokenKind::Dedent) {
                    break;
                }
                params.push(self.parse_param_line()?);
                self.skip_newlines();
            }
            self.expect_dedent()?;
        }
        self.skip_newlines();
        if let TokenKind::Ident(ref w) = self.peek().kind {
            if w == "stores" {
                self.bump();
                let w = self.word()?;
                stores = if w == "all" {
                    StoreMode::All
                } else {
                    let mut names = vec![w];
                    while let TokenKind::Ident(_) = self.peek().kind {
                        names.push(self.word()?);
                        if self.at_newline() {
                            break;
                        }
                    }
                    StoreMode::Named(names)
                };
            }
        }
        self.skip_newlines();
        Ok(Decl::Event(EventDecl {
            name,
            params,
            stores,
            span,
        }))
    }

    fn parse_param_line(&mut self) -> Result<Param, Diagnostic> {
        let span = self.span();
        let name = self.word()?;
        let ty = Type::Named(self.word()?);
        Ok(Param { name, ty, span })
    }

    fn parse_output(&mut self) -> Result<Decl, Diagnostic> {
        let span = self.span();
        let name = self.word()?;
        let mut params = Vec::new();
        self.skip_newlines();
        if matches!(self.peek().kind, TokenKind::Indent) {
            self.bump();
            while !matches!(self.peek().kind, TokenKind::Dedent | TokenKind::Eof) {
                self.skip_newlines();
                params.push(self.parse_param_line()?);
                self.skip_newlines();
            }
            self.expect_dedent()?;
        }
        self.skip_newlines();
        Ok(Decl::Output(OutputDecl { name, params, span }))
    }

    fn parse_define(&mut self) -> Result<Decl, Diagnostic> {
        let span = self.span();
        let name = self.word()?;
        let mut params = Vec::new();
        while let Some(w) = self.peek_word() {
            if w == "returns" {
                break;
            }
            let pname = self.word()?;
            let pty = Type::Named(self.word()?);
            params.push(Param {
                name: pname,
                ty: pty,
                span: self.span(),
            });
        }
        if self.word()? != "returns" {
            return Err(self.err_here("'returns'"));
        }
        let ret = Type::Named(self.word()?);
        self.skip_newlines();
        self.expect_indent()?;
        let body = self.parse_expr_until_line()?;
        self.expect_dedent()?;
        Ok(Decl::Define(DefineDecl {
            name,
            params,
            ret,
            body,
            span,
        }))
    }

    fn parse_machine(&mut self) -> Result<Decl, Diagnostic> {
        let span = self.span();
        let path = self.parse_path_words()?;
        self.skip_newlines();
        self.expect_indent()?;
        let mut transitions = Vec::new();
        while !matches!(self.peek().kind, TokenKind::Dedent | TokenKind::Eof) {
            self.skip_newlines();
            transitions.push(self.parse_transition()?);
            self.skip_newlines();
        }
        self.expect_dedent()?;
        Ok(Decl::Machine(MachineDecl {
            path,
            transitions,
            span,
        }))
    }

    fn parse_transition(&mut self) -> Result<MachineTransition, Diagnostic> {
        let span = self.span();
        let from = self.word()?;
        if self.word()? != "to" {
            return Err(self.err_here("'to'"));
        }
        // accept `to` only
        let mut to = Vec::new();
        if let TokenKind::Ident(ref w) = self.peek().kind {
            if w == "final" {
                self.bump();
                return Ok(MachineTransition {
                    from,
                    to: vec![],
                    final_state: true,
                    span,
                });
            }
        }
        to.push(self.word()?);
        while let TokenKind::Ident(_) = self.peek().kind {
            let w = self.word()?;
            if w == "|" || w == "or" {
                to.push(self.word()?);
            } else {
                self.pos -= 1;
                break;
            }
        }
        Ok(MachineTransition {
            from,
            to,
            final_state: false,
            span,
        })
    }

    fn parse_handler(&mut self) -> Result<Decl, Diagnostic> {
        let span = self.span();
        let event = self.word()?;
        let mut bindings = Vec::new();
        if let TokenKind::Ident(ref w) = self.peek().kind {
            if w == "with" {
                self.bump();
                while !self.at_newline() && !matches!(self.peek().kind, TokenKind::Indent) {
                    if let TokenKind::Ident(ref bw) = self.peek().kind {
                        if bw == "if" || bw == "all" {
                            break;
                        }
                    }
                    bindings.push(self.word()?);
                    if !matches!(self.peek().kind, TokenKind::Ident(_)) {
                        break;
                    }
                }
            }
        }
        let mut condition = None;
        if let TokenKind::Ident(ref w) = self.peek().kind {
            if w == "if" {
                self.bump();
                condition = Some(self.parse_expr_until_line()?);
            }
        }
        let mut run_all = false;
        if let TokenKind::Ident(ref w) = self.peek().kind {
            if w == "all" {
                self.bump();
                run_all = true;
            }
        }
        self.skip_newlines();
        self.expect_indent()?;
        let mut rules = Vec::new();
        while !matches!(self.peek().kind, TokenKind::Dedent | TokenKind::Eof) {
            self.skip_newlines();
            rules.push(self.parse_rule()?);
        }
        self.expect_dedent()?;
        Ok(Decl::Handler(HandlerDecl {
            event,
            bindings,
            condition,
            run_all,
            rules,
            span,
        }))
    }

    fn parse_rule(&mut self) -> Result<RuleDecl, Diagnostic> {
        let span = self.span();
        if self.word()? != "rule" {
            return Err(self.err_here("'rule'"));
        }
        let name = self.word()?;
        self.skip_newlines();
        self.expect_indent()?;
        let when = if self.peek_word() == Some("when".into()) {
            self.bump();
            RuleWhen::Expr(self.parse_expr_until_line()?)
        } else if self.peek_word() == Some("otherwise".into()) {
            self.bump();
            RuleWhen::Otherwise
        } else {
            return Err(Diagnostic::error(
                "P012",
                "a rule needs a \"when\" or \"otherwise\" line first",
                &self.file,
                self.peek().line,
                self.peek().column,
                snippet_at(
                    &self.source,
                    self.peek().line,
                    self.peek().column,
                    "rule body",
                ),
            )
            .with_help("add  when <condition>  on the first indented line"));
        };
        self.skip_newlines();
        let mut actions = Vec::new();
        while !matches!(self.peek().kind, TokenKind::Dedent | TokenKind::Eof) {
            self.skip_newlines();
            actions.push(self.parse_action()?);
            self.skip_newlines();
        }
        self.expect_dedent()?;
        Ok(RuleDecl {
            name,
            when,
            actions,
            span,
        })
    }

    fn parse_action(&mut self) -> Result<Action, Diagnostic> {
        let span = self.span();
        let head = self.word()?;
        match head.as_str() {
            "set" => {
                let path = self.parse_path_words()?;
                if self.word()? != "to" {
                    return Err(self.err_here("'to'"));
                }
                let value = self.parse_expr_until_line()?;
                Ok(Action::Set { path, value, span })
            }
            "emit" => {
                let name = self.word()?;
                let mut args = Vec::new();
                if self.peek_word() == Some("with".into()) {
                    self.bump();
                    while !self.at_newline() {
                        let _field = self.word()?;
                        args.push(self.parse_expr_primary()?);
                    }
                }
                Ok(Action::Emit { name, args, span })
            }
            "reject" => {
                let msg = match self.bump().kind {
                    TokenKind::String(s) => s,
                    _ => return Err(self.err_here("reject message string")),
                };
                Ok(Action::Reject { message: msg, span })
            }
            "say" | "print" => {
                let parts = self.parse_say_parts()?;
                Ok(Action::Say { parts, span })
            }
            _ => Err(self.err_here("set, emit, say, print, or reject")),
        }
    }

    fn parse_invariant(&mut self) -> Result<Decl, Diagnostic> {
        let span = self.span();
        let name = self.word()?;
        self.skip_newlines();
        self.expect_indent()?;
        let expr = if self.peek_word() == Some("for".into()) {
            self.parse_for_invariant()?
        } else {
            self.parse_expr_until_line()?
        };
        self.skip_newlines();
        self.expect_dedent()?;
        Ok(Decl::Invariant(InvariantDecl { name, expr, span }))
    }

    fn parse_for_invariant(&mut self) -> Result<Expr, Diagnostic> {
        let span = self.span();
        self.bump();
        let _each = self.word()?;
        let var = self.word()?;
        if self.word()? != "in" {
            return Err(self.err_here("'in'"));
        }
        let entity = self.word()?;
        let body = self.parse_expr_until_line()?;
        Ok(Expr::ForEach {
            var,
            entity,
            body: Box::new(body),
            span,
        })
    }

    fn parse_limit(&mut self) -> Result<Decl, Diagnostic> {
        let span = self.span();
        let output = self.word()?;
        if self.word()? != "at" || self.word()? != "most" {
            return Err(self.err_here("'at most'"));
        }
        let max = match self.bump().kind {
            TokenKind::Int(n) => n,
            _ => return Err(self.err_here("limit number")),
        };
        if self.word()? != "per" {
            return Err(self.err_here("'per'"));
        }
        let per = self.word()?;
        self.skip_newlines();
        Ok(Decl::Limit(LimitDecl {
            output,
            max,
            per,
            span,
        }))
    }

    fn parse_test(&mut self) -> Result<Decl, Diagnostic> {
        let span = self.span();
        let name = self.word()?;
        self.skip_newlines();
        self.expect_indent()?;
        let mut givens = Vec::new();
        let mut steps = Vec::new();
        while !matches!(self.peek().kind, TokenKind::Dedent | TokenKind::Eof) {
            self.skip_newlines();
            if matches!(self.peek().kind, TokenKind::Dedent | TokenKind::Eof) {
                break;
            }
            let w = self.peek_word().unwrap_or_default();
            match w.as_str() {
                "given" => {
                    givens.push(self.parse_given()?);
                }
                "send" => {
                    steps.push(TestStep::Send(self.parse_send()?));
                }
                "expect" => {
                    steps.push(TestStep::Expect(self.parse_expect()?));
                }
                _ => {
                    self.bump();
                }
            }
        }
        self.expect_dedent()?;
        Ok(Decl::Test(TestDecl {
            name,
            givens,
            steps,
            span,
        }))
    }

    fn parse_given(&mut self) -> Result<Given, Diagnostic> {
        let span = self.span();
        self.bump();
        let name = self.word()?;
        if matches!(self.peek().kind, TokenKind::Indent) {
            self.bump();
            let mut fields = BTreeMap::new();
            while !matches!(self.peek().kind, TokenKind::Dedent) {
                self.skip_newlines();
                if matches!(self.peek().kind, TokenKind::Dedent) {
                    break;
                }
                let fname = self.word()?;
                fields.insert(fname, self.parse_expr_primary()?);
                self.skip_newlines();
            }
            self.expect_dedent()?;
            Ok(Given::EntityInstance {
                entity: name,
                fields,
                span,
            })
        } else if !self.at_newline()
            && !matches!(self.peek_word().as_deref(), Some("is") | Some("to"))
        {
            let mut fields = BTreeMap::new();
            while !self.at_newline() {
                let fname = self.word()?;
                fields.insert(fname, self.parse_expr_primary()?);
            }
            Ok(Given::EntityInstance {
                entity: name,
                fields,
                span,
            })
        } else {
            if matches!(self.peek_word().as_deref(), Some("to") | Some("is")) {
                self.bump();
            }
            let value = self.parse_expr_until_line()?;
            Ok(Given::StateAssign { name, value, span })
        }
    }

    fn parse_send(&mut self) -> Result<Send, Diagnostic> {
        let span = self.span();
        self.bump();
        let event = self.word()?;
        let mut args = BTreeMap::new();
        if let TokenKind::Ident(ref w) = self.peek().kind {
            if w == "with" {
                self.bump();
                while !self.at_newline() {
                    let k = self.word()?;
                    args.insert(k, self.parse_expr_primary()?);
                }
            }
        }
        self.skip_newlines();
        Ok(Send { event, args, span })
    }

    fn parse_expect(&mut self) -> Result<Expect, Diagnostic> {
        let span = self.span();
        self.bump();
        let w = self.word()?;
        if w == "emitted" {
            let output = self.word()?;
            let mut args = Vec::new();
            if let TokenKind::Ident(ref x) = self.peek().kind {
                if x == "with" {
                    self.bump();
                    while !self.at_newline() {
                        let _field = self.word()?;
                        args.push(self.parse_expr_primary()?);
                    }
                }
            }
            return Ok(Expect::Emitted { output, args, span });
        }
        if w == "rejected" {
            let msg = match self.bump().kind {
                TokenKind::String(s) => s,
                _ => return Err(self.err_here("rejection message")),
            };
            return Ok(Expect::Rejected { message: msg, span });
        }
        if w == "no" && self.peek_word() == Some("outputs".into()) {
            self.bump();
            return Ok(Expect::NoOutputs { span });
        }
        self.pos -= 1;
        let expr = self.parse_expr_until_line()?;
        Ok(Expect::Expr { expr, span })
    }

    fn peek_word(&self) -> Option<String> {
        match &self.peek().kind {
            TokenKind::Ident(s) => Some(s.clone()),
            _ => None,
        }
    }

    fn parse_path_words(&mut self) -> Result<PathExpr, Diagnostic> {
        let span = self.span();
        let mut parts = vec![self.word()?];
        while matches!(self.peek().kind, TokenKind::Dot) {
            self.bump();
            parts.push(self.word()?);
        }
        Ok(PathExpr { parts, span })
    }

    fn parse_expr_until_line(&mut self) -> Result<Expr, Diagnostic> {
        self.parse_or_line()
    }

    fn at_line_end(&self) -> bool {
        matches!(
            self.peek().kind,
            TokenKind::Newline | TokenKind::Dedent | TokenKind::Eof | TokenKind::Indent
        ) || (self.list_item_mode && matches!(self.peek().kind, TokenKind::Comma))
            || (self.expr_stop_word.is_some()
                && self.expr_stop_word.as_deref() != Some("times")
                && self.peek_word() == self.expr_stop_word)
    }

    fn at_physical_line_end(&self) -> bool {
        matches!(
            self.peek().kind,
            TokenKind::Newline | TokenKind::Dedent | TokenKind::Eof | TokenKind::Indent
        )
    }

    fn parse_or_line(&mut self) -> Result<Expr, Diagnostic> {
        let mut e = self.parse_and_line()?;
        while self.peek_word() == Some("or".into()) && !self.at_line_end() {
            let span = self.span();
            self.bump();
            let r = self.parse_and_line()?;
            e = Expr::Binary {
                op: BinaryOp::Or,
                left: Box::new(e),
                right: Box::new(r),
                span,
            };
        }
        Ok(e)
    }

    fn parse_and_line(&mut self) -> Result<Expr, Diagnostic> {
        let mut e = self.parse_not_line()?;
        while self.peek_word() == Some("and".into()) && !self.at_line_end() {
            let span = self.span();
            self.bump();
            let r = self.parse_not_line()?;
            e = Expr::Binary {
                op: BinaryOp::And,
                left: Box::new(e),
                right: Box::new(r),
                span,
            };
        }
        Ok(e)
    }

    fn parse_not_line(&mut self) -> Result<Expr, Diagnostic> {
        if self.peek_word() == Some("not".into()) {
            let span = self.span();
            self.bump();
            let e = self.parse_not_line()?;
            return Ok(Expr::Unary {
                op: UnaryOp::Not,
                expr: Box::new(e),
                span,
            });
        }
        self.parse_cmp_line()
    }

    fn comparison_op_after_is(&mut self) -> BinaryOp {
        if self.peek_word() == Some("not".into()) {
            self.bump();
            return BinaryOp::Ne;
        }
        if self.peek_word() == Some("below".into()) {
            self.bump();
            return BinaryOp::Lt;
        }
        if self.peek_word() == Some("above".into()) {
            self.bump();
            return BinaryOp::Gt;
        }
        if self.peek_word() == Some("less".into()) {
            self.bump();
            if self.peek_word() == Some("than".into()) {
                self.bump();
            }
            return BinaryOp::Lt;
        }
        if self.peek_word() == Some("more".into()) {
            self.bump();
            if self.peek_word() == Some("than".into()) {
                self.bump();
            }
            return BinaryOp::Gt;
        }
        if self.peek_word() == Some("at".into()) {
            self.bump();
            if self.peek_word() == Some("least".into()) {
                self.bump();
                return BinaryOp::Ge;
            }
            if self.peek_word() == Some("most".into()) {
                self.bump();
                return BinaryOp::Le;
            }
        }
        BinaryOp::Eq
    }

    fn parse_cmp_line(&mut self) -> Result<Expr, Diagnostic> {
        let mut e = self.parse_add_line()?;
        loop {
            if self.at_line_end() {
                break;
            }
            if matches!(self.peek_word().as_deref(), Some("contains") | Some("has")) {
                let span = self.span();
                self.bump();
                let r = self.parse_add_line()?;
                e = Expr::Binary {
                    op: BinaryOp::In,
                    left: Box::new(r),
                    right: Box::new(e),
                    span,
                };
                continue;
            }
            let op = if self.peek_word() == Some("is".into()) {
                self.bump();
                if self.peek_word() == Some("in".into()) {
                    self.bump();
                    BinaryOp::In
                } else {
                    self.comparison_op_after_is()
                }
            } else if self.peek_word() == Some("in".into())
                && self.expr_stop_word.as_deref() != Some("in")
            {
                self.bump();
                BinaryOp::In
            } else if self.peek_word() == Some("above".into()) {
                self.bump();
                BinaryOp::Gt
            } else if self.peek_word() == Some("below".into()) {
                self.bump();
                BinaryOp::Lt
            } else {
                match self.peek().kind {
                    TokenKind::Assign => return Err(self.err_lone_equals()),
                    TokenKind::Eq => {
                        self.bump();
                        BinaryOp::Eq
                    }
                    TokenKind::Ne => {
                        self.bump();
                        BinaryOp::Ne
                    }
                    TokenKind::Lt => {
                        self.bump();
                        BinaryOp::Lt
                    }
                    TokenKind::Le => {
                        self.bump();
                        BinaryOp::Le
                    }
                    TokenKind::Gt => {
                        self.bump();
                        BinaryOp::Gt
                    }
                    TokenKind::Ge => {
                        self.bump();
                        BinaryOp::Ge
                    }
                    _ => break,
                }
            };
            let span = self.span();
            let r = self.parse_add_line()?;
            e = Expr::Binary {
                op,
                left: Box::new(e),
                right: Box::new(r),
                span,
            };
        }
        Ok(e)
    }

    fn parse_add_line(&mut self) -> Result<Expr, Diagnostic> {
        let mut e = self.parse_mul_line()?;
        loop {
            if self.at_line_end() {
                break;
            }
            let op = if self.peek_word() == Some("plus".into()) {
                self.bump();
                BinaryOp::Add
            } else if self.peek_word() == Some("minus".into()) {
                self.bump();
                BinaryOp::Sub
            } else {
                match self.peek().kind {
                    TokenKind::Plus => {
                        self.bump();
                        BinaryOp::Add
                    }
                    TokenKind::Minus => {
                        self.bump();
                        BinaryOp::Sub
                    }
                    _ => break,
                }
            };
            let span = self.span();
            let r = self.parse_mul_line()?;
            e = Expr::Binary {
                op,
                left: Box::new(e),
                right: Box::new(r),
                span,
            };
        }
        if matches!(self.peek().kind, TokenKind::Assign) {
            return Err(self.err_lone_equals());
        }
        Ok(e)
    }

    fn parse_mul_line(&mut self) -> Result<Expr, Diagnostic> {
        let mut e = self.parse_unary_line()?;
        loop {
            if self.at_line_end() {
                break;
            }
            let op = if self.peek_word() == Some("times".into()) {
                if self.expr_stop_word.as_deref().is_some_and(|w| w == "times") {
                    break;
                }
                self.bump();
                BinaryOp::Mul
            } else if self.peek_word() == Some("divided".into()) {
                self.bump();
                let w = self.word()?;
                if w != "by" {
                    return Err(self.err_here("'by' after divided"));
                }
                BinaryOp::Div
            } else if self.peek_word() == Some("remainder".into()) {
                self.bump();
                BinaryOp::Mod
            } else {
                match self.peek().kind {
                    TokenKind::Star => {
                        self.bump();
                        BinaryOp::Mul
                    }
                    TokenKind::Slash => {
                        self.bump();
                        BinaryOp::Div
                    }
                    TokenKind::Percent => {
                        self.bump();
                        BinaryOp::Mod
                    }
                    _ => break,
                }
            };
            let span = self.span();
            let r = self.parse_unary_line()?;
            e = Expr::Binary {
                op,
                left: Box::new(e),
                right: Box::new(r),
                span,
            };
        }
        Ok(e)
    }

    fn parse_unary_line(&mut self) -> Result<Expr, Diagnostic> {
        if matches!(self.peek().kind, TokenKind::Minus) {
            let span = self.span();
            self.bump();
            let e = self.parse_unary_line()?;
            return Ok(Expr::Unary {
                op: UnaryOp::Neg,
                expr: Box::new(e),
                span,
            });
        }
        self.parse_postfix_line()
    }

    fn parse_postfix_line(&mut self) -> Result<Expr, Diagnostic> {
        let mut e = self.parse_primary()?;
        loop {
            if self.at_line_end() {
                break;
            }
            if let Expr::Path(p, path_span) = &e {
                if p.parts.len() == 1
                    && user_callable_name(&p.parts[0])
                    && self.looks_like_call_suffix()
                {
                    let name = p.parts[0].clone();
                    let call_span = Span {
                        line: path_span.line,
                        column: path_span.column,
                        end_line: self.peek().line,
                        end_column: self.peek().column,
                    };
                    if matches!(self.peek().kind, TokenKind::Colon) {
                        return Err(Diagnostic::error(
                            "P027",
                            "calls never use `:`; use `name 3, 4`",
                            &self.file,
                            self.peek().line,
                            self.peek().column,
                            snippet_at(&self.source, self.peek().line, self.peek().column, ":"),
                        )
                        .with_help("define with `name: …`, `fn name …`, or `function name …`"));
                    }
                    let (args, named) = self.parse_call_args_line(CallArgMode::RestOfLine)?;
                    e = Expr::Call {
                        name,
                        args,
                        named,
                        span: call_span,
                    };
                    break;
                }
            }
            if self.peek_word() == Some("of".into()) {
                let span = self.span();
                self.bump();
                let arg = self.parse_unary_line()?;
                if let Expr::Path(p, _) = &e {
                    e = Expr::Call {
                        name: p.parts[0].clone(),
                        args: vec![arg],
                        named: Vec::new(),
                        span,
                    };
                    continue;
                }
            }
            break;
        }
        Ok(e)
    }

    fn parse_call_args_line(
        &mut self,
        mode: CallArgMode,
    ) -> Result<(Vec<Expr>, Vec<NamedArg>), Diagnostic> {
        let mut args = Vec::new();
        let mut named = Vec::new();
        let prev = self.list_item_mode;
        self.list_item_mode = true;
        let mut need_arg = true;
        while !self.at_physical_line_end() {
            if matches!(self.peek().kind, TokenKind::Comma) {
                if need_arg {
                    self.list_item_mode = prev;
                    return Err(self.comma_err_doubled());
                }
                if matches!(
                    mode,
                    CallArgMode::SayPart {
                        rest_of_line: false
                    }
                ) {
                    if self.comma_precedes_callable_say_part() {
                        break;
                    }
                }
                self.bump();
                need_arg = true;
                if self.at_physical_line_end() {
                    self.list_item_mode = prev;
                    return Err(self.comma_err_trailing());
                }
                continue;
            }
            if self.peek_word() == Some("with".into()) {
                self.bump();
                while !self.at_physical_line_end() {
                    if matches!(self.peek().kind, TokenKind::Comma) {
                        self.bump();
                        if self.at_physical_line_end() {
                            self.list_item_mode = prev;
                            return Err(self.comma_err_trailing());
                        }
                        continue;
                    }
                    let span = self.span();
                    let pname = self.word()?;
                    let value = self.parse_add_line()?;
                    named.push(NamedArg {
                        name: pname,
                        value,
                        span,
                    });
                }
                self.list_item_mode = prev;
                return Ok((args, named));
            }
            if !self.might_start_call_arg() {
                break;
            }
            let arg = self.parse_add_line()?;
            if matches!(&arg, Expr::Call { .. })
                && self.might_start_call_arg()
                && !matches!(
                    mode,
                    CallArgMode::SayPart {
                        rest_of_line: false
                    }
                )
            {
                self.list_item_mode = prev;
                let t = self.peek();
                return Err(Diagnostic::error(
                    "P028",
                    "a nested call is only allowed as the last argument on the line",
                    &self.file,
                    t.line,
                    t.column,
                    snippet_at(&self.source, t.line, t.column, "call"),
                )
                .with_help("set the inner result to a variable first, then pass it"));
            }
            args.push(arg);
            need_arg = false;
        }
        self.list_item_mode = prev;
        Ok((args, named))
    }

    fn comma_precedes_callable_say_part(&self) -> bool {
        let mut i = self.pos + 1;
        while i < self.tokens.len() {
            match &self.tokens[i].kind {
                TokenKind::Newline | TokenKind::Eof | TokenKind::Dedent | TokenKind::Indent => {
                    return false;
                }
                TokenKind::Ident(name) => return user_callable_name(name),
                _ => i += 1,
            }
        }
        false
    }

    fn might_start_call_arg(&self) -> bool {
        if self.at_line_end() {
            return false;
        }
        match &self.peek().kind {
            TokenKind::Int(_)
            | TokenKind::Decimal(_)
            | TokenKind::String(_)
            | TokenKind::Color(_)
            | TokenKind::Minus => true,
            TokenKind::Ident(s) => !call_arg_stop_word(s),
            _ => false,
        }
    }

    fn looks_like_infix_after_value(&self) -> bool {
        if self.token_starts_infix_op() {
            return true;
        }
        matches!(
            self.peek_word().as_deref(),
            Some(
                "plus" | "minus" | "times" | "divided" | "remainder" | "is" | "not" | "and" | "or"
            )
        )
    }

    fn token_starts_infix_op(&self) -> bool {
        matches!(
            self.peek().kind,
            TokenKind::Plus
                | TokenKind::Minus
                | TokenKind::Star
                | TokenKind::Slash
                | TokenKind::Percent
                | TokenKind::Eq
                | TokenKind::Ne
                | TokenKind::Lt
                | TokenKind::Le
                | TokenKind::Gt
                | TokenKind::Ge
        )
    }

    fn looks_like_call_suffix(&self) -> bool {
        if self.at_line_end() {
            return false;
        }
        if self.peek_word() == Some("with".into()) {
            return true;
        }
        if self.looks_like_infix_after_value() {
            return false;
        }
        self.might_start_call_arg()
    }

    fn parse_or(&mut self) -> Result<Expr, Diagnostic> {
        let mut e = self.parse_and()?;
        while self.peek_word() == Some("or".into()) {
            let span = self.span();
            self.bump();
            let r = self.parse_and()?;
            e = Expr::Binary {
                op: BinaryOp::Or,
                left: Box::new(e),
                right: Box::new(r),
                span,
            };
        }
        Ok(e)
    }

    fn parse_and(&mut self) -> Result<Expr, Diagnostic> {
        let mut e = self.parse_not()?;
        while self.peek_word() == Some("and".into()) {
            let span = self.span();
            self.bump();
            let r = self.parse_not()?;
            e = Expr::Binary {
                op: BinaryOp::And,
                left: Box::new(e),
                right: Box::new(r),
                span,
            };
        }
        Ok(e)
    }

    fn parse_not(&mut self) -> Result<Expr, Diagnostic> {
        if self.peek_word() == Some("not".into()) {
            let span = self.span();
            self.bump();
            let e = self.parse_not()?;
            return Ok(Expr::Unary {
                op: UnaryOp::Not,
                expr: Box::new(e),
                span,
            });
        }
        self.parse_cmp()
    }

    fn parse_cmp(&mut self) -> Result<Expr, Diagnostic> {
        let mut e = self.parse_add()?;
        loop {
            let op = if self.peek_word() == Some("is".into()) {
                self.bump();
                self.comparison_op_after_is()
            } else if self.peek_word() == Some("above".into()) {
                self.bump();
                BinaryOp::Gt
            } else if self.peek_word() == Some("below".into()) {
                self.bump();
                BinaryOp::Lt
            } else {
                match self.peek().kind {
                    TokenKind::Assign => return Err(self.err_lone_equals()),
                    TokenKind::Eq => {
                        self.bump();
                        BinaryOp::Eq
                    }
                    TokenKind::Ne => {
                        self.bump();
                        BinaryOp::Ne
                    }
                    TokenKind::Lt => {
                        self.bump();
                        BinaryOp::Lt
                    }
                    TokenKind::Le => {
                        self.bump();
                        BinaryOp::Le
                    }
                    TokenKind::Gt => {
                        self.bump();
                        BinaryOp::Gt
                    }
                    TokenKind::Ge => {
                        self.bump();
                        BinaryOp::Ge
                    }
                    _ => break,
                }
            };
            let span = self.span();
            let r = self.parse_add()?;
            e = Expr::Binary {
                op,
                left: Box::new(e),
                right: Box::new(r),
                span,
            };
        }
        Ok(e)
    }

    fn parse_add(&mut self) -> Result<Expr, Diagnostic> {
        let mut e = self.parse_mul()?;
        loop {
            let op = if self.peek_word() == Some("plus".into()) {
                self.bump();
                BinaryOp::Add
            } else if self.peek_word() == Some("minus".into()) {
                self.bump();
                BinaryOp::Sub
            } else {
                match self.peek().kind {
                    TokenKind::Plus => {
                        self.bump();
                        BinaryOp::Add
                    }
                    TokenKind::Minus => {
                        self.bump();
                        BinaryOp::Sub
                    }
                    _ => break,
                }
            };
            let span = self.span();
            let r = self.parse_mul()?;
            e = Expr::Binary {
                op,
                left: Box::new(e),
                right: Box::new(r),
                span,
            };
        }
        Ok(e)
    }

    fn parse_mul(&mut self) -> Result<Expr, Diagnostic> {
        let mut e = self.parse_unary()?;
        loop {
            let op = if self.peek_word() == Some("times".into()) {
                self.bump();
                BinaryOp::Mul
            } else if self.peek_word() == Some("divided".into()) {
                self.bump();
                let w = self.word()?;
                if w != "by" {
                    return Err(self.err_here("'by' after divided"));
                }
                BinaryOp::Div
            } else {
                match self.peek().kind {
                    TokenKind::Star => {
                        self.bump();
                        BinaryOp::Mul
                    }
                    TokenKind::Slash => {
                        self.bump();
                        BinaryOp::Div
                    }
                    _ => break,
                }
            };
            let span = self.span();
            let r = self.parse_unary()?;
            e = Expr::Binary {
                op,
                left: Box::new(e),
                right: Box::new(r),
                span,
            };
        }
        Ok(e)
    }

    fn parse_unary(&mut self) -> Result<Expr, Diagnostic> {
        if matches!(self.peek().kind, TokenKind::Minus) {
            let span = self.span();
            self.bump();
            let e = self.parse_unary()?;
            return Ok(Expr::Unary {
                op: UnaryOp::Neg,
                expr: Box::new(e),
                span,
            });
        }
        self.parse_postfix()
    }

    fn parse_postfix(&mut self) -> Result<Expr, Diagnostic> {
        let mut e = self.parse_primary()?;
        while self.peek_word() == Some("of".into()) {
            let span = self.span();
            self.bump();
            let arg = self.parse_unary()?;
            if let Expr::Path(p, _) = &e {
                e = Expr::Call {
                    name: p.parts[0].clone(),
                    args: vec![arg],
                    named: Vec::new(),
                    span,
                };
            } else if let Expr::Call {
                name,
                mut args,
                named,
                ..
            } = e
            {
                args.push(arg);
                e = Expr::Call {
                    name,
                    args,
                    named,
                    span,
                };
            }
        }
        Ok(e)
    }

    fn parse_primary(&mut self) -> Result<Expr, Diagnostic> {
        let span = self.span();
        if self.peek_word() == Some("if".into()) {
            self.bump();
            let cond = self.parse_or()?;
            if self.word()? != "then" {
                return Err(self.err_here("'then'"));
            }
            let then_b = self.parse_or()?;
            if self.word()? != "else" {
                return Err(self.err_here("'else'"));
            }
            let else_b = self.parse_or()?;
            return Ok(Expr::If {
                cond: Box::new(cond),
                then_br: Box::new(then_b),
                else_br: Box::new(else_b),
                span,
            });
        }
        if let Some(w) = self.peek_word() {
            if w == "item"
                && matches!(
                    self.tokens.get(self.pos + 2).map(|t| &t.kind),
                    Some(TokenKind::Ident(ref of)) if of == "of"
                )
            {
                // `item 2 of fruits` — 1-based position
                self.bump();
                let index = self.parse_expr_primary()?;
                self.bump(); // of
                let list = self.parse_unary_line()?;
                return Ok(Expr::Call {
                    name: "item".into(),
                    args: vec![index, list],
                    named: Vec::new(),
                    span,
                });
            }
            if matches!(w.as_str(), "count" | "sum" | "min" | "max" | "any")
                && matches!(
                    self.tokens.get(self.pos + 1).map(|t| &t.kind),
                    Some(TokenKind::Ident(ref nx)) if nx != "of"
                )
            {
                self.bump();
                let entity = self.word()?;
                let mut filter = None;
                if self.peek_word() == Some("where".into()) {
                    self.bump();
                    filter = Some(Box::new(self.parse_or()?));
                }
                return Ok(Expr::Aggregate {
                    kind: match w.as_str() {
                        "count" => AggregateKind::Count,
                        "sum" => AggregateKind::Sum,
                        "min" => AggregateKind::Min,
                        "max" => AggregateKind::Max,
                        _ => AggregateKind::Any,
                    },
                    entity,
                    field: None,
                    filter,
                    span,
                });
            }
        }
        match self.peek().kind.clone() {
            TokenKind::Int(_)
            | TokenKind::Decimal(_)
            | TokenKind::String(_)
            | TokenKind::Ident(_) => self.parse_expr_primary(),
            _ => Err(self.err_here("expression")),
        }
    }

    fn parse_expr_primary(&mut self) -> Result<Expr, Diagnostic> {
        let span = self.span();
        match self.bump().kind {
            TokenKind::String(s) if s.contains('{') => self.interpolate(&s, span),
            TokenKind::String(s) | TokenKind::Color(s) => Ok(Expr::Literal(Literal::Text(s), span)),
            TokenKind::Int(n) => Ok(Expr::Literal(Literal::Int(n), span)),
            TokenKind::Decimal(d) => Ok(Expr::Literal(Literal::Decimal(d), span)),
            TokenKind::Ident(s) if s == "true" || s == "false" => {
                Ok(Expr::Literal(Literal::Bool(s == "true"), span))
            }
            TokenKind::Ident(s) => {
                let parts = vec![s];
                if matches!(self.peek().kind, TokenKind::Dot) {
                    let p = self.parse_path_tail(parts, span.clone())?;
                    return Ok(Expr::Path(p, span));
                }
                if self.peek_word() == Some("at".into()) {
                    self.bump();
                    let key = self.parse_expr_primary()?;
                    let field = self.word()?;
                    return Ok(Expr::Index {
                        entity: parts[0].clone(),
                        key: Box::new(key),
                        field,
                        span,
                    });
                }
                Ok(Expr::Path(
                    PathExpr {
                        parts,
                        span: span.clone(),
                    },
                    span,
                ))
            }
            other => Err(Diagnostic::error(
                "P021",
                format!("expected expression, got {:?}", other),
                &self.file,
                self.peek().line,
                self.peek().column,
                snippet_at(&self.source, self.peek().line, self.peek().column, "expr"),
            )),
        }
    }

    /// `"Hi {name}, you have {count of items} items"` → text + expr + text …
    fn interpolate(&mut self, s: &str, span: Span) -> Result<Expr, Diagnostic> {
        let mut parts: Vec<Expr> = Vec::new();
        let mut lit = String::new();
        let mut chars = s.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '{' {
                if chars.peek() == Some(&'{') {
                    chars.next();
                    lit.push('{');
                    continue;
                }
                let mut inner = String::new();
                let mut closed = false;
                for c2 in chars.by_ref() {
                    if c2 == '}' {
                        closed = true;
                        break;
                    }
                    inner.push(c2);
                }
                if !closed {
                    return Err(Diagnostic::error(
                        "P029",
                        "`{` in a string needs a matching `}`",
                        &self.file,
                        span.line,
                        span.column,
                        snippet_at(&self.source, span.line, span.column, "{"),
                    )
                    .with_help("write `\"Hi {name}\"`, or `{{` for a literal brace"));
                }
                if !lit.is_empty() {
                    parts.push(Expr::Literal(
                        Literal::Text(std::mem::take(&mut lit)),
                        span.clone(),
                    ));
                }
                parts.push(self.parse_fragment(inner.trim(), &span)?);
            } else if c == '}' && chars.peek() == Some(&'}') {
                chars.next();
                lit.push('}');
            } else {
                lit.push(c);
            }
        }
        if !lit.is_empty() {
            parts.push(Expr::Literal(Literal::Text(lit), span.clone()));
        }
        // Start with "" so `+` always means text joining.
        let mut e = Expr::Literal(Literal::Text(String::new()), span.clone());
        for p in parts {
            e = Expr::Binary {
                op: BinaryOp::Add,
                left: Box::new(e),
                right: Box::new(p),
                span: span.clone(),
            };
        }
        Ok(e)
    }

    fn parse_fragment(&self, text: &str, span: &Span) -> Result<Expr, Diagnostic> {
        let relocate = |mut d: Diagnostic| {
            d.file = self.file.clone();
            d.line = span.line;
            d.column = span.column;
            d.snippet = snippet_at(&self.source, span.line, span.column, "{");
            d
        };
        let mut lexer = Lexer::new(format!("{text}\n"));
        let tokens = lexer.tokenize(&self.file).map_err(relocate)?;
        let mut tokens: Vec<Token> = tokens
            .into_iter()
            .map(|mut t| {
                t.line = span.line;
                t.column = span.column;
                t
            })
            .collect();
        tokens.retain(|t| !matches!(t.kind, TokenKind::Indent | TokenKind::Dedent));
        let mut sub = Parser {
            file: self.file.clone(),
            source: self.source.clone(),
            tokens,
            pos: 0,
            errors: Vec::new(),
            list_item_mode: false,
            expr_stop_word: None,
        };
        let e = sub.parse_expr_until_line().map_err(relocate)?;
        if !sub.at_physical_line_end() {
            return Err(relocate(sub.err_here("end of `{…}` expression")));
        }
        Ok(e)
    }

    fn parse_path_tail(
        &mut self,
        mut parts: Vec<String>,
        span: Span,
    ) -> Result<PathExpr, Diagnostic> {
        while matches!(self.peek().kind, TokenKind::Dot) {
            self.bump();
            parts.push(self.word()?);
        }
        Ok(PathExpr { parts, span })
    }
}

fn map_compare(op: &str) -> CompareOp {
    match op {
        "above" => CompareOp::Gt,
        "below" => CompareOp::Lt,
        "at" => CompareOp::Ge,
        _ => CompareOp::Gt,
    }
}

#[derive(Debug, Clone, Copy)]
enum CallArgMode {
    RestOfLine,
    SayPart { rest_of_line: bool },
}

fn user_callable_name(name: &str) -> bool {
    !matches!(
        name,
        "is" | "not"
            | "and"
            | "or"
            | "plus"
            | "minus"
            | "times"
            | "divided"
            | "by"
            | "remainder"
            | "above"
            | "below"
            | "more"
            | "less"
            | "than"
            | "fn"
            | "function"
            | "true"
            | "false"
    )
}

fn call_arg_stop_word(word: &str) -> bool {
    matches!(
        word,
        "of" | "from"
            | "contains"
            | "has"
            | "say"
            | "print"
            | "set"
            | "to"
            | "if"
            | "otherwise"
            | "repeat"
            | "for"
            | "while"
            | "give"
            | "spec"
            | "with"
            | "and"
            | "or"
            | "not"
            | "is"
            | "then"
            | "else"
            | "times"
            | "each"
            | "in"
            | "plus"
            | "minus"
            | "divided"
            | "by"
            | "above"
            | "below"
    )
}

fn script_conflicts_with_spec(word: &str) -> bool {
    matches!(
        word,
        "spec"
            | "enum"
            | "config"
            | "state"
            | "entity"
            | "event"
            | "output"
            | "define"
            | "machine"
            | "on"
            | "invariant"
            | "limit"
            | "test"
    )
}
