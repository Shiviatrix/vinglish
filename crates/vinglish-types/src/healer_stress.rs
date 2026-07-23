#![allow(clippy::module_inception)]
/// Stress tests for the MCTS healer.
#[cfg(test)]
mod healer_stress {
    use crate::healer::{Healer, HealingRule, TypeConstraint, try_heal_in_place};
    use vinglish_hir::types::Type;
    use vinglish_lexer::Span;
    use vinglish_parser::ast::{Expr, Literal};

    fn span() -> Span { Span::dummy() }

    fn int_expr(n: i64) -> Expr {
        Expr::Lit { value: Literal::Int(n), span: span() }
    }
    fn bool_expr(b: bool) -> Expr {
        Expr::Lit { value: Literal::Bool(b), span: span() }
    }
    fn text_expr(s: &str) -> Expr {
        Expr::Lit { value: Literal::Text(s.into()), span: span() }
    }
    fn float_expr(f: f64) -> Expr {
        Expr::Lit { value: Literal::Float(f), span: span() }
    }

    fn constraint(expected: Type, actual: Type) -> TypeConstraint {
        TypeConstraint { expected, actual, span: span() }
    }

    // ─── 1. ToText: every non-text type should be healed to text ─────────────

    #[test]
    fn heals_int_to_text() {
        let mut expr = int_expr(42);
        let c = constraint(Type::Text, Type::Int);
        let rule = try_heal_in_place(&Healer, &mut expr, &c, || Ok::<_, ()>(()));
        assert_eq!(rule, Some(HealingRule::ToText));
        assert!(matches!(expr, Expr::Call { .. }), "must wrap in to_text call");
    }

    #[test]
    fn heals_bool_to_text() {
        let mut expr = bool_expr(true);
        let c = constraint(Type::Text, Type::Bool);
        let rule = try_heal_in_place(&Healer, &mut expr, &c, || Ok::<_, ()>(()));
        assert_eq!(rule, Some(HealingRule::ToText));
    }

    #[test]
    fn heals_float_to_text() {
        let mut expr = float_expr(3.14);
        let c = constraint(Type::Text, Type::Float);
        let rule = try_heal_in_place(&Healer, &mut expr, &c, || Ok::<_, ()>(()));
        assert_eq!(rule, Some(HealingRule::ToText));
    }

    // ─── 2. No heal when types already match ─────────────────────────────────

    #[test]
    fn no_heal_when_text_to_text() {
        let mut expr = text_expr("hello");
        let c = constraint(Type::Text, Type::Text);
        let rule = try_heal_in_place(&Healer, &mut expr, &c, || Ok::<_, ()>(()));
        assert_eq!(rule, None, "text-to-text must not generate candidates");
    }

    #[test]
    fn no_heal_when_int_to_int() {
        let mut expr = int_expr(1);
        let c = constraint(Type::Int, Type::Int);
        let rule = try_heal_in_place(&Healer, &mut expr, &c, || Ok::<_, ()>(()));
        assert_eq!(rule, None);
    }

    // ─── 3. No false heal for structural type mismatches ─────────────────────

    #[test]
    fn no_heal_int_expected_bool_actual() {
        let mut expr = bool_expr(false);
        let c = constraint(Type::Int, Type::Bool);
        let rule = try_heal_in_place(&Healer, &mut expr, &c, || Ok::<_, ()>(()));
        // Int ← Bool has no defined rule; must be None
        assert_eq!(rule, None);
    }

    #[test]
    fn no_heal_float_expected_int_actual() {
        let mut expr = int_expr(5);
        let c = constraint(Type::Float, Type::Int);
        let rule = try_heal_in_place(&Healer, &mut expr, &c, || Ok::<_, ()>(()));
        assert_eq!(rule, None);
    }

    // ─── 4. Rollback: recheck always fails → AST must be identical to original

    #[test]
    fn rollback_on_failed_recheck_int_to_text() {
        let original = int_expr(99);
        let mut expr = original.clone();
        let c = constraint(Type::Text, Type::Int);
        // recheck always returns error
        let result = try_heal_in_place(&Healer, &mut expr, &c, || Err::<(), _>(()));
        assert_eq!(result, None);
        assert_eq!(expr, original, "failed recheck must restore original AST node");
    }

    #[test]
    fn rollback_on_failed_recheck_bool_to_text() {
        let original = bool_expr(false);
        let mut expr = original.clone();
        let c = constraint(Type::Text, Type::Bool);
        let result = try_heal_in_place(&Healer, &mut expr, &c, || Err::<(), _>(()));
        assert_eq!(result, None);
        assert_eq!(expr, original);
    }

    // ─── 5. Idempotency: calling twice on the same expression ────────────────

    #[test]
    fn heal_is_idempotent_on_same_input() {
        let original = int_expr(7);
        let c = constraint(Type::Text, Type::Int);

        let mut expr1 = original.clone();
        let rule1 = try_heal_in_place(&Healer, &mut expr1, &c, || Ok::<_, ()>(()));

        let mut expr2 = original.clone();
        let rule2 = try_heal_in_place(&Healer, &mut expr2, &c, || Ok::<_, ()>(()));

        assert_eq!(rule1, rule2, "same input must produce same healing rule");
    }

    // ─── 6. candidates() is pure — no side effects on the expression ─────────

    #[test]
    fn candidates_does_not_mutate_expression() {
        let healer = Healer;
        let expr = int_expr(42);
        let c = constraint(Type::Text, Type::Int);
        let before_ptr = &expr as *const Expr;
        let candidates = healer.candidates(&expr, &c);
        let after_ptr = &expr as *const Expr;
        assert_eq!(before_ptr, after_ptr, "candidates() must not move the expression");
        assert!(!candidates.is_empty(), "must generate at least one candidate");
    }

    // ─── 7. Cost gate: MAX_STEPS = 2, no candidate with cost > 2 passes ──────

    #[test]
    fn healer_constant_max_steps_is_2() {
        assert_eq!(Healer::MAX_STEPS, 2, "MAX_STEPS must be 2 to bound compile time");
    }

    #[test]
    fn all_candidates_have_cost_lte_max_steps() {
        let healer = Healer;
        let exprs_and_constraints = vec![
            (int_expr(1), constraint(Type::Text, Type::Int)),
            (bool_expr(true), constraint(Type::Text, Type::Bool)),
            (float_expr(0.5), constraint(Type::Text, Type::Float)),
        ];
        for (expr, c) in &exprs_and_constraints {
            for candidate in healer.candidates(expr, c) {
                assert!(
                    candidate.cost <= Healer::MAX_STEPS,
                    "candidate cost {} exceeds MAX_STEPS {} for rule {:?}",
                    candidate.cost,
                    Healer::MAX_STEPS,
                    candidate.rule
                );
            }
        }
    }

    // ─── 8. MCTS budget is never exceeded ─────────────────────────────────────

    #[test]
    fn mcts_budget_not_exceeded_under_repeated_healing() {
        // Run 200 iterations to catch any budget overrun (would manifest as
        // stack overflow or hang in a real budget-exceeded scenario)
        let c = constraint(Type::Text, Type::Int);
        for _ in 0..200 {
            let mut expr = int_expr(1);
            let _ = try_heal_in_place(&Healer, &mut expr, &c, || Ok::<_, ()>(()));
        }
    }

    // ─── 9. AutoDeref: reference inner type match ──────────────────────────────

    #[test]
    fn auto_deref_candidate_generated_for_ref_mismatch() {
        let healer = Healer;
        // actual is &Int, expected is Int → AutoDeref candidate
        let expr = int_expr(5);
        let c = TypeConstraint {
            expected: Type::Int,
            actual: Type::Reference(Box::new(Type::Int), false),
            span: span(),
        };
        let candidates = healer.candidates(&expr, &c);
        assert!(
            candidates.iter().any(|cand| cand.rule == HealingRule::AutoDeref),
            "must generate AutoDeref for &T ← T mismatch"
        );
    }

    #[test]
    fn auto_deref_applies_correctly() {
        let mut expr = int_expr(5);
        let c = TypeConstraint {
            expected: Type::Int,
            actual: Type::Reference(Box::new(Type::Int), false),
            span: span(),
        };
        let rule = try_heal_in_place(&Healer, &mut expr, &c, || Ok::<_, ()>(()));
        assert_eq!(rule, Some(HealingRule::AutoDeref));
        // Result should be a UnOp::Deref wrapping the original
        assert!(
            matches!(&expr, Expr::UnOp { op: vinglish_parser::ast::UnOp::Deref, .. }),
            "AutoDeref must wrap in UnOp::Deref"
        );
    }

    // ─── 10. Stress: 10 000 heal calls must complete without panic ────────────

    #[test]
    fn stress_10k_heal_iterations_no_panic() {
        let pairs: Vec<(Expr, TypeConstraint)> = vec![
            (int_expr(0),   constraint(Type::Text,  Type::Int)),
            (bool_expr(true), constraint(Type::Text, Type::Bool)),
            (float_expr(1.0), constraint(Type::Text, Type::Float)),
            (int_expr(1),   constraint(Type::Int,   Type::Int)),   // no-op
            (text_expr("x"), constraint(Type::Text, Type::Text)), // no-op
        ];
        for i in 0..10_000 {
            let (expr, c) = &pairs[i % pairs.len()];
            let mut e = expr.clone();
            let _ = try_heal_in_place(&Healer, &mut e, c, || Ok::<_, ()>(()));
        }
    }
}
