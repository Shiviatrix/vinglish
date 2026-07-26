# Type System

The Vinglish type system uses Hindley-Milner-style inference with a unification-based approach.

---

## Purpose

Assign types to all expressions, validate type constraints, and optionally heal type mismatches.

---

## Responsibilities

- Name resolution via `NameResolutionPass`.
- Type inference via `TypeInferencePass`.
- Type healing via `Healer` (bounded AST repair).
- HIR validation via `HirValidatorPass`.
- MIR lowering via `MirLowerer`.

---

## Design

Defined across `vinglish-types/src/`.

### CompilerContext

Carries state across compilation passes:
- `symbol_table: SymbolTable`
- `current_module: String`
- `type_errors: Vec<TypeError>`
- `healing_warnings: Vec<HealingWarning>`

### Name Resolution

`NameResolutionPass::run(&ast, &mut ctx)` populates the symbol table with type, function, and variable definitions from the AST.

### Type Inference

`TypeInferencePass::run_with_healing(&mut ast, &mut ctx) -> hir::Module`

1. Runs type inference on the AST, producing an HIR module.
2. On type mismatches (`TypeError::Mismatch`), invokes `attempt_heal()`.
3. Successful healings modify the AST in place and record a `HealingWarning`.

### Type Algebra

See [Reference: HIR](hir.md) for the `Type` enum.

Primitive types: `Int` (number), `Float` (decimal), `Bool` (boolean), `Text` (text), `Unit`.

Composite types: `List(T)`, `Dict(K, V)`, `Optional(T)`, `Result(T, E)`.

Type constructors: `Reference(T, mutable)`, `Pointer(T)`, `Function(args, ret)`, `Named(name, type_args)`.

Inference variables: `Var(TypeVar)`.

### TypeVar

Generated from a global atomic counter. Fresh variables are created during inference and unified as constraints are discovered.

### Copy Semantics

`Int`, `Float`, `Bool`, `Text`, `Unit`, and `Pointer` have copy semantics. All other types have move semantics.

### MIR Lowering

`MirLowerer::lower_module(&hir) -> MirModule<VariableId>` converts HIR to MIR. Struct layout (field byte offsets) is computed in `vinglish-hir/src/layout.rs`.

---

## Error Types

| Error | Code | Description |
|---|---|---|
| `TypeError::Mismatch` | `T0001` | Expected type does not match actual type |
| `HealingWarning` | `T1001` | Type mismatch was automatically repaired |

---

## Related Components

- [Reference: HIR](hir.md)
- [Type Healing](../explanation/type-healing.md)
- [Compiler Pipeline](../explanation/compiler-pipeline.md)
