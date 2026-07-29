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
1. Generate candidate transformations and calculate costs.
2. Sort candidates greedily by lowest cost first.
3. For each candidate (using Alpha-Beta Pruning):
   - Path-clone the AST down to the failing node using `Arc::make_mut` (persistent structural sharing).
   - Apply the transformation.
   - Re-run type checking.
   - If it passes, commit the modified AST and update the alpha bound.
   - Immediately break out of the loop, pruning all more expensive candidates.
4. Emit warning `T1001` for successful healings.

Evidence from the codebase:
- `Healer::MAX_STEPS` is 2. Candidates with cost > 2 are skipped.
- `attempt_heal()` uses persistent structural sharing, avoiding $O(N)$ clones.
- `try_heal_in_place()` provides an alternative API that mutates a single expression slot.

## Consequences

- **Ergonomics**: Reduces explicit conversion boilerplate.
- **Transparency**: All healings produce warnings, never silent.
- **Determinism & Optimality**: Because candidates are sorted by cost, the first successful candidate is guaranteed to be the optimal fix. The engine is fully deterministic.
- **Performance**: Path-cloning via `Arc` allows near $O(1)$ memory overhead per rollout, and alpha-beta pruning completely avoids evaluating sub-optimal paths.
- **Bounded**: Bounded by finite discrete candidate generation and a strict max cost threshold.

## Related Files

- [vinglish-types/src/healer.rs](../../crates/vinglish-types/src/healer.rs)
- [vinglish-types/src/type_pass.rs](../../crates/vinglish-types/src/type_pass.rs) (`run_with_healing`)
