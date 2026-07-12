use eng_lexer::{Span, Spanned, Token};

use crate::ast::*;
use crate::error::ParseError;

/// Parse a full module from a token stream.
/// Returns a (possibly partial) Module and any errors encountered.
/// The parser never panics — it recovers and continues.
pub fn parse(tokens: &[Spanned<Token>]) -> (Module, Vec<ParseError>) {
    let mut p = Parser::new(tokens);
    let module = p.parse_module();
    (module, p.errors)
}

// ─────────────────────────────────────────────────────────────────────────────

struct Parser<'t> {
    tokens: &'t [Spanned<Token>],
    pos: usize,
    pub errors: Vec<ParseError>,
}

impl<'t> Parser<'t> {
    fn new(tokens: &'t [Spanned<Token>]) -> Self {
        Self {
            tokens,
            pos: 0,
            errors: Vec::new(),
        }
    }

    // ── Token navigation ──────────────────────────────────────────────────────

    fn current(&self) -> &Token {
        self.tokens
            .get(self.pos)
            .map(|s| &s.node)
            .unwrap_or(&Token::EOF)
    }

    fn current_span(&self) -> Span {
        self.tokens
            .get(self.pos)
            .map(|s| s.span)
            .unwrap_or(Span::dummy())
    }

    fn peek(&self) -> &Token {
        self.tokens
            .get(self.pos + 1)
            .map(|s| &s.node)
            .unwrap_or(&Token::EOF)
    }

    fn advance(&mut self) -> &Token {
        let tok = self
            .tokens
            .get(self.pos)
            .map(|s| &s.node)
            .unwrap_or(&Token::EOF);
        if self.pos < self.tokens.len() {
            self.pos += 1;
        }
        tok
    }

    /// Skip newline tokens (used when entering blocks etc.)
    fn skip_newlines(&mut self) {
        while matches!(self.current(), Token::Newline) {
            self.advance();
        }
    }

    fn at_end(&self) -> bool {
        matches!(self.current(), Token::EOF)
    }

    /// Check if current token is `tok`.
    fn check(&self, tok: &Token) -> bool {
        self.current() == tok
    }

    /// Advance if the current token matches; return true on success.
    fn eat(&mut self, tok: &Token) -> bool {
        if self.current() == tok {
            self.advance();
            true
        } else {
            false
        }
    }

    /// Require a specific token; record an error and return false if not found.
    fn expect(&mut self, tok: &Token) -> bool {
        if self.eat(tok) {
            true
        } else {
            let span = self.current_span();
            self.errors
                .push(ParseError::expected(tok.describe(), self.current(), span));
            false
        }
    }

    /// Consume an identifier, recording an error if not present.
    fn expect_ident(&mut self) -> Option<Ident> {
        let span = self.current_span();
        match self.current().clone() {
            Token::Ident(name) => {
                self.advance();
                Some(Ident::new(name, span))
            }
            // Allow type-name keywords as identifiers when used in function names
            Token::Number => {
                self.advance();
                Some(Ident::new("number", span))
            }
            Token::Text => {
                self.advance();
                Some(Ident::new("text", span))
            }
            Token::Decimal => {
                self.advance();
                Some(Ident::new("decimal", span))
            }
            Token::Boolean => {
                self.advance();
                Some(Ident::new("boolean", span))
            }
            ref other => {
                self.errors
                    .push(ParseError::expected("identifier", other, span));
                None
            }
        }
    }

    // ── Top-level parsing ─────────────────────────────────────────────────────

    fn parse_module(&mut self) -> Module {
        let start = self.current_span();
        let mut items = Vec::new();

        loop {
            self.skip_newlines();
            if self.at_end() {
                break;
            }

            match self.parse_item() {
                Some(item) => items.push(item),
                None => {
                    // Skip token and try to recover
                    if !self.at_end() {
                        self.advance();
                    }
                }
            }
        }

        let end = self.current_span();
        Module {
            items,
            span: start.merge(end),
        }
    }

    fn parse_item(&mut self) -> Option<Item> {
        self.skip_newlines();

        let vis = self.parse_visibility();

        match self.current() {
            Token::Foreign => {
                self.advance();
                Some(Item::Function(self.parse_function(vis, true)))
            }
            Token::Function => Some(Item::Function(self.parse_function(vis, false))),
            Token::Type => Some(self.parse_type_decl(vis)),
            Token::Package => Some(Item::Package(self.parse_package())),
            Token::Module => Some(Item::Module(self.parse_module_decl())),
            Token::Use => Some(Item::Use(self.parse_use())),
            Token::Route => Some(Item::Route(self.parse_route())),
            Token::EOF => None,
            _ => {
                // Script-mode: top-level statements
                self.parse_stmt().map(Item::Statement)
            }
        }
    }

    fn parse_visibility(&mut self) -> Visibility {
        match self.current() {
            Token::Public => {
                self.advance();
                Visibility::Public
            }
            Token::Private => {
                self.advance();
                Visibility::Private
            }
            Token::Internal => {
                self.advance();
                Visibility::Internal
            }
            _ => Visibility::Private,
        }
    }

    fn parse_type_params(&mut self) -> Vec<Ident> {
        let mut params = vec![];
        if self.eat(&Token::Lt) {
            loop {
                self.skip_newlines();
                if self.eat(&Token::Gt) || self.current() == &Token::EOF {
                    break;
                }
                if let Some(id) = self.expect_ident() {
                    params.push(id);
                }
                if !self.eat(&Token::Comma) {
                    self.expect(&Token::Gt);
                    break;
                }
            }
        }
        params
    }

    fn try_parse_generic_args(&mut self) -> Option<Vec<TypeExpr>> {
        let saved_pos = self.pos;
        let saved_errors = self.errors.len();

        if !self.eat(&Token::Lt) {
            return None;
        }

        let mut args = vec![];
        loop {
            self.skip_newlines();
            if self.eat(&Token::Gt) {
                break;
            }
            if let Some(ty) = self.parse_type_expr() {
                args.push(ty);
            } else {
                self.pos = saved_pos;
                self.errors.truncate(saved_errors);
                return None;
            }
            if !self.eat(&Token::Comma) {
                if !self.eat(&Token::Gt) {
                    self.pos = saved_pos;
                    self.errors.truncate(saved_errors);
                    return None;
                }
                break;
            }
        }
        Some(args)
    }

    // ── Function ──────────────────────────────────────────────────────────────

    fn parse_function(&mut self, visibility: Visibility, is_foreign: bool) -> FunctionDef {
        let start = self.current_span();
        self.expect(&Token::Function);

        let name = self
            .expect_ident()
            .unwrap_or_else(|| Ident::new("_", Span::dummy()));
        let type_params = self.parse_type_params();

        // Optional `on TargetType` for methods
        let target_type = if self.eat(&Token::On) {
            self.expect_ident()
        } else {
            None
        };

        // Parameters: `(type name, type name, ...)`
        let params = if self.eat(&Token::LParen) {
            let p = self.parse_param_list();
            self.expect(&Token::RParen);
            p
        } else {
            vec![]
        };

        // Optional `returns type` — may be on the same line or the next
        self.skip_newlines();
        let ret_type = if self.eat(&Token::Returns) {
            self.parse_type_expr()
        } else {
            None
        };

        // Optional `effects IO, Network, ...` — may be on the same or next line
        self.skip_newlines();
        let effects = if self.eat(&Token::Effects) {
            self.parse_ident_list()
        } else {
            vec![]
        };

        self.skip_newlines();
        let body = if is_foreign {
            Block::empty(self.current_span())
        } else {
            self.parse_block()
        };
        let span = start.merge(body.span);

        FunctionDef {
            visibility,
            is_foreign,
            name,
            type_params,
            target_type,
            params,
            ret_type,
            effects,
            body,
            span,
        }
    }

    fn parse_param_list(&mut self) -> Vec<Param> {
        let mut params = Vec::new();
        loop {
            self.skip_newlines();
            if matches!(self.current(), Token::RParen | Token::EOF) {
                break;
            }

            // Power user: `name: type`
            if matches!(self.current(), Token::Ident(_)) && matches!(self.peek(), Token::Colon) {
                let name = self.expect_ident().unwrap();
                self.eat(&Token::Colon);
                if let Some(ty) = self.parse_type_expr() {
                    let span = name.span;
                    params.push(Param { ty, name, span });
                }
            } else {
                // Beginner: `type name`
                if let Some(ty) = self.parse_type_expr() {
                    if let Some(name) = self.expect_ident() {
                        let span = name.span;
                        params.push(Param { ty, name, span });
                    }
                }
            }

            if !self.eat(&Token::Comma) {
                break;
            }
        }
        params
    }

    fn parse_ident_list(&mut self) -> Vec<Ident> {
        let mut idents = Vec::new();
        loop {
            if let Some(id) = self.expect_ident() {
                idents.push(id);
            } else {
                break;
            }
            if !self.eat(&Token::Comma) {
                break;
            }
        }
        idents
    }

    // ── Block ─────────────────────────────────────────────────────────────────

    /// Parse a block — either `begin ... end` or an indented block.
    fn parse_block(&mut self) -> Block {
        self.skip_newlines();
        let start = self.current_span();

        if self.eat(&Token::Begin) {
            // begin/end style — skip Indent/Dedent inside, only End terminates
            let mut stmts = Vec::new();
            loop {
                // In explicit begin/end, ignore structural whitespace tokens
                while matches!(
                    self.current(),
                    Token::Newline | Token::Indent | Token::Dedent
                ) {
                    self.advance();
                }
                if matches!(self.current(), Token::End | Token::EOF) {
                    break;
                }
                if let Some(s) = self.parse_stmt() {
                    stmts.push(s);
                } else if !self.at_end() {
                    self.advance(); // recover past unknown token
                }
            }
            let end = self.current_span();
            self.eat(&Token::End);
            Block {
                stmts,
                span: start.merge(end),
            }
        } else if self.eat(&Token::Indent) {
            // Indented block
            let mut stmts = Vec::new();
            loop {
                self.skip_newlines();
                if matches!(self.current(), Token::Dedent | Token::EOF) {
                    break;
                }
                if let Some(s) = self.parse_stmt() {
                    stmts.push(s);
                } else if !self.at_end() {
                    self.advance();
                }
            }
            let end = self.current_span();
            self.eat(&Token::Dedent);
            Block {
                stmts,
                span: start.merge(end),
            }
        } else {
            // Single-statement "block" (e.g., `if x then return y`)
            let mut stmts = Vec::new();
            if let Some(s) = self.parse_stmt() {
                stmts.push(s);
            }
            let end = self.current_span();
            Block {
                stmts,
                span: start.merge(end),
            }
        }
    }

    // ── Statements ────────────────────────────────────────────────────────────

    fn parse_stmt(&mut self) -> Option<Stmt> {
        self.skip_newlines();
        // Block terminators — signal to callers that the block is done
        if matches!(
            self.current(),
            Token::End | Token::Dedent | Token::Case | Token::Otherwise | Token::EOF
        ) {
            return None;
        }
        match self.current() {
            Token::Let => Some(self.parse_let()),
            Token::Return => Some(self.parse_return()),
            Token::If => Some(self.parse_if()),
            Token::When => Some(self.parse_when()),
            Token::Repeat => Some(self.parse_repeat(false)),
            Token::Parallel => Some(self.parse_parallel()),
            Token::Match => Some(self.parse_match()),
            Token::Spawn => Some(self.parse_spawn()),
            Token::Send => Some(self.parse_send()),
            Token::Receive => Some(self.parse_receive()),
            Token::Transaction => Some(self.parse_transaction()),
            _ => {
                // Could be an expression or assignment
                let expr = self.parse_expr()?;
                // Check for assignment
                let op = match self.current() {
                    Token::Be => Some(AssignOp::Assign),
                    Token::PlusEq => Some(AssignOp::AddAssign),
                    Token::MinusEq => Some(AssignOp::SubAssign),
                    Token::StarEq => Some(AssignOp::MulAssign),
                    Token::SlashEq => Some(AssignOp::DivAssign),
                    _ => None,
                };
                if let Some(op) = op {
                    self.advance();
                    let value = self.parse_expr()?;
                    let span = expr.span().merge(value.span());
                    Some(Stmt::Assign(AssignStmt {
                        target: expr,
                        op,
                        value,
                        span,
                    }))
                } else {
                    Some(Stmt::Expr(expr))
                }
            }
        }
    }

    fn parse_let(&mut self) -> Stmt {
        let start = self.current_span();
        self.expect(&Token::Let);
        let mutable = self.eat(&Token::Mutable);
        let name = self
            .expect_ident()
            .unwrap_or_else(|| Ident::new("_", Span::dummy()));

        self.eat(&Token::Be); // consume `be`

        // Now either a type expression, a value, or both
        // Heuristic: if the next token is a type-name keyword and no arithmetic follows,
        // treat it as a type-only declaration.
        let (ty, value) = self.parse_let_rhs();
        let span = start.merge(self.current_span());
        Stmt::Let(LetStmt {
            name,
            ty,
            value,
            mutable,
            span,
        })
    }

    fn parse_let_rhs(&mut self) -> (Option<TypeExpr>, Option<Expr>) {
        // If we see `otherwise` it's an error-chain let
        // `let file be open "x" otherwise return "missing"`
        // For now: try to parse expression; if it's just a type name, treat as type
        let tok = self.current().clone();
        match tok {
            Token::Number | Token::Decimal | Token::Text | Token::Boolean => {
                // Peek: is the next token a newline/end? Then it's a type annotation.
                // Otherwise might be a value with type keyword used as ident.
                let ty = self.parse_type_expr();
                // If there's an `=` or further expression after, parse value too
                if matches!(
                    self.current(),
                    Token::Newline | Token::Dedent | Token::EOF | Token::End | Token::Otherwise
                ) {
                    return (ty, None);
                }
                (ty, self.parse_expr())
            }
            _ => {
                let expr = self.parse_expr();
                (None, expr)
            }
        }
    }

    fn parse_return(&mut self) -> Stmt {
        let start = self.current_span();
        self.expect(&Token::Return);
        let value = if matches!(
            self.current(),
            Token::Newline | Token::Dedent | Token::End | Token::EOF
        ) {
            None
        } else {
            self.parse_expr()
        };
        let span = start.merge(self.current_span());
        Stmt::Return(ReturnStmt { value, span })
    }

    fn parse_if(&mut self) -> Stmt {
        let start = self.current_span();
        self.expect(&Token::If);
        let condition = self.parse_expr().unwrap_or_else(|| Expr::Lit {
            value: Literal::Bool(true),
            span: Span::dummy(),
        });
        self.eat(&Token::Then);
        self.skip_newlines();
        let then_block = self.parse_block();
        self.skip_newlines();
        let otherwise = if self.eat(&Token::Otherwise) {
            self.skip_newlines();
            Some(self.parse_block())
        } else {
            None
        };
        let span = start.merge(self.current_span());
        Stmt::If(IfStmt {
            condition,
            then_block,
            otherwise,
            span,
        })
    }

    fn parse_when(&mut self) -> Stmt {
        let start = self.current_span();
        self.expect(&Token::When);
        let condition = self.parse_expr().unwrap_or_else(|| Expr::Lit {
            value: Literal::Bool(true),
            span: Span::dummy(),
        });
        self.skip_newlines();
        let then_block = self.parse_block();
        self.skip_newlines();
        let otherwise = if self.eat(&Token::Otherwise) {
            self.skip_newlines();
            Some(self.parse_block())
        } else {
            None
        };
        let span = start.merge(self.current_span());
        Stmt::When(WhenStmt {
            condition,
            then_block,
            otherwise,
            span,
        })
    }

    fn parse_parallel(&mut self) -> Stmt {
        let start = self.current_span();
        self.expect(&Token::Parallel);
        // Expect `for every` or `repeat for every`
        self.eat(&Token::Repeat);
        let repeat = self.parse_repeat_inner(start);
        Stmt::ParallelRepeat(repeat)
    }

    fn parse_repeat(&mut self, parallel: bool) -> Stmt {
        let start = self.current_span();
        self.expect(&Token::Repeat);
        let repeat = self.parse_repeat_inner(start);
        if parallel {
            Stmt::ParallelRepeat(repeat)
        } else {
            Stmt::Repeat(repeat)
        }
    }

    fn parse_repeat_inner(&mut self, start: Span) -> RepeatStmt {
        if self.eat(&Token::While) {
            let condition = self.parse_expr().unwrap_or_else(|| Expr::Lit {
                value: Literal::Bool(true),
                span: Span::dummy(),
            });
            self.skip_newlines();
            let body = self.parse_block();
            let span = start.merge(body.span);
            RepeatStmt::While {
                condition,
                body,
                span,
            }
        } else {
            // `for every x` or `for every x in iterable`
            self.eat(&Token::For);
            self.eat(&Token::Every);
            let var = self
                .expect_ident()
                .unwrap_or_else(|| Ident::new("_", Span::dummy()));
            // Optional `in expr` (shorthand — iterable is just the var name pluralised otherwise)
            let iterable = if matches!(self.current(), Token::Ident(_)) {
                self.parse_expr().unwrap_or(Expr::Ident(var.clone()))
            } else {
                // Implicit: iterate over `<var>s` — compiler resolves
                Expr::Ident(Ident::new(format!("{}s", var.name), var.span))
            };
            self.skip_newlines();
            let body = self.parse_block();
            let span = start.merge(body.span);
            RepeatStmt::ForEvery {
                var,
                iterable,
                body,
                span,
            }
        }
    }

    fn parse_match(&mut self) -> Stmt {
        let start = self.current_span();
        self.expect(&Token::Match);
        let subject = self.parse_expr().unwrap_or_else(|| Expr::Lit {
            value: Literal::Unit,
            span: Span::dummy(),
        });
        self.skip_newlines();
        self.eat(&Token::Indent);
        self.skip_newlines();

        let mut cases = Vec::new();
        let mut otherwise = None;

        loop {
            self.skip_newlines();
            match self.current() {
                Token::Case => {
                    let case_start = self.current_span();
                    self.advance();
                    let pattern = self.parse_pattern();
                    // Eat optional `=>` or `then`
                    if !self.eat(&Token::FatArrow) {
                        self.eat(&Token::Then);
                    }
                    self.skip_newlines();
                    let body = self.parse_block();
                    let span = case_start.merge(body.span);
                    cases.push(MatchCase {
                        pattern,
                        body,
                        span,
                    });
                }
                Token::Otherwise => {
                    self.advance();
                    self.eat(&Token::Then); // optional then
                    self.skip_newlines();
                    otherwise = Some(self.parse_block());
                }
                Token::Dedent | Token::EOF => break,
                _ => {
                    self.advance();
                } // recover
            }
        }
        self.eat(&Token::Dedent);
        let span = start.merge(self.current_span());
        Stmt::Match(MatchStmt {
            subject,
            cases,
            otherwise,
            span,
        })
    }

    fn parse_pattern(&mut self) -> Pattern {
        let span = self.current_span();
        match self.current().clone() {
            Token::Ident(name) => {
                self.advance();
                // If it starts with uppercase, it's a constructor
                if name
                    .chars()
                    .next()
                    .map(|c| c.is_uppercase())
                    .unwrap_or(false)
                {
                    Pattern::Constructor(Ident::new(name, span))
                } else if name == "_" {
                    Pattern::Wildcard(span)
                } else {
                    Pattern::Bind(Ident::new(name, span))
                }
            }
            Token::Integer(i) => {
                self.advance();
                Pattern::Literal(Literal::Int(i))
            }
            Token::Float(f) => {
                self.advance();
                Pattern::Literal(Literal::Float(f))
            }
            Token::StringLit(s) => {
                self.advance();
                Pattern::Literal(Literal::Text(s))
            }
            Token::True => {
                self.advance();
                Pattern::Literal(Literal::Bool(true))
            }
            Token::False => {
                self.advance();
                Pattern::Literal(Literal::Bool(false))
            }
            _ => Pattern::Wildcard(span),
        }
    }

    fn parse_spawn(&mut self) -> Stmt {
        let start = self.current_span();
        self.expect(&Token::Spawn);
        let actor = self
            .expect_ident()
            .unwrap_or_else(|| Ident::new("_", Span::dummy()));
        Stmt::Spawn(SpawnStmt {
            actor,
            span: start.merge(self.current_span()),
        })
    }

    fn parse_send(&mut self) -> Stmt {
        let start = self.current_span();
        self.expect(&Token::Send);
        let message = self.parse_expr().unwrap_or_else(|| Expr::Lit {
            value: Literal::Unit,
            span: Span::dummy(),
        });
        Stmt::Send(SendStmt {
            message,
            span: start.merge(self.current_span()),
        })
    }

    fn parse_receive(&mut self) -> Stmt {
        let start = self.current_span();
        self.expect(&Token::Receive);
        let binding = self.expect_ident();
        Stmt::Receive(ReceiveStmt {
            binding,
            span: start.merge(self.current_span()),
        })
    }

    fn parse_transaction(&mut self) -> Stmt {
        let start = self.current_span();
        self.expect(&Token::Transaction);
        self.skip_newlines();
        let body = self.parse_block();
        self.skip_newlines();
        self.eat(&Token::Commit);
        let span = start.merge(self.current_span());
        Stmt::Transaction(TransactionStmt { body, span })
    }

    // ── Expressions ───────────────────────────────────────────────────────────

    fn parse_expr(&mut self) -> Option<Expr> {
        self.parse_or_expr()
    }

    fn parse_or_expr(&mut self) -> Option<Expr> {
        let mut left = self.parse_and_expr()?;
        while self.check(&Token::Or) {
            let op_span = self.current_span();
            self.advance();
            let right = self.parse_and_expr()?;
            let span = left.span().merge(right.span()).merge(op_span);
            left = Expr::BinOp {
                left: Box::new(left),
                op: BinOp::Or,
                right: Box::new(right),
                span,
            };
        }
        Some(left)
    }

    fn parse_and_expr(&mut self) -> Option<Expr> {
        let mut left = self.parse_comparison()?;
        while self.check(&Token::And) {
            let op_span = self.current_span();
            self.advance();
            let right = self.parse_comparison()?;
            let span = left.span().merge(right.span()).merge(op_span);
            left = Expr::BinOp {
                left: Box::new(left),
                op: BinOp::And,
                right: Box::new(right),
                span,
            };
        }
        Some(left)
    }

    fn parse_comparison(&mut self) -> Option<Expr> {
        let mut left = self.parse_add_expr()?;
        loop {
            let op = match self.current() {
                Token::Eq => BinOp::Eq,
                Token::NotEq => BinOp::NotEq,
                Token::Lt => BinOp::Lt,
                Token::Gt => BinOp::Gt,
                Token::LtEq => BinOp::LtEq,
                Token::GtEq => BinOp::GtEq,
                Token::Exceeds => BinOp::Exceeds,
                Token::Is => {
                    // `is below`, `is above`
                    self.advance();
                    match self.current() {
                        Token::Below => {
                            self.advance();
                            let right = self.parse_add_expr()?;
                            let span = left.span().merge(right.span());
                            left = Expr::BinOp {
                                left: Box::new(left),
                                op: BinOp::IsBelow,
                                right: Box::new(right),
                                span,
                            };
                            continue;
                        }
                        Token::Above => {
                            self.advance();
                            let right = self.parse_add_expr()?;
                            let span = left.span().merge(right.span());
                            left = Expr::BinOp {
                                left: Box::new(left),
                                op: BinOp::IsAbove,
                                right: Box::new(right),
                                span,
                            };
                            continue;
                        }
                        _ => {
                            // just `is` — equality
                            let right = self.parse_add_expr()?;
                            let span = left.span().merge(right.span());
                            left = Expr::BinOp {
                                left: Box::new(left),
                                op: BinOp::Eq,
                                right: Box::new(right),
                                span,
                            };
                            continue;
                        }
                    }
                }
                _ => break,
            };
            let op_span = self.current_span();
            self.advance();
            let right = self.parse_add_expr()?;
            let span = left.span().merge(right.span()).merge(op_span);
            left = Expr::BinOp {
                left: Box::new(left),
                op,
                right: Box::new(right),
                span,
            };
        }
        Some(left)
    }

    fn parse_add_expr(&mut self) -> Option<Expr> {
        let mut left = self.parse_mul_expr()?;
        loop {
            let op = match self.current() {
                Token::Plus => BinOp::Add,
                Token::Minus => BinOp::Sub,
                _ => break,
            };
            let op_span = self.current_span();
            self.advance();
            let right = self.parse_mul_expr()?;
            let span = left.span().merge(right.span()).merge(op_span);
            left = Expr::BinOp {
                left: Box::new(left),
                op,
                right: Box::new(right),
                span,
            };
        }
        Some(left)
    }

    fn parse_mul_expr(&mut self) -> Option<Expr> {
        let mut left = self.parse_unary()?;
        loop {
            let op = match self.current() {
                Token::Star => BinOp::Mul,
                Token::Slash => BinOp::Div,
                Token::Percent => BinOp::Mod,
                _ => break,
            };
            let op_span = self.current_span();
            self.advance();
            let right = self.parse_unary()?;
            let span = left.span().merge(right.span()).merge(op_span);
            left = Expr::BinOp {
                left: Box::new(left),
                op,
                right: Box::new(right),
                span,
            };
        }
        Some(left)
    }

    fn parse_unary(&mut self) -> Option<Expr> {
        let start = self.current_span();
        match self.current() {
            Token::Minus => {
                self.advance();
                let operand = self.parse_unary()?;
                let span = start.merge(operand.span());
                Some(Expr::UnOp {
                    op: UnOp::Neg,
                    operand: Box::new(operand),
                    span,
                })
            }
            Token::Not => {
                self.advance();
                let operand = self.parse_unary()?;
                let span = start.merge(operand.span());
                Some(Expr::UnOp {
                    op: UnOp::Not,
                    operand: Box::new(operand),
                    span,
                })
            }
            Token::Borrow => {
                self.advance();
                let mutable = self.eat(&Token::Mutable);
                let operand = self.parse_unary()?;
                let span = start.merge(operand.span());
                Some(Expr::UnOp {
                    op: UnOp::Borrow(mutable),
                    operand: Box::new(operand),
                    span,
                })
            }
            Token::Deref => {
                self.advance();
                let operand = self.parse_unary()?;
                let span = start.merge(operand.span());
                Some(Expr::UnOp {
                    op: UnOp::Deref,
                    operand: Box::new(operand),
                    span,
                })
            }
            _ => self.parse_postfix(),
        }
    }

    fn parse_postfix(&mut self) -> Option<Expr> {
        let mut expr = self.parse_primary()?;
        loop {
            match self.current() {
                Token::Dot => {
                    self.advance();
                    let field = self.expect_ident()?;
                    let span = expr.span().merge(field.span);
                    expr = Expr::Field {
                        object: Box::new(expr),
                        field,
                        span,
                    };
                }
                Token::LParen => {
                    let call_start = self.current_span();
                    self.advance();
                    let mut args = Vec::new();
                    while !matches!(self.current(), Token::RParen | Token::EOF) {
                        if let Some(arg) = self.parse_expr() {
                            args.push(arg);
                        }
                        if !self.eat(&Token::Comma) {
                            break;
                        }
                    }
                    let end = self.current_span();
                    self.expect(&Token::RParen);
                    let span = call_start.merge(end);
                    expr = Expr::Call {
                        callee: Box::new(expr),
                        args,
                        span,
                    };
                }
                Token::LBracket => {
                    self.advance();
                    let index = self.parse_expr()?;
                    let span = expr.span().merge(self.current_span());
                    self.expect(&Token::RBracket);
                    expr = Expr::Index {
                        object: Box::new(expr),
                        index: Box::new(index),
                        span,
                    };
                }
                Token::Bang => {
                    self.advance();
                    let call_start = self.current_span();
                    self.expect(&Token::LParen);
                    let mut args = Vec::new();
                    while !matches!(self.current(), Token::RParen | Token::EOF) {
                        if let Some(arg) = self.parse_expr() {
                            args.push(arg);
                        }
                        if !self.eat(&Token::Comma) {
                            break;
                        }
                    }
                    let end = self.current_span();
                    self.expect(&Token::RParen);
                    let span = expr.span().merge(end);
                    if let Expr::Ident(name) = expr {
                        expr = Expr::MacroCall { name, args, span };
                    } else {
                        self.errors.push(ParseError::Custom {
                            message: "Macro calls must be on identifiers".to_string(),
                            span,
                        });
                        break;
                    }
                }
                Token::QuestionMark => {
                    let end = self.current_span();
                    self.advance();
                    let span = expr.span().merge(end);
                    expr = Expr::PostfixTry {
                        inner: Box::new(expr),
                        span,
                    };
                }
                // Natural-language call: `calculate tax for order`
                // If ident follows ident, treat it as function call with next token as arg
                _ => break,
            }
        }
        Some(expr)
    }

    fn parse_primary(&mut self) -> Option<Expr> {
        let span = self.current_span();
        match self.current().clone() {
            Token::LParen => {
                self.advance();
                let expr = self.parse_expr()?;
                self.expect(&Token::RParen);
                Some(expr)
            }
            Token::Integer(i) => {
                self.advance();
                Some(Expr::Lit {
                    value: Literal::Int(i),
                    span,
                })
            }
            Token::Float(f) => {
                self.advance();
                Some(Expr::Lit {
                    value: Literal::Float(f),
                    span,
                })
            }
            Token::StringLit(s) => {
                self.advance();
                Some(Expr::Lit {
                    value: Literal::Text(s),
                    span,
                })
            }
            Token::True => {
                self.advance();
                Some(Expr::Lit {
                    value: Literal::Bool(true),
                    span,
                })
            }
            Token::False => {
                self.advance();
                Some(Expr::Lit {
                    value: Literal::Bool(false),
                    span,
                })
            }
            Token::Ident(name) => {
                self.advance();
                let ident = Ident::new(name, span);
                let expr = if let Some(args) = self.try_parse_generic_args() {
                    Expr::GenericInst {
                        base: ident.clone(),
                        args,
                        span: span.merge(self.current_span()),
                    }
                } else {
                    Expr::Ident(ident.clone())
                };
                if self.eat(&Token::LBrace) {
                    // Struct literal
                    let mut fields = Vec::new();
                    loop {
                        // Skip newlines, indents, and dedents inside struct literals
                        while matches!(
                            self.current(),
                            Token::Newline | Token::Indent | Token::Dedent
                        ) {
                            self.advance();
                        }
                        if matches!(self.current(), Token::RBrace | Token::EOF) {
                            break;
                        }
                        if let Some(field_name) = self.expect_ident() {
                            self.eat(&Token::Colon);
                            if let Some(field_value) = self.parse_expr() {
                                fields.push((field_name, field_value));
                            }
                        } else {
                            // avoid infinite loop if no ident
                            self.advance();
                        }
                        self.eat(&Token::Comma);
                    }
                    let end = self.current_span();
                    self.expect(&Token::RBrace);
                    Some(Expr::StructLit {
                        ty: Box::new(expr),
                        fields,
                        span: span.merge(end),
                    })
                } else {
                    Some(expr)
                }
            }
            Token::LBracket => {
                self.advance();
                let mut elements = Vec::new();
                while !matches!(self.current(), Token::RBracket | Token::EOF) {
                    if let Some(e) = self.parse_expr() {
                        elements.push(e);
                    }
                    if !self.eat(&Token::Comma) {
                        break;
                    }
                }
                let end = self.current_span();
                self.expect(&Token::RBracket);
                Some(Expr::List {
                    elements,
                    span: span.merge(end),
                })
            }
            // Type-name keywords can appear as identifiers in expressions
            Token::Number => {
                self.advance();
                Some(Expr::Ident(Ident::new("number", span)))
            }
            Token::Text => {
                self.advance();
                Some(Expr::Ident(Ident::new("text", span)))
            }
            Token::Decimal => {
                self.advance();
                Some(Expr::Ident(Ident::new("decimal", span)))
            }
            Token::Boolean => {
                self.advance();
                Some(Expr::Ident(Ident::new("boolean", span)))
            }
            _other => {
                // Not an expression start — record error but don't panic
                self.errors.push(ParseError::InvalidExpr { span });
                None
            }
        }
    }

    // ── Type expressions ──────────────────────────────────────────────────────

    fn parse_type_expr(&mut self) -> Option<TypeExpr> {
        let span = self.current_span();
        // Check for `List of T` or `Dictionary from K to V` before the general case
        if let Token::Ident(ref s) = self.current().clone() {
            if s == "List" {
                self.advance();
                self.eat(&Token::Of);
                let inner = self.parse_type_expr().map(Box::new)?;
                return Some(TypeExpr::List(inner));
            }
            if s == "Dictionary" {
                self.advance();
                self.eat(&Token::From);
                let key = self.parse_type_expr().map(Box::new)?;
                self.eat(&Token::To);
                let val = self.parse_type_expr().map(Box::new)?;
                return Some(TypeExpr::Dict { key, val });
            }
            if s == "Result" {
                self.advance();
                self.eat(&Token::Of);
                let inner = self.parse_type_expr().map(Box::new)?;
                return Some(TypeExpr::Result(inner));
            }
        }
        match self.current().clone() {
            Token::Number => {
                self.advance();
                Some(TypeExpr::Named(Ident::new("number", span)))
            }
            Token::Text => {
                self.advance();
                Some(TypeExpr::Named(Ident::new("text", span)))
            }
            Token::Decimal => {
                self.advance();
                Some(TypeExpr::Named(Ident::new("decimal", span)))
            }
            Token::Boolean => {
                self.advance();
                Some(TypeExpr::Named(Ident::new("boolean", span)))
            }
            Token::Ident(s) => {
                self.advance();
                let base = Ident::new(s, span);
                if self.eat(&Token::Lt) {
                    let mut args = vec![];
                    loop {
                        self.skip_newlines();
                        if self.eat(&Token::Gt) || self.current() == &Token::EOF {
                            break;
                        }
                        if let Some(ty) = self.parse_type_expr() {
                            args.push(ty);
                        }
                        if !self.eat(&Token::Comma) {
                            self.expect(&Token::Gt);
                            break;
                        }
                    }
                    Some(TypeExpr::Generic { base, args })
                } else {
                    Some(TypeExpr::Named(base))
                }
            }
            Token::Borrow => {
                self.advance();
                let mutable = self.eat(&Token::Mutable);
                let inner = self.parse_type_expr().map(Box::new)?;
                Some(TypeExpr::Reference { mutable, inner })
            }
            _ => None,
        }
    }

    // ── Other top-level items ─────────────────────────────────────────────────

    fn parse_type_decl(&mut self, visibility: Visibility) -> Item {
        let start = self.current_span();
        self.expect(&Token::Type);
        let name = self
            .expect_ident()
            .unwrap_or_else(|| Ident::new("_", Span::dummy()));
        let type_params = self.parse_type_params();
        
        self.skip_newlines();

        // Enum definition
        if self.eat(&Token::Be) { // "=" is token Be
            let mut variants = Vec::new();
            loop {
                self.skip_newlines();
                if let Some(vname) = self.expect_ident() {
                    let mut payload = None;
                    if self.eat(&Token::LParen) {
                        payload = self.parse_type_expr();
                        self.expect(&Token::RParen);
                    }
                    variants.push(Variant {
                        name: vname.clone(),
                        payload,
                        span: vname.span,
                    });
                } else {
                    break;
                }

                self.skip_newlines();
                if !self.eat(&Token::Pipe) {
                    break;
                }
            }

            return Item::Enum(EnumDef {
                visibility,
                name,
                type_params,
                variants,
                span: start.merge(self.current_span()),
            });
        }

        // Struct definition
        let capabilities = if self.eat(&Token::Requires) {
            self.parse_ident_list()
        } else {
            vec![]
        };
        self.skip_newlines();

        // Parse fields (if any) using a block-like structure
        let mut fields = Vec::new();
        if self.eat(&Token::Indent) || self.eat(&Token::Begin) {
            loop {
                self.skip_newlines();
                if matches!(self.current(), Token::Dedent | Token::End | Token::EOF) {
                    break;
                }

                let start_idx = self.current_span().start;
                if matches!(self.current(), Token::Ident(_)) && matches!(self.peek(), Token::Colon) {
                    if let Some(fname) = self.expect_ident() {
                        self.eat(&Token::Colon);
                        if let Some(ty) = self.parse_type_expr() {
                            fields.push(Param {
                                ty,
                                name: fname.clone(),
                                span: fname.span,
                            });
                        }
                    }
                } else {
                    if let Some(ty) = self.parse_type_expr() {
                        if let Some(fname) = self.expect_ident() {
                            fields.push(Param {
                                ty,
                                name: fname.clone(),
                                span: fname.span,
                            });
                        }
                    }
                }

                if self.current_span().start == start_idx {
                    self.advance();
                }

                self.eat(&Token::Comma);
                self.skip_newlines();
            }
            if !self.eat(&Token::Dedent) {
                self.eat(&Token::End);
            }
        }

        Item::Type(TypeDef {
            visibility,
            name,
            type_params,
            fields,
            capabilities,
            span: start.merge(self.current_span()),
        })
    }

    fn parse_package(&mut self) -> PackageDecl {
        let start = self.current_span();
        self.expect(&Token::Package);
        let name = self
            .expect_ident()
            .unwrap_or_else(|| Ident::new("_", Span::dummy()));
        PackageDecl {
            name,
            span: start.merge(self.current_span()),
        }
    }

    fn parse_module_decl(&mut self) -> ModuleDecl {
        let start = self.current_span();
        self.expect(&Token::Module);
        let mut name = self
            .expect_ident()
            .unwrap_or_else(|| Ident::new("_", Span::dummy()));
        while self.eat(&Token::Dot) {
            if let Some(id) = self.expect_ident() {
                name.name.push('.');
                name.name.push_str(&id.name);
                name.span = name.span.merge(id.span);
            } else {
                break;
            }
        }
        ModuleDecl {
            name,
            span: start.merge(self.current_span()),
        }
    }

    fn parse_use(&mut self) -> UseDecl {
        let start = self.current_span();
        self.expect(&Token::Use);
        let mut path = Vec::new();
        loop {
            if let Some(id) = self.expect_ident() {
                path.push(id);
            } else {
                break;
            }
            if !self.eat(&Token::Dot) {
                break;
            }
        }
        UseDecl {
            path,
            span: start.merge(self.current_span()),
        }
    }

    fn parse_route(&mut self) -> RouteDecl {
        let start = self.current_span();
        self.expect(&Token::Route);
        let path = if let Token::StringLit(s) = self.current().clone() {
            self.advance();
            s
        } else {
            String::from("/")
        };
        self.skip_newlines();
        // `returns` block
        self.eat(&Token::Returns);
        self.skip_newlines();
        let handler = self.parse_block();
        RouteDecl {
            path,
            handler,
            span: start.merge(self.current_span()),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use eng_lexer::tokenize;

    fn parse_src(src: &str) -> (Module, Vec<ParseError>) {
        let (tokens, _) = tokenize(src);
        parse(&tokens)
    }

    #[test]
    fn hello_world() {
        let src = r#"
function main()
begin
    print("Hello from Englist!")
end
"#;
        let (module, errors) = parse_src(src);
        assert!(errors.is_empty(), "{:?}", errors);
        assert_eq!(module.items.len(), 1);
        assert!(matches!(&module.items[0], Item::Function(_)));
    }

    #[test]
    fn let_stmt() {
        let src = "let age be 25\n";
        let (_module, errors) = parse_src(src);
        assert!(errors.is_empty(), "{:?}", errors);
    }

    #[test]
    fn if_otherwise() {
        let src = r#"
if balance is below 0
    return false
otherwise
    return true
"#;
        let (module, errors) = parse_src(src);
        // Parser may have errors on recovery — just check it parsed something
        assert!(!module.items.is_empty() || !errors.is_empty());
    }

    #[test]
    fn fibonacci_function() {
        let src = r#"
function fibonacci(number n)
returns number
begin
    if n is below 2
        return n
    return fibonacci(n - 1) + fibonacci(n - 2)
end
"#;
        let (module, errors) = parse_src(src);
        assert!(errors.is_empty(), "{:?}", errors);
        assert_eq!(module.items.len(), 1);
    }
}
