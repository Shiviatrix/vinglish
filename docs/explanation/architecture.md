# Architecture

The Vinglish compiler is a multi-stage pipeline that reads `.ving` source files, lowers them through two intermediate representations, applies optimizations, and emits C source code that is compiled to a native binary by a system C compiler.

---

## 1. Overview

Vinglish is a statically-typed language with English-inspired syntax. The compiler is implemented as a Cargo workspace containing 19 crates. Source files use the `.ving` extension.

---

## 2. Goals

The compiler aims to:
- Parse an English-like syntax into a typed AST.
- Perform type inference and name resolution.
- Lower to a mid-level IR in SSA form for optimization.
- Generate C code from SSA MIR.

---

## 3. Constraints

- The primary code generation target is C. An experimental LLVM backend exists but is not the default path.
- The C runtime (`rt/`) provides foreign function implementations for the standard library.
- The Rust runtime (`rt_rust/`) provides UI capabilities via `minifb` and FFI bridge generation.

---

## 4. System Context

```
Developer
  │
  ▼
vng CLI (vinglish-cli)
  │
  ├─ .ving source files
  ├─ std/ (standard library)
  ├─ .ving_modules/ (packages)
  │
  ▼
Compiler Pipeline
  │
  ▼
Generated C source + rt/*.c + rt_rust (optional)
  │
  ▼
System C compiler (cc/gcc/clang)
  │
  ▼
Native binary
```

---

## 5. Compiler Pipeline

The pipeline is orchestrated by `compile_project()` and `cmd_build()` in `vinglish-cli/src/main.rs`.

| Stage | Crate | Input | Output |
|---|---|---|---|
| Lexing | `vinglish-lexer` | Source string | `Vec<Spanned<Token>>` |
| Parsing | `vinglish-parser` | Token stream | `ast::Module` |
| Name resolution | `vinglish-types` | `ast::Module` | Populated `CompilerContext` |
| Type inference | `vinglish-types` | `ast::Module` + context | `hir::Module` |
| HIR validation | `vinglish-types` | `hir::Module` + context | Errors or pass |
| Ownership check (AST) | `vinglish-ownership` | `ast::Module` | Errors or pass |
| MIR lowering | `vinglish-types` | `hir::Module` | `MirModule<VariableId>` |
| MIR validation | `vinglish-mir` | `MirModule` | Errors or pass |
| Pre-SSA optimization | `vinglish-opt` | `MirModule<VariableId>` | Optimized `MirModule<VariableId>` |
| SSA conversion | `vinglish-ssa` | `MirModule<VariableId>` | `MirModule<SsaValueId>` |
| SSA validation | `vinglish-ssa` | `MirModule<SsaValueId>` | Errors or pass |
| Post-SSA optimization | `vinglish-opt` | `MirModule<SsaValueId>` | Optimized `MirModule<SsaValueId>` |
| Ownership analysis (MIR) | `vinglish-own` | `MirModule<SsaValueId>` | `OwnershipGraph` |
| Ownership validation | `vinglish-own` | `MirModule` + graph | Errors or pass |
| C code generation | `vinglish-codegen` | `MirModule<SsaValueId>` | C source string |
| Native compilation | System `cc` | C source + `rt/*.c` | Binary |

Multi-module programs are loaded via `load_module_graph()`, which resolves `use` declarations recursively, then compiled in topological order via `topological_sort()`.

---

## 6. Workspace Structure

```
vinglish/
├── crates/
│   ├── vinglish-cli/         CLI entry point
│   ├── vinglish-lexer/       Tokenizer
│   ├── vinglish-parser/      Parser and AST types
│   ├── vinglish-hir/         HIR types, symbol table, type algebra
│   ├── vinglish-types/       Name resolution, type inference, healing, MIR lowering
│   ├── vinglish-mir/         MIR data structures
│   ├── vinglish-ssa/         SSA conversion
│   ├── vinglish-opt/         Optimization passes
│   ├── vinglish-codegen/     C backend and interpreter
│   ├── vinglish-own/         MIR-level ownership analysis
│   ├── vinglish-ownership/   AST-level ownership checking
│   ├── vinglish-diagnostics/ Error reporting
│   ├── vinglish-decompile/   MIR payload extraction from C
│   ├── vinglish-ir-export/   HIR JSON serialization
│   ├── vinglish-fmt/         Source formatter
│   ├── vinglish-lsp/         LSP server
│   ├── vinglish-llvm/        Experimental LLVM backend
│   ├── vinglish-macro/       Procedural macro for FFI
│   └── vinglish-analysis/    Static analysis passes
├── rt/                       C runtime implementations
├── rt_rust/                  Rust runtime (UI, FFI bridge)
├── std/                      Standard library (.ving)
├── tests/                    Integration test programs
└── examples/                 Example programs
```

---

## 7. Crate Responsibilities

### vinglish-lexer

Tokenizes source text line-by-line. Tracks indentation via an indent stack, emitting synthetic `Indent`/`Dedent` tokens. Handles string literals with escape sequences, integer and float literals (with `_` separators), keywords, natural-language aliases (e.g., `compute` → `calculate`), and operators. Comments start with `--` or `#`.

### vinglish-parser

Recursive-descent parser that consumes `Vec<Spanned<Token>>` and produces `ast::Module`. The AST includes types for: `FunctionDef`, `TypeDef`, `EnumDef`, `UseDecl`, `PackageDecl`, `ModuleDecl`, `RouteDecl`, and statements (`Let`, `If`, `When`, `Repeat`, `Match`, `Assign`, `Spawn`, `Send`, `Receive`, `Transaction`). Expressions include `Call`, `BinOp`, `UnOp`, `Field`, `Index`, `StructLit`, `List`, `MacroCall`, `PostfixTry`, and `GenericInst`.

### vinglish-hir

Defines the HIR node types (`Module`, `Item`, `FunctionDef`, `TypeDef`, `EnumDef`, `Stmt`, `Expr`, `Block`). All HIR expressions carry a resolved `TypeId`. Contains the `SymbolTable`, which is a flat `Vec<SymbolKind>` indexed by `SymbolId`. The symbol table supports type interning, named type/function/variable definitions, and anonymous definitions. Also contains the `Type` enum (`Int`, `Float`, `Bool`, `Text`, `Unit`, `Reference`, `Pointer`, `List`, `Dict`, `Optional`, `Result`, `Function`, `Named`, `Var`) and struct layout computation.

### vinglish-types

Contains the `CompilerContext` and two compiler passes: `NameResolutionPass` and `TypeInferencePass`. The type inference pass supports a healing mode (`run_with_healing`) that uses the `Healer` to attempt bounded AST repairs on type mismatches. Two healing rules exist: `AutoDeref` (inserts a deref when a reference is passed where the inner type is expected) and `ToText` (wraps an expression in a `to_text()` call when `Text` is expected). The candidate search has a budget of 100 rollouts and a max cost of 2 steps. Also contains `MirLowerer` which converts HIR to MIR, and `HirValidatorPass`.

### vinglish-mir

Defines the MIR data structures: `MirModule<V>`, `MirFunction<V>`, `BasicBlock<V>`, `Instruction<V>`, `Terminator<V>`, and `Operand<V>`. The type parameter `V` is either `VariableId` (pre-SSA) or `SsaValueId` (post-SSA). Instructions include: `Assign`, `LoadField`, `StoreField`, `Call`, `CallIntrinsic`, `HeapAllocate`, `StackAllocate`, `BinaryOp`, `UnaryOp`, `Borrow`, `BorrowMut`, `Deref`, `Drop`, `Phi`. Terminators: `Return`, `Jump`, `Branch`. Call targets are either `Direct(FunctionId)` or `Foreign { c_symbol }`. Field accesses carry byte offsets computed at lowering time.

### vinglish-ssa

Converts `MirModule<VariableId>` to `MirModule<SsaValueId>`. The process is: compute dominator tree → insert phi nodes at dominance frontiers → rename variables. Includes `SSAValidator` to verify SSA properties.

### vinglish-opt

Provides a `PassManager<V>` that runs a sequence of `OptimizationPass` implementations. After each pass, MIR validation is re-run. Pre-SSA pipeline: `DeadCodeEliminationPass`, `CfgSimplifyPass`. Post-SSA pipeline: `ConstantFoldingPass`, `ConstantPropagationPass`, `CopyPropagationPass`, `GlobalValueNumberingPass`, `DeadCodeEliminationPass`, `CfgSimplifyPass`. Collects `PassStats` (removed instructions, merged blocks, folded constants, GVN eliminated).

### vinglish-codegen

Contains two backends. `emit_mir_c()` generates C from SSA MIR: emits forward declarations, a static string pool, struct field access via byte offsets and pointer arithmetic, `calloc` for heap/stack allocation, and `goto`-based control flow. Appends a compressed, SHA-256-signed MIR payload as a C comment. The `Interpreter` is a tree-walk interpreter that executes MIR directly.

### vinglish-own

Builds an `OwnershipGraph` from SSA MIR by running `OwnershipAnalysisPass`. `OwnershipValidator` checks the graph against the symbol table and MIR module. Reports errors as `Diagnostic` values.

### vinglish-diagnostics

Defines `Diagnostic` with severity (`Error`, `Warning`, `Info`), error code, message, source span, optional source line, and suggestions. The `renderer` module formats diagnostics for terminal output. The `intent` module resolves user intent from failed tokens. The `heuristics` module provides lexical and polyglot error detection.

### vinglish-decompile

Extracts the MIR payload from generated C source. Locates the `VINGLISH_MIR_PAYLOAD` comment, decodes base64, decompresses zlib, and verifies the SHA-256 hash of the C source preceding the payload. Returns `DecompileError::Desync` if the C source was modified after generation.

---

## 8. Data Flow

### Key Types

| Type | Crate | Description |
|---|---|---|
| `Token` | `vinglish-lexer` | Enum of all token kinds (keywords, literals, operators, punctuation) |
| `Spanned<T>` | `vinglish-lexer` | Pairs a value with its source `Span` |
| `Span` | `vinglish-lexer` | Half-open byte range `[start, end)` |
| `ast::Module` | `vinglish-parser` | Root of the parse tree |
| `ast::Expr` | `vinglish-parser` | Expression node (16 variants) |
| `ast::Stmt` | `vinglish-parser` | Statement node (12 variants) |
| `hir::Module` | `vinglish-hir` | Typed IR module |
| `hir::Expr` | `vinglish-hir` | Typed expression (every variant carries a `TypeId`) |
| `Type` | `vinglish-hir` | Type algebra (13 variants) |
| `SymbolTable` | `vinglish-hir` | Flat vector of `SymbolKind`, indexed by `SymbolId` |
| `SymbolId` | `vinglish-hir` | Newtype `u32` index into the symbol table |
| `TypeId` | `vinglish-hir` | `SymbolId` wrapper for type entries |
| `FunctionId` | `vinglish-hir` | `SymbolId` wrapper for function entries |
| `VariableId` | `vinglish-hir` | `SymbolId` wrapper for variable entries |
| `SsaValueId` | `vinglish-hir` | `u32` identity for SSA values |
| `MirModule<V>` | `vinglish-mir` | Collection of `MirFunction<V>` |
| `MirFunction<V>` | `vinglish-mir` | Function with params, locals, and basic blocks |
| `BasicBlock<V>` | `vinglish-mir` | Block ID, instruction list, terminator |
| `Instruction<V>` | `vinglish-mir` | MIR instruction (14 variants) |
| `Terminator<V>` | `vinglish-mir` | Block terminator (3 variants: `Return`, `Jump`, `Branch`) |
| `Diagnostic` | `vinglish-diagnostics` | Error/warning with code, message, span, suggestions |
| `CompilerContext` | `vinglish-types` | Carries symbol table, type errors, healing warnings |

---

## 9. Error Handling

Errors are represented as `Diagnostic` values with error codes:
- `P0001`: Parse errors
- `T0001`: Type errors
- `T1001`: Healing warnings (successful automatic repairs)
- `O0001`: Ownership errors

Diagnostics are enriched with the source line via `diag.enrich(&src)` and rendered to stderr via `vinglish_diagnostics::render()`. The intent resolution module attempts to suggest corrections for parse errors.

---

## 10. Build Process

1. `cargo build` compiles the Rust workspace.
2. `vng build file.ving` runs the full pipeline and invokes the system C compiler.
3. The C compiler links `rt/*.c` runtime files. If `rt_rust/Cargo.toml` exists, the Rust runtime is built and linked (adds macOS frameworks for UI).
4. The `CC` environment variable selects the C compiler (default: `cc`).

---

## 11. Testing

- Unit tests exist in most crates (run via `cargo test`).
- Integration test programs are `.ving` files in `tests/` (e.g., `test_factorial.ving`, `test_generics.ving`, `test_ssa.ving`, `test_borrow.ving`).
- Stress tests exist for codegen (`codegen_stress.rs`), decompilation (`stress_tests.rs`), layout computation (`layout_stress.rs`), and type healing (`healer_stress.rs`).

---

## 12. Current Limitations

- The C backend uses `long` / `int64_t` for most values; type-specific C types are limited to `double` for floats and `const char *` for strings.
- Stack allocations use `calloc` (heap allocation) in generated C.
- The `Drop` instruction emits `(void)0` (no-op) in C output.
- Phi nodes in C emit only the first operand's value.
- The LLVM backend is experimental and not the default compilation path.
- The package manager (`vng pkg`) creates stub files; no registry or version resolution exists.

---

## 13. Future Work

TODO: Describe planned improvements.
