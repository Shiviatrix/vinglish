# ADR-0002: SSA Form for MIR

## Status

Accepted.

## Context

The MIR needs a form suitable for dataflow optimizations. The compiler must support constant propagation, copy propagation, global value numbering, and dead code elimination.

## Decision

The MIR uses a two-phase approach:
1. Initial MIR uses `VariableId` (non-SSA, mutable variables).
2. `SSAConversionPass` transforms to `MirModule<SsaValueId>` via dominator tree computation, phi node insertion, and variable renaming.

The `MirModule<V>` type is generic over `V`, allowing the same data structures to represent both forms.

Evidence from the codebase:
- `vinglish-ssa/src/lib.rs` defines `SSAConversionPass` which takes `MirModule<VariableId>` and returns `MirModule<SsaValueId>`.
- Pre-SSA passes operate on `MirModule<VariableId>`; post-SSA passes operate on `MirModule<SsaValueId>`.
- The `PassManager<V>` in `vinglish-opt` is also generic over `V`.

## Consequences

- **Optimization**: SSA form enables precise dataflow analysis (GVN, constant propagation, copy propagation).
- **Validation**: `SSAValidator` can verify SSA properties between passes.
- **Complexity**: Two pipeline phases (pre-SSA and post-SSA) with separate pass managers.
- **Type safety**: The Rust type system prevents accidentally running SSA-only passes on non-SSA MIR.

## Related Files

- [vinglish-ssa/src/lib.rs](../../crates/vinglish-ssa/src/lib.rs)
- [vinglish-opt/src/lib.rs](../../crates/vinglish-opt/src/lib.rs) (`pre_ssa_pipeline`, `post_ssa_pipeline`)
- [vinglish-mir/src/lib.rs](../../crates/vinglish-mir/src/lib.rs) (`MirModule<V>`)
