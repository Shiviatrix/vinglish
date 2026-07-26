# Type Healing

The Vinglish compiler includes a bounded AST repair mechanism called "type healing." When a type mismatch is detected during inference, the compiler attempts to automatically transform the AST to satisfy the constraint before reporting an error.

---

## Purpose

Reduce the number of type errors that require manual intervention. The compiler inserts well-defined transformations rather than accepting unsound programs.

---

## Design

The healer is implemented in `vinglish-types/src/healer.rs`. It is invoked by `TypeInferencePass::run_with_healing()`.

### Healing Rules

| Rule | Condition | Transformation |
|---|---|---|
| `AutoDeref` | Expected `T`, actual `Reference(T, _)` | Wraps expression in `UnOp::Deref` |
| `ToText` | Expected `Text`, actual is not `Text` | Wraps expression in `Call { callee: "to_text", args: [expr] }` |

### Candidate Search

The `Healer::candidates()` method generates a list of `HealingCandidate` values. Each candidate has a `rule`, a `replacement` expression, and a `cost` (always 1 in the current implementation).

`attempt_heal()` works as follows:
1. Extract the type constraint from the `TypeError::Mismatch` variant.
2. Find the failing expression in the AST by matching `AstNodeId` (span-based lookup).
3. Generate candidates.
4. Shuffle candidates (uses `rand::seq::SliceRandom`).
5. For each candidate (up to 100 rollouts):
   - Clone the AST.
   - Replace the expression.
   - Re-run the type pass on the modified AST.
   - If it succeeds, commit the AST and return a `HealingWarning`.
   - Otherwise, discard and try the next candidate.

### Bounds

- `Healer::MAX_STEPS`: 2 (candidates with cost > 2 are skipped).
- Rollout budget: 100 iterations per error.

### Diagnostics

Successful healings produce warning `T1001` which is rendered to stderr. The warning includes which rule was applied and the span of the affected expression.

---

## Limitations

- Only two healing rules exist.
- The candidate search uses random shuffling, introducing non-determinism.
- `find_expr()` and `find_expr_mut()` perform a full AST walk per healing attempt.
- Each rollout clones the entire AST module.

---

## Related Components

- [Reference: Type System](../reference/type-system.md)
- [Compiler Pipeline](compiler-pipeline.md)
