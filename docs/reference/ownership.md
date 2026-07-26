# Ownership

Two crates implement ownership checking at different levels of the pipeline.

---

## Purpose

Enforce ownership rules to prevent use-after-move, double-free, and aliasing violations.

---

## vinglish-ownership (AST-level)

Defined in `vinglish-ownership/src/checker.rs`.

`check_module(ast: &ast::Module) -> Vec<OwnershipError>` performs ownership checking on the AST. Called after type inference and before MIR lowering.

Each error carries a `message: String`, `span: Span`, and optional `note: Option<String>`. Errors are reported with code `O0001`.

---

## vinglish-own (MIR-level)

Defined in `vinglish-own/src/`.

### Modules

| Module | Description |
|---|---|
| `analysis.rs` | `OwnershipAnalysisPass` — builds the ownership graph from SSA MIR |
| `graph.rs` | `OwnershipGraph` — ownership state graph (implements `Display`) |
| `state.rs` | `OwnershipState` — per-value ownership state |
| `validator.rs` | `OwnershipValidator` — checks graph for violations, returns `Vec<Diagnostic>` |
| `diagnostics.rs` | Diagnostic formatting for ownership errors |

### Pipeline Integration

1. `OwnershipAnalysisPass::run(&mut ssa_module, &symbol_table)` → `OwnershipGraph`
2. `OwnershipValidator::validate(&symbol_table, &ssa_module, &own_graph)` → `Result<(), Vec<Diagnostic>>`

The ownership graph can be printed with `--emit ownership`.

### Convenience Function

`analyze_ownership(module, symbol_table) -> Result<MirModule, Vec<Diagnostic>>` combines analysis and validation into a single call.

---

## Copy vs Move

Types with copy semantics (as defined by `Type::is_copy()`) do not participate in ownership transfer. Currently: `Int`, `Float`, `Bool`, `Text`, `Unit`, `Pointer`.

---

## Related Components

- [Reference: MIR](mir.md)
- [Reference: Type System](type-system.md)
- [Compiler Pipeline](../explanation/compiler-pipeline.md)
