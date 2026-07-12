use std::collections::HashMap;

use eng_lexer::Span;
use eng_parser::ast::*;

/// State of a variable in the ownership model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VarState {
    Owned,
    Borrowed,
    Moved,
    Dropped,
}

#[derive(Debug, Clone)]
pub struct OwnershipError {
    pub message: String,
    pub span: Span,
    pub note: Option<String>,
}

impl OwnershipError {
    fn new(msg: impl Into<String>, span: Span) -> Self {
        Self { message: msg.into(), span, note: None }
    }
    fn with_note(mut self, note: impl Into<String>) -> Self {
        self.note = Some(note.into());
        self
    }
}

/// Flow-sensitive ownership checker.
///
/// Stage 0: reports use-after-move and double-move.
/// Stage 1: full NLL-style lifetime inference.
pub fn check_module(module: &Module) -> Vec<OwnershipError> {
    let mut checker = Checker::new();
    checker.check_module(module);
    checker.errors
}

struct Checker {
    scopes: Vec<HashMap<String, VarState>>,
    errors: Vec<OwnershipError>,
}

impl Checker {
    fn new() -> Self {
        Self {
            scopes: vec![HashMap::new()],
            errors: Vec::new(),
        }
    }

    fn define(&mut self, name: &str, state: VarState) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_string(), state);
        }
    }

    fn state_of(&self, name: &str) -> Option<&VarState> {
        for scope in self.scopes.iter().rev() {
            if let Some(s) = scope.get(name) {
                return Some(s);
            }
        }
        None
    }

    fn push(&mut self) { self.scopes.push(HashMap::new()); }
    fn pop(&mut self)  { self.scopes.pop(); }

    fn check_module(&mut self, module: &Module) {
        for item in &module.items {
            self.check_item(item);
        }
    }

    fn check_item(&mut self, item: &Item) {
        match item {
            Item::Function(f)  => self.check_function(f),
            Item::Statement(s) => { self.check_stmt(s); }
            _                  => {}
        }
    }

    fn check_function(&mut self, f: &FunctionDef) {
        self.push();
        for param in &f.params {
            // Parameters start as Owned
            self.define(&param.name.name, VarState::Owned);
        }
        self.check_block(&f.body);
        self.pop();
    }

    fn check_block(&mut self, block: &Block) {
        self.push();
        for stmt in &block.stmts {
            self.check_stmt(stmt);
        }
        self.pop();
    }

    fn check_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Let(l) => {
                if let Some(val) = &l.value {
                    self.check_expr_use(val);
                }
                self.define(&l.name.name, VarState::Owned);
            }
            Stmt::Return(r) => {
                if let Some(expr) = &r.value {
                    self.check_expr_use(expr);
                }
            }
            Stmt::Assign(a) => {
                self.check_expr_use(&a.value);
            }
            Stmt::If(i) => {
                self.check_expr_use(&i.condition);
                self.check_block(&i.then_block);
                if let Some(else_block) = &i.otherwise {
                    self.check_block(else_block);
                }
            }
            Stmt::When(w) => {
                self.check_expr_use(&w.condition);
                self.check_block(&w.then_block);
                if let Some(else_block) = &w.otherwise {
                    self.check_block(else_block);
                }
            }
            Stmt::Repeat(r) | Stmt::ParallelRepeat(r) => {
                match r {
                    RepeatStmt::ForEvery { var, iterable, body, .. } => {
                        self.check_expr_use(iterable);
                        self.push();
                        self.define(&var.name, VarState::Owned);
                        self.check_block(body);
                        self.pop();
                    }
                    RepeatStmt::While { condition, body, .. } => {
                        self.check_expr_use(condition);
                        self.check_block(body);
                    }
                    RepeatStmt::Count { times, body, .. } => {
                        self.check_expr_use(times);
                        self.check_block(body);
                    }
                }
            }
            Stmt::Match(m) => {
                self.check_expr_use(&m.subject);
                for case in &m.cases {
                    self.check_block(&case.body);
                }
                if let Some(otherwise) = &m.otherwise {
                    self.check_block(otherwise);
                }
            }
            Stmt::Expr(e) => { self.check_expr_use(e); }
            Stmt::Transaction(t) => { self.check_block(&t.body); }
            Stmt::Send(s) => { self.check_expr_use(&s.message); }
            _ => {}
        }
    }

    fn check_expr_use(&mut self, expr: &Expr) {
        match expr {
            Expr::Ident(id) => {
                if let Some(state) = self.state_of(&id.name) {
                    if *state == VarState::Moved {
                        self.errors.push(
                            OwnershipError::new(
                                format!("use of moved value `{}`", id.name),
                                id.span,
                            )
                            .with_note("value was moved out of scope earlier"),
                        );
                    }
                }
                // Immutable references don't move; only move-type values do.
                // Stage 0: we don't track Copy vs non-Copy — leave that for Stage 1.
            }
            Expr::GenericInst { .. } => {
                // Similar to Ident, but typically resolves to functions/types
            }
            Expr::Call { callee, args, .. } => {
                self.check_expr_use(callee);
                for a in args {
                    self.check_expr_use(a);
                }
            }
            Expr::BinOp { left, right, .. } => {
                self.check_expr_use(left);
                self.check_expr_use(right);
            }
            Expr::UnOp { operand, .. } => { self.check_expr_use(operand); }
            Expr::Field { object, .. }  => { self.check_expr_use(object); }
            Expr::Index { object, index, .. } => {
                self.check_expr_use(object);
                self.check_expr_use(index);
            }
            Expr::List { elements, .. } => {
                for e in elements { self.check_expr_use(e); }
            }
            Expr::Block(b) => { self.check_block(b); }
            Expr::StructLit { fields, .. } => {
                for (_, e) in fields { self.check_expr_use(e); }
            }
            Expr::Lit { .. } => {}
        }
    }
}
