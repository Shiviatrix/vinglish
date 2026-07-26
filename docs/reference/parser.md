# Parser

The Vinglish parser consumes a token stream and produces an abstract syntax tree.

---

## Purpose

Convert `Vec<Spanned<Token>>` into `ast::Module`, reporting parse errors as structured diagnostics.

---

## Responsibilities

- Parse top-level items: functions, types, enums, use declarations, packages, modules, routes.
- Parse statements: let, return, if/then/otherwise, when, repeat, match, assign, spawn, send, receive, transaction.
- Parse expressions: literals, identifiers, calls, binary/unary operations, field access, indexing, struct literals, list literals, macro calls, postfix try (`?`), generic instantiation.
- Support both `begin`/`end` block syntax and indentation-based blocks.
- Report errors with spans for diagnostic rendering.

---

## Design

Recursive-descent parser. Implemented in `vinglish-parser/src/parser.rs`.

### Entry Point

`parse(tokens: &[Spanned<Token>]) -> (ast::Module, Vec<ParseError>)`

### Error Types

Defined in `vinglish-parser/src/error.rs`:

| Variant | Description |
|---|---|
| `Expected` | Expected a specific token, found something else |
| `Custom` | Free-form error message with span |

Each `ParseError` has a `span()` method returning the error location.

---

## Data Flow

**Input**: `&[Spanned<Token>]`

**Output**: `(ast::Module, Vec<ParseError>)`

---

## Public API: AST Types

All defined in `vinglish-parser/src/ast.rs`.

### Top-Level Items

| Type | Description |
|---|---|
| `Module` | Root node; contains `Vec<Item>` |
| `Item` | `Function`, `Type`, `Enum`, `Package`, `Module`, `Use`, `Route`, `Statement` |
| `FunctionDef` | Name, visibility, params, return type, body, type params, target type (`on T`), effects, `is_foreign` |
| `TypeDef` | Name, visibility, fields, type params, capabilities (`requires`) |
| `EnumDef` | Name, visibility, variants, type params |
| `Variant` | Name, optional payload type |
| `UseDecl` | Dot-separated path |
| `PackageDecl` | Package name |
| `ModuleDecl` | Module name |
| `RouteDecl` | Path string, handler block |

### Statements

| Type | Description |
|---|---|
| `LetStmt` | `let name be value` with optional type annotation, `mutable` flag |
| `ReturnStmt` | `return` with optional expression |
| `IfStmt` | Condition, then block, optional otherwise block |
| `WhenStmt` | Same structure as `IfStmt` |
| `RepeatStmt` | `ForEvery { var, iterable, body }`, `While { condition, body }`, `Count { times, body }` |
| `MatchStmt` | Subject expression, cases, optional otherwise |
| `AssignStmt` | Target, operator (`=`, `+=`, `-=`, `*=`, `/=`), value |
| `SpawnStmt` | Actor identifier |
| `SendStmt` | Message expression |
| `ReceiveStmt` | Optional binding |
| `TransactionStmt` | Body block |

### Expressions

| Variant | Description |
|---|---|
| `Lit` | Literal value (`Int`, `Float`, `Text`, `Bool`, `Unit`) |
| `Ident` | Identifier reference |
| `GenericInst` | `base<type_args>` |
| `Call` | Callee expression + argument list |
| `BinOp` | Left, operator, right. Operators: `Add`, `Sub`, `Mul`, `Div`, `Mod`, `Eq`, `NotEq`, `Lt`, `Gt`, `LtEq`, `GtEq`, `And`, `Or`, `IsBelow`, `IsAbove`, `Exceeds` |
| `UnOp` | Operator + operand. Operators: `Neg`, `Not`, `Deref`, `Borrow(mutable)` |
| `Field` | `object.field` |
| `Index` | `object[index]` |
| `StructLit` | `Type { field: value, ... }` |
| `Block` | Block of statements |
| `List` | `[elements]` |
| `MacroCall` | `name!(args)` |
| `PostfixTry` | `expr?` |

### Type Expressions

| Variant | Description |
|---|---|
| `Named(ident)` | `number`, `text`, `boolean`, `decimal`, or user-defined type |
| `List(inner)` | `List of T` |
| `Dict { key, val }` | `Dictionary from K to V` |
| `Optional(inner)` | `T?` or `Optional T` |
| `Result(inner)` | `Result of T` |
| `Generic { base, args }` | Generic instantiation |
| `Reference { mutable, inner }` | `borrow T` or `borrow mutable T` |

### Visibility

`Private` (default), `Public`, `Internal`.

---

## Limitations

- No operator precedence climbing; precedence is handled by the recursive structure.
- Error recovery is limited to collecting errors and continuing.

---

## Related Components

- [Reference: Lexer](lexer.md)
- [Reference: HIR](hir.md)
- [Compiler Pipeline](../explanation/compiler-pipeline.md)
