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
        let mut stmts = Vec::new();
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
                        "use only `say`, `print`, `set`, and `name = value` lines, or start the file with `spec` for the optional spec layer",
                    ));
                }
            }
            let stmt_span = self.span();
            let head = self
                .peek_word()
                .ok_or_else(|| self.err_here("a `say`, `print`, `set`, or assignment statement"))?;
            match head.as_str() {
                "say" | "print" => {
                    self.bump();
                    let parts = self.parse_say_parts()?;
                    self.skip_newlines();
                    stmts.push(ScriptStmt::Say {
                        parts,
                        span: stmt_span,
                    });
                }
                "set" => {
                    self.bump();
                    let name = self.word()?;
                    if self.word()? != "to" {
                        return Err(self.err_here("'to' after variable name"));
                    }
                    let value = self.parse_expr_until_line()?;
                    self.skip_newlines();
                    stmts.push(ScriptStmt::Set {
                        name,
                        value,
                        span: stmt_span,
                    });
                }
                _ => {
                    let assign = self
                        .tokens
                        .get(self.pos + 1)
                        .is_some_and(|t| matches!(t.kind, TokenKind::Assign));
                    if !assign {
                        return Err(self
                            .err_here("a `say`, `print`, `set`, or assignment statement")
                            .with_help(
                                "assign with `name = value` or `set name to value`; output with `say` or `print`",
                            ));
                    }
                    let name = head;
                    self.bump();
                    self.bump();
                    let value = self.parse_expr_until_line()?;
                    self.skip_newlines();
                    stmts.push(ScriptStmt::Set {
                        name,
                        value,
                        span: stmt_span,
                    });
                }
            }
        }
        Ok(Script { stmts, span })
    }

    fn parse_say_parts(&mut self) -> Result<Vec<Expr>, Diagnostic> {
        let mut parts = Vec::new();
        while !self.at_line_end() {
            parts.push(self.parse_add_line()?);
        }
        if parts.is_empty() {
            return Err(self.err_here("a value after `say` or `print`"));
        }
        Ok(parts)
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
        self.skip_newlines();
        if matches!(self.peek().kind, TokenKind::Indent) {
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

    fn parse_cmp_line(&mut self) -> Result<Expr, Diagnostic> {
        let mut e = self.parse_add_line()?;
        loop {
            if self.at_line_end() {
                break;
            }
            let op = if self.peek_word() == Some("is".into()) {
                self.bump();
                if self.peek_word() == Some("not".into()) {
                    self.bump();
                    BinaryOp::Ne
                } else {
                    BinaryOp::Eq
                }
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
        while self.peek_word() == Some("of".into()) && !self.at_line_end() {
            let span = self.span();
            self.bump();
            let arg = self.parse_unary_line()?;
            if let Expr::Path(p, _) = &e {
                e = Expr::Call {
                    name: p.parts[0].clone(),
                    args: vec![arg],
                    span,
                };
            }
        }
        Ok(e)
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
                if self.peek_word() == Some("not".into()) {
                    self.bump();
                    BinaryOp::Ne
                } else {
                    BinaryOp::Eq
                }
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
                    span,
                };
            } else if let Expr::Call { name, mut args, .. } = e {
                args.push(arg);
                e = Expr::Call { name, args, span };
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
            if matches!(w.as_str(), "count" | "sum" | "min" | "max" | "any") {
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
            TokenKind::String(s) | TokenKind::Color(s) => Ok(Expr::Literal(Literal::Text(s), span)),
            TokenKind::Int(n) => Ok(Expr::Literal(Literal::Int(n), span)),
            TokenKind::Decimal(d) => Ok(Expr::Literal(Literal::Decimal(d), span)),
            TokenKind::Ident(s) if s == "true" || s == "false" => {
                Ok(Expr::Literal(Literal::Bool(s == "true"), span))
            }
            TokenKind::Ident(s) => {
                let mut parts = vec![s];
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
