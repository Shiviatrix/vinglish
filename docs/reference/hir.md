# HIR

The High-Level Intermediate Representation is a typed, resolved version of the AST. Every expression carries a `TypeId`.

---

## Purpose

Provide a typed representation of the program after name resolution and type inference, suitable for lowering to MIR.

---

## Responsibilities

- Represent typed program structure (functions, types, enums, statements, expressions).
- Hold the symbol table for name resolution and type interning.
- Provide struct layout computation.

---

## Design

Defined in `vinglish-hir/src/lib.rs`. The HIR mirrors the AST structure but replaces names with resolved IDs and adds type annotations.

### Key Differences from AST

| AST | HIR |
|---|---|
| `Ident` (name string) | `VariableId`, `FunctionId`, `TypeId` |
| `TypeExpr` (syntax) | `TypeId` (interned) |
| No type on expressions | Every `Expr` variant has a `ty: TypeId` field |
| `UseDecl`, `PackageDecl`, `ModuleDecl` | Removed (resolved during compilation) |

### HIR Nodes

| Type | Description |
|---|---|
| `Module` | `items: Vec<Item>` |
| `Item` | `Function(FunctionDef)`, `Type(TypeDef)`, `Enum(EnumDef)`, `Statement(Stmt)` |
| `FunctionDef` | `id: FunctionId`, `name`, `params: Vec<Param>`, `ret_ty: TypeId`, `body: Expr` |
| `Param` | `id: VariableId`, `name`, `ty: TypeId` |
| `TypeDef` | `id: TypeId`, `name`, `fields: Vec<Param>` |
| `EnumDef` | `id: TypeId`, `variants: Vec<Variant>` |
| `Block` | `stmts: Vec<Stmt>`, `expr: Option<Box<Expr>>`, `ty: TypeId` |

### HIR Statements

`Let`, `Assign`, `If`, `Return`, `RepeatWhile`, `Expr`.

### HIR Expressions

`Lit`, `VarRef`, `Call`, `BinOp`, `UnOp`, `FieldIndex`, `Index`, `List`, `StructInit`, `MacroCall`, `PostfixTry`, `Block`. Every variant includes `ty: TypeId` and `span: Span`.

---

## Symbol Table

Defined in `vinglish-hir/src/symbol.rs`.

### IDs

| Type | Underlying |
|---|---|
| `SymbolId` | `u32` |
| `TypeId` | `SymbolId` |
| `FunctionId` | `SymbolId` |
| `VariableId` | `SymbolId` |
| `SsaValueId` | `u32` |
| `FieldId` | `usize` |

### `SymbolTable`

Flat `Vec<SymbolKind>` indexed by `SymbolId`. Supports:
- `intern_type(Type) -> TypeId`: Deduplicated type interning.
- `define_type`, `define_func`, `define_var`: Named definitions.
- `define_anon_func`, `define_anon_var`: Anonymous definitions (no name lookup).
- `lookup(name) -> Option<SymbolId>`: Name-based lookup.

### Symbol Types

| Type | Fields |
|---|---|
| `TypeSymbol` | `id`, `name`, `visibility`, `fields: Vec<FieldSymbol>`, `methods`, `generic_params`, `capabilities` |
| `FunctionSymbol` | `id`, `name`, `visibility`, `ty: Type`, `generic_params`, `is_variant_constructor`, `is_foreign` |
| `VariableSymbol` | `id`, `name`, `is_mut`, `ty: Type`, `span` |
| `FieldSymbol` | `id`, `name`, `ty`, `visibility` |

---

## Type System

Defined in `vinglish-hir/src/types.rs`.

### Type Enum

| Variant | Description |
|---|---|
| `Int` | Integer (displayed as `number`) |
| `Float` | Float (displayed as `decimal`) |
| `Bool` | Boolean |
| `Text` | String |
| `Unit` | Unit/void |
| `Reference(Box<Type>, bool)` | Borrow (mutable if `true`) |
| `Pointer(Box<Type>)` | Raw pointer |
| `List(Box<Type>)` | List container |
| `Dict(Box<Type>, Box<Type>)` | Dictionary (key, value) |
| `Optional(Box<Type>)` | Optional value |
| `Result(Box<Type>, Box<Type>)` | Result (ok, err) |
| `Function(Vec<Type>, Box<Type>)` | Function type (params, return) |
| `Named(String, Vec<Type>)` | User-defined type with optional type arguments |
| `Var(TypeVar)` | Inference variable |

### `TypeVar`

Fresh type variable IDs are generated from a global `AtomicU32` counter. Displayed as `'a`, `'b`, etc.

### Copy Semantics

`Type::is_copy()` returns `true` for: `Int`, `Float`, `Bool`, `Text`, `Unit`, `Pointer`. All other types use move semantics.

---

## Related Components

- [Reference: Parser](parser.md)
- [Reference: MIR](mir.md)
- [Reference: Type System](type-system.md)
