//! Deterministic, bounded AST repair candidates for recoverable type constraints.
//!
//! A repair is never guessed: each rule is explicit, has a bounded cost, and is
//! accepted only after the normal type pass succeeds on the rebuilt AST.

use vinglish_lexer::Span;
use vinglish_parser::ast::{Block, Expr, Ident, Item, Module, Stmt, TypeExpr, UnOp};
use crate::{AstNodeId, Type, TypeError};

#[derive(Debug, Clone)]
pub struct TypeConstraint {
    pub expected: Type,
    pub actual: Type,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealingRule { AutoDeref, ToText }

#[derive(Debug, Clone)]
pub struct HealingCandidate {
    pub rule: HealingRule,
    pub replacement: Expr,
    pub cost: u8,
}

#[derive(Debug, Clone)]
pub struct HealingWarning {
    pub rule: HealingRule,
    pub span: Span,
}

/// Pure candidate generator. It does not mutate a tree or suppress diagnostics.
#[derive(Debug, Default)]
pub struct Healer;

impl Healer {
    pub const MAX_STEPS: u8 = 2;

    pub fn candidates(&self, expr: &Expr, constraint: &TypeConstraint) -> Vec<HealingCandidate> {
        let mut candidates = Vec::new();
        if let Type::Reference(inner, _) = &constraint.actual {
            if **inner == constraint.expected {
                candidates.push(HealingCandidate {
                    rule: HealingRule::AutoDeref,
                    replacement: Expr::UnOp { op: UnOp::Deref, operand: Box::new(expr.clone()), span: constraint.span },
                    cost: 1,
                });
            }
        }
        if constraint.expected == Type::Text && constraint.actual != Type::Text {
            let callee = Expr::Ident(Ident::new("to_text", constraint.span));
            candidates.push(HealingCandidate {
                rule: HealingRule::ToText,
                replacement: Expr::Call { callee: Box::new(callee), args: vec![expr.clone()], span: constraint.span },
                cost: 1,
            });
        }
        candidates
    }
}

/// Hook contract for `TypeInferencePass`: it supplies a structured failed
/// constraint and a mutable AST slot; this helper tries candidates in a stable
/// order, restoring the original node after every failed re-check.
pub fn try_heal_in_place<E>(
    healer: &Healer,
    slot: &mut Expr,
    constraint: &TypeConstraint,
    mut recheck: impl FnMut() -> Result<(), E>,
) -> Option<HealingRule> {
    let original = slot.clone();
    for candidate in healer.candidates(&original, constraint) {
        if candidate.cost > Healer::MAX_STEPS { continue; }
        *slot = candidate.replacement;
        if recheck().is_ok() { return Some(candidate.rule); }
        *slot = original.clone();
    }
    None
}

/// Transactionally applies one rule to a cloned module. The caller supplies the
/// ordinary type-pass recheck; only a fully successful recheck commits the AST.
pub fn attempt_heal<E>(
    error: &TypeError,
    ast: &mut Module,
    mut recheck: impl FnMut(&Module) -> Result<(), E>,
) -> Option<HealingWarning> {
    let TypeError::Mismatch { expected, actual, node_id, span } = error else { return None; };
    let constraint = TypeConstraint { expected: expected.clone(), actual: actual.clone(), span: *span };
    let original = ast.clone();
    let expr = find_expr(&original, *node_id)?;
    for candidate in Healer.candidates(expr, &constraint) {
        if candidate.cost > Healer::MAX_STEPS { continue; }
        let mut candidate_ast = original.clone();
        let Some(slot) = find_expr_mut(&mut candidate_ast, *node_id) else { continue; };
        *slot = candidate.replacement;
        if recheck(&candidate_ast).is_ok() {
            *ast = candidate_ast;
            return Some(HealingWarning { rule: candidate.rule, span: *span });
        }
    }
    None
}

fn matches_id(expr: &Expr, id: AstNodeId) -> bool {
    let span = expr.span();
    span.start == id.start && span.end == id.end
}

fn find_expr(module: &Module, id: AstNodeId) -> Option<&Expr> {
    for item in &module.items {
        let block = match item { Item::Function(f) => Some(&f.body), Item::Route(r) => Some(&r.handler), Item::Statement(s) => return find_expr_in_stmt(s, id), _ => None };
        if let Some(block) = block { if let Some(expr) = find_expr_in_block(block, id) { return Some(expr); } }
    }
    None
}

fn find_expr_mut(module: &mut Module, id: AstNodeId) -> Option<&mut Expr> {
    for item in &mut module.items {
        let block = match item { Item::Function(f) => Some(&mut f.body), Item::Route(r) => Some(&mut r.handler), Item::Statement(s) => return find_expr_in_stmt_mut(s, id), _ => None };
        if let Some(block) = block { if let Some(expr) = find_expr_in_block_mut(block, id) { return Some(expr); } }
    }
    None
}

fn find_expr_in_block(block: &Block, id: AstNodeId) -> Option<&Expr> {
    block.stmts.iter().find_map(|stmt| find_expr_in_stmt(stmt, id))
}

fn find_expr_in_block_mut(block: &mut Block, id: AstNodeId) -> Option<&mut Expr> {
    for stmt in &mut block.stmts { if let Some(expr) = find_expr_in_stmt_mut(stmt, id) { return Some(expr); } }
    None
}

fn find_expr_in_stmt(stmt: &Stmt, id: AstNodeId) -> Option<&Expr> {
    match stmt {
        Stmt::Let(s) => s.value.as_ref().and_then(|e| find_expr_in_expr(e, id)),
        Stmt::Return(s) => s.value.as_ref().and_then(|e| find_expr_in_expr(e, id)),
        Stmt::If(s) => find_expr_in_expr(&s.condition, id).or_else(|| find_expr_in_block(&s.then_block, id)).or_else(|| s.otherwise.as_ref().and_then(|b| find_expr_in_block(b, id))),
        Stmt::When(s) => find_expr_in_expr(&s.condition, id).or_else(|| find_expr_in_block(&s.then_block, id)).or_else(|| s.otherwise.as_ref().and_then(|b| find_expr_in_block(b, id))),
        Stmt::Assign(s) => find_expr_in_expr(&s.target, id).or_else(|| find_expr_in_expr(&s.value, id)),
        Stmt::Expr(e) | Stmt::Send(vinglish_parser::ast::SendStmt { message: e, .. }) => find_expr_in_expr(e, id),
        Stmt::Transaction(t) => find_expr_in_block(&t.body, id),
        Stmt::Repeat(r) | Stmt::ParallelRepeat(r) => match r { vinglish_parser::ast::RepeatStmt::ForEvery { iterable, body, .. } => find_expr_in_expr(iterable, id).or_else(|| find_expr_in_block(body, id)), vinglish_parser::ast::RepeatStmt::While { condition, body, .. } => find_expr_in_expr(condition, id).or_else(|| find_expr_in_block(body, id)), vinglish_parser::ast::RepeatStmt::Count { times, body, .. } => find_expr_in_expr(times, id).or_else(|| find_expr_in_block(body, id)) },
        Stmt::Match(m) => find_expr_in_expr(&m.subject, id).or_else(|| m.cases.iter().find_map(|c| find_expr_in_block(&c.body, id))).or_else(|| m.otherwise.as_ref().and_then(|b| find_expr_in_block(b, id))),
        _ => None,
    }
}

fn find_expr_in_stmt_mut(stmt: &mut Stmt, id: AstNodeId) -> Option<&mut Expr> {
    match stmt {
        Stmt::Let(s) => s.value.as_mut().and_then(|e| find_expr_in_expr_mut(e, id)),
        Stmt::Return(s) => s.value.as_mut().and_then(|e| find_expr_in_expr_mut(e, id)),
        Stmt::If(s) => { if let Some(e) = find_expr_in_expr_mut(&mut s.condition, id) { return Some(e); } if let Some(e) = find_expr_in_block_mut(&mut s.then_block, id) { return Some(e); } s.otherwise.as_mut().and_then(|b| find_expr_in_block_mut(b, id)) },
        Stmt::When(s) => { if let Some(e) = find_expr_in_expr_mut(&mut s.condition, id) { return Some(e); } if let Some(e) = find_expr_in_block_mut(&mut s.then_block, id) { return Some(e); } s.otherwise.as_mut().and_then(|b| find_expr_in_block_mut(b, id)) },
        Stmt::Assign(s) => { if let Some(e) = find_expr_in_expr_mut(&mut s.target, id) { return Some(e); } find_expr_in_expr_mut(&mut s.value, id) },
        Stmt::Expr(e) | Stmt::Send(vinglish_parser::ast::SendStmt { message: e, .. }) => find_expr_in_expr_mut(e, id),
        Stmt::Transaction(t) => find_expr_in_block_mut(&mut t.body, id),
        Stmt::Repeat(r) | Stmt::ParallelRepeat(r) => match r { vinglish_parser::ast::RepeatStmt::ForEvery { iterable, body, .. } => { if let Some(e) = find_expr_in_expr_mut(iterable, id) { return Some(e); } find_expr_in_block_mut(body, id) }, vinglish_parser::ast::RepeatStmt::While { condition, body, .. } => { if let Some(e) = find_expr_in_expr_mut(condition, id) { return Some(e); } find_expr_in_block_mut(body, id) }, vinglish_parser::ast::RepeatStmt::Count { times, body, .. } => { if let Some(e) = find_expr_in_expr_mut(times, id) { return Some(e); } find_expr_in_block_mut(body, id) } },
        Stmt::Match(m) => { if let Some(e) = find_expr_in_expr_mut(&mut m.subject, id) { return Some(e); } for case in &mut m.cases { if let Some(e) = find_expr_in_block_mut(&mut case.body, id) { return Some(e); } } m.otherwise.as_mut().and_then(|b| find_expr_in_block_mut(b, id)) },
        _ => None,
    }
}

fn find_expr_in_expr(expr: &Expr, id: AstNodeId) -> Option<&Expr> {
    if matches_id(expr, id) { return Some(expr); }
    match expr { Expr::Call { callee, args, .. } => find_expr_in_expr(callee, id).or_else(|| args.iter().find_map(|e| find_expr_in_expr(e, id))), Expr::BinOp { left, right, .. } => find_expr_in_expr(left, id).or_else(|| find_expr_in_expr(right, id)), Expr::UnOp { operand, .. } | Expr::PostfixTry { inner: operand, .. } => find_expr_in_expr(operand, id), Expr::Field { object, .. } => find_expr_in_expr(object, id), Expr::Index { object, index, .. } => find_expr_in_expr(object, id).or_else(|| find_expr_in_expr(index, id)), Expr::StructLit { ty, fields, .. } => find_expr_in_expr(ty, id).or_else(|| fields.iter().find_map(|(_, e)| find_expr_in_expr(e, id))), Expr::Block(b) => find_expr_in_block(b, id), Expr::List { elements, .. } | Expr::MacroCall { args: elements, .. } => elements.iter().find_map(|e| find_expr_in_expr(e, id)), _ => None }
}

fn find_expr_in_expr_mut(expr: &mut Expr, id: AstNodeId) -> Option<&mut Expr> {
    if matches_id(expr, id) { return Some(expr); }
    match expr { Expr::Call { callee, args, .. } => { if let Some(e) = find_expr_in_expr_mut(callee, id) { return Some(e); } args.iter_mut().find_map(|e| find_expr_in_expr_mut(e, id)) }, Expr::BinOp { left, right, .. } => { if let Some(e) = find_expr_in_expr_mut(left, id) { return Some(e); } find_expr_in_expr_mut(right, id) }, Expr::UnOp { operand, .. } | Expr::PostfixTry { inner: operand, .. } => find_expr_in_expr_mut(operand, id), Expr::Field { object, .. } => find_expr_in_expr_mut(object, id), Expr::Index { object, index, .. } => { if let Some(e) = find_expr_in_expr_mut(object, id) { return Some(e); } find_expr_in_expr_mut(index, id) }, Expr::StructLit { ty, fields, .. } => { if let Some(e) = find_expr_in_expr_mut(ty, id) { return Some(e); } fields.iter_mut().find_map(|(_, e)| find_expr_in_expr_mut(e, id)) }, Expr::Block(b) => find_expr_in_block_mut(b, id), Expr::List { elements, .. } | Expr::MacroCall { args: elements, .. } => elements.iter_mut().find_map(|e| find_expr_in_expr_mut(e, id)), _ => None }
}

/// Intended type-pass integration point (pseudo-call in the unifier error arm):
/// `try_heal_in_place(&healer, ast_slot, &constraint, || recheck_subtree(...))`.
/// The existing pass must first retain `expected`, `actual`, and the AST slot;
/// message-only `TypeError` values are intentionally not eligible for healing.
pub fn type_expr_hint(ty: &Type) -> Option<TypeExpr> {
    match ty { Type::Int => Some(TypeExpr::Named(Ident::new("number", Span::dummy()))), Type::Text => Some(TypeExpr::Named(Ident::new("text", Span::dummy()))), _ => None }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vinglish_parser::ast::Literal;

    #[test]
    fn converts_non_text_expression_when_text_is_required() {
        let mut expr = Expr::Lit { value: Literal::Int(7), span: Span::dummy() };
        let constraint = TypeConstraint { expected: Type::Text, actual: Type::Int, span: Span::dummy() };
        let rule = try_heal_in_place(&Healer, &mut expr, &constraint, || Ok::<_, ()>(()));
        assert_eq!(rule, Some(HealingRule::ToText));
        assert!(matches!(expr, Expr::Call { .. }));
    }

    #[test]
    fn restores_original_when_recheck_fails() {
        let original = Expr::Lit { value: Literal::Int(7), span: Span::dummy() };
        let mut expr = original.clone();
        let constraint = TypeConstraint { expected: Type::Text, actual: Type::Int, span: Span::dummy() };
        assert_eq!(try_heal_in_place(&Healer, &mut expr, &constraint, || Err::<(), _>(())), None);
        assert_eq!(expr, original);
    }
}
