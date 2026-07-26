# Optimizations

The optimization framework applies a configurable sequence of MIR transformation passes.

---

## Purpose

Reduce generated code size and improve runtime performance by eliminating redundant instructions, simplifying the CFG, and folding constants.

---

## Responsibilities

- Provide a `PassManager` that runs passes in sequence.
- Validate MIR after each pass.
- Collect statistics about transformations performed.

---

## Design

Defined in `vinglish-opt/src/lib.rs`.

### PassManager

`PassManager<V>` holds a `Vec<Box<dyn OptimizationPass<V>>>`. `run_all()` executes each pass in order, running `MirValidatorPass::validate()` after each pass. If validation fails, the pipeline aborts with the validation errors.

### Optimization Pass Trait

```rust
pub trait OptimizationPass<V> {
    fn name(&self) -> &'static str;
    fn run(&mut self, module: &mut MirModule<V>, symbol_table: &SymbolTable) -> PassStats;
}
```

### Pass Statistics

```rust
pub struct PassStats {
    pub removed_instructions: usize,
    pub merged_blocks: usize,
    pub folded_constants: usize,
    pub gvn_eliminated: usize,
}
```

### Pipeline Configuration

#### Pre-SSA Pipeline

Operates on `MirModule<VariableId>`:

| Order | Pass | Module |
|---|---|---|
| 1 | Dead Code Elimination | `dce.rs` |
| 2 | CFG Simplification | `cfg_simplify.rs` |

#### Post-SSA Pipeline

Operates on `MirModule<SsaValueId>`:

| Order | Pass | Module |
|---|---|---|
| 1 | Constant Folding | `constant_folding.rs` |
| 2 | Constant Propagation | `constant_prop.rs` |
| 3 | Copy Propagation | `copy_prop.rs` |
| 4 | Global Value Numbering | `gvn.rs` |
| 5 | Dead Code Elimination | `dce.rs` |
| 6 | CFG Simplification | `cfg_simplify.rs` |

### Pass Descriptions

- **Dead Code Elimination** (`dce.rs`): Removes instructions whose results are never used.
- **CFG Simplification** (`cfg_simplify.rs`): Merges basic blocks connected by unconditional jumps. Tracks `merged_blocks` in statistics.
- **Constant Folding** (`constant_folding.rs`): Evaluates operations on constant operands at compile time. Tracks `folded_constants`.
- **Constant Propagation** (`constant_prop.rs`): Replaces variable references with their constant values.
- **Copy Propagation** (`copy_prop.rs`): Replaces variable references with the source of copy assignments.
- **Global Value Numbering** (`gvn.rs`): Eliminates redundant computations by assigning value numbers. Tracks `gvn_eliminated`.

---

## CLI Integration

- `--emit mir-before`: Print MIR before optimization.
- `--emit mir` / `--emit mir-after`: Print MIR after optimization.
- `--emit ssa`: Print SSA form after optimization.
- `--emit mir-stats`: Print optimization statistics.
- `--emit mir-diff`: Print before and after MIR.

---

## Related Components

- [Reference: MIR](mir.md)
- [Reference: SSA](ssa.md)
- [Compiler Pipeline](../explanation/compiler-pipeline.md)
