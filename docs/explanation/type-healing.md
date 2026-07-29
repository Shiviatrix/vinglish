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
3. Generate candidate repairs and calculate their costs.
4. Sort candidates by cost (lowest cost first).
5. For each candidate (Greedy + Alpha-Beta Pruning):
   - Path-clone the AST to the failing expression using `Arc::make_mut` (persistent sharing).
   - Replace the expression with the candidate fix.
   - Re-run the type pass on the modified AST.
   - If it succeeds, the candidate is accepted. Since candidates are sorted, the first success is guaranteed to be the optimal lowest-cost fix.
   - The engine updates its alpha bound and prunes all remaining, more expensive permutations without testing them.
   - Emit a `HealingWarning` and commit the new AST.

### Bounds

- `Healer::MAX_STEPS`: 2 (candidates with cost > 2 are skipped).
- Budget: Because the engine is completely deterministic and prunes sub-optimal paths immediately, there is no arbitrary iteration budget.

### Diagnostics

Successful healings produce warning `T1001` which is rendered to stderr. The warning includes which rule was applied and the span of the affected expression.

---

## Limitations

- Only two healing rules exist.
- `find_expr()` and `find_expr_mut()` perform a full AST walk per healing attempt.

---

## Related Components

- [Reference: Type System](../reference/type-system.md)
- [Compiler Pipeline](compiler-pipeline.md)
