# SSA

The SSA conversion pass transforms MIR from a variable-based form (`VariableId`) to static single assignment form (`SsaValueId`).

---

## Purpose

Enable optimization passes that require SSA properties (e.g., constant propagation, GVN, copy propagation).

---

## Responsibilities

- Compute dominator trees per function.
- Insert phi nodes at dominance frontiers.
- Rename variables to SSA form.
- Convert `MirModule<VariableId>` to `MirModule<SsaValueId>`.

---

## Design

Defined in `vinglish-ssa/src/lib.rs`.

### Entry Point

```rust
SSAConversionPass::run(
    &mut self,
    module: MirModule<VariableId>,
    symbol_table: &mut SymbolTable,
) -> MirModule<SsaValueId>
```

### Algorithm

For each function in the module:
1. **Dominator tree** (`dominators.rs`): Compute the dominator tree from the CFG.
2. **Phi insertion** (`phi.rs`): Insert phi nodes at dominance frontiers for each variable that is defined in multiple blocks.
3. **Variable renaming** (`rename.rs`): Walk the dominator tree, renaming each variable definition and use to a fresh SSA value.

After all functions are processed, `convert_to_ssa_types()` converts the module's type parameter from `VariableId` to `SsaValueId`. The conversion maps `VariableId(SymbolId(n))` to `SsaValueId(n)`.

### Validation

`SSAValidator` in `validator.rs` checks that SSA properties hold in the output module.

---

## Data Flow

**Input**: `MirModule<VariableId>` + `&mut SymbolTable`

**Output**: `MirModule<SsaValueId>`

---

## Public API

| Item | Description |
|---|---|
| `SSAConversionPass` | Unit struct; `run()` performs full conversion |
| `DominatorTree` | Computed from a `MirFunction<VariableId>` |
| `SSAValidator` | Validates SSA properties |

---

## Related Components

- [Reference: MIR](mir.md)
- [Reference: Optimizations](optimizations.md)
- [Compiler Pipeline](../explanation/compiler-pipeline.md)
