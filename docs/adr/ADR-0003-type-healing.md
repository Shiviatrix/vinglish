# ADR-0003: Bounded AST Repair for Type Mismatches

## Status

Accepted.

## Context

Type mismatches between references and values, and between numeric types and text, are common programmer errors. Rather than always requiring explicit conversions, the compiler can attempt automatic repairs.

## Decision

The compiler includes a "type healer" that attempts bounded AST transformations when a type mismatch is detected during inference. The healer is implemented in `vinglish-types/src/healer.rs`.

Two rules exist:
1. `AutoDeref`: When expected `T` but actual is `Reference(T, _)`, insert a deref.
2. `ToText`: When expected `Text` but actual is any other type, insert a `to_text()` call.

The healing process:
1. Generate candidate transformations.
2. Shuffle candidates randomly.
3. For each candidate (up to 100 rollouts):
   - Clone the AST.
   - Apply the transformation.
   - Re-run type checking.
   - If it passes, commit the modified AST.
4. Emit warning `T1001` for successful healings.

Evidence from the codebase:
- `Healer::MAX_STEPS` is 2. Candidates with cost > 2 are skipped.
- `attempt_heal()` clones the entire `Module` for each trial.
- `try_heal_in_place()` provides an alternative API that mutates a single expression slot.

## Consequences

- **Ergonomics**: Reduces explicit conversion boilerplate.
- **Transparency**: All healings produce warnings, never silent.
- **Non-determinism**: Random candidate shuffling means different compilations may try candidates in different orders. However, any successful healing is semantically correct.
- **Performance**: Cloning the entire AST per rollout is expensive for large programs.
- **Bounded**: The budget (100 rollouts, max cost 2) prevents unbounded search.

## Related Files

- [vinglish-types/src/healer.rs](../../crates/vinglish-types/src/healer.rs)
- [vinglish-types/src/type_pass.rs](../../crates/vinglish-types/src/type_pass.rs) (`run_with_healing`)
