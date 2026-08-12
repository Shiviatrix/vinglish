# Architecture

Vinglish compiles down to C in a few distinct passes. It takes your `.ving` code, performs parsing and type inference, optimizes the intermediate representation, and generates C code that your system's compiler (like `gcc` or `clang`) turns into a native binary.

---

## 1. Overview

Vinglish is a statically-typed language. The compiler is written in Rust and split across 19 different crates in a Cargo workspace so it's easier to maintain and test. 

## 2. What it does

- Parses an English-like syntax into a typed Abstract Syntax Tree (AST).
- Infers types and resolves variable names.
- Converts the AST into a mid-level Intermediate Representation (MIR) in Static Single Assignment (SSA) form so we can optimize it.
- Generates C code from that optimized MIR.

## 3. Constraints

- We primarily compile to C. There is an experimental LLVM backend, but it's not the main path.
- The C runtime (`rt/`) provides the underlying C code for standard library features.
- A Rust runtime (`rt_rust/`) handles UI capabilities via `minifb` and facilitates Rust interop.

---

## 4. How the pieces fit together

```text
You write .ving files
       │
       ▼
vng CLI (vinglish-cli) parses your code + standard library
       │
       ▼
Compiler Pipeline processes code (AST -> HIR -> MIR -> C)
       │
       ▼
Generated C source + rt/*.c
       │
       ▼
System C compiler (cc/gcc/clang) builds it
       │
       ▼
Native executable!
```

---

## 5. The Pipeline

The whole process is kicked off by `cmd_build()` in `vinglish-cli/src/main.rs`. 

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
| Pre-SSA opt | `vinglish-opt` | `MirModule<VariableId>` | Optimized `MirModule<VariableId>` |
| SSA conversion | `vinglish-ssa` | `MirModule<VariableId>` | `MirModule<SsaValueId>` |
| SSA validation | `vinglish-ssa` | `MirModule<SsaValueId>` | Errors or pass |
| Post-SSA opt | `vinglish-opt` | `MirModule<SsaValueId>` | Optimized `MirModule<SsaValueId>` |
| Ownership analysis (MIR) | `vinglish-own` | `MirModule<SsaValueId>` | `OwnershipGraph` |
| Ownership validation | `vinglish-own` | `MirModule` + graph | Errors or pass |
| C code generation | `vinglish-codegen` | `MirModule<SsaValueId>` | C source string |
| Native compilation | System `cc` | C source + `rt/*.c` | Binary |

If you have multiple files, we load the module graph, resolve your `use` statements, and compile them in topological order.

---

## 6. What each crate actually does

### Frontend (`vinglish-lexer`, `vinglish-parser`)
The lexer tokenizes your code line-by-line. It's indentation-sensitive, meaning it tracks your indents and emits synthetic `Indent`/`Dedent` tokens. It also maps English keywords (like `compute`) to standard ones. The parser is a standard recursive-descent parser that builds the AST.

### Middle-end (`vinglish-hir`, `vinglish-types`)
`vinglish-hir` holds our High-level IR types and the central symbol table. 
`vinglish-types` resolves names and infers types. It also implements an automatic "type healing" mode. If it sees a type mismatch, it performs a bounded search to fix it (like automatically dereferencing a pointer or wrapping something in a `to_text()` call) before throwing an error. Once types are valid, it lowers the HIR into MIR.

### Optimizer (`vinglish-mir`, `vinglish-ssa`, `vinglish-opt`)
`vinglish-mir` defines our instructions and blocks. 
`vinglish-ssa` converts those into SSA form by building a dominator tree, dropping in phi nodes, and renaming variables. 
`vinglish-opt` runs the actual optimization passes: dead code elimination, constant folding, constant/copy propagation, and global value numbering.

### Backend (`vinglish-codegen`, `vinglish-decompile`)
`vinglish-codegen` generates C code. It handles struct fields using byte offsets and pointer math. A notable feature: it takes the optimized MIR payload, compresses it with zlib, signs it with SHA-256, and embeds it as a base64 comment at the bottom of the C file. `vinglish-decompile` lets you extract that payload later.

### Safety Checks (`vinglish-ownership`, `vinglish-own`)
These enforce ownership rules to prevent double-freeing memory or using variables after they are moved. We check this both at the AST level and the MIR level.

### Tooling (`vinglish-diagnostics`, `vinglish-cli`, `vinglish-fmt`, `vinglish-lsp`)
Additional tooling for the ecosystem. `diagnostics` provides formatted terminal errors with line snippets. `cli` is the main binary. We also have a formatter and a language server for VS Code.

---

## 7. Key Data Types

Here are the primary data structures used across the compiler:

| Type | Crate | What it is |
|---|---|---|
| `Token` | `vinglish-lexer` | Keywords, literals, punctuation |
| `Spanned<T>` | `vinglish-lexer` | Wraps something with its byte location in the source code |
| `ast::Module` | `vinglish-parser` | The root of the un-typed AST |
| `hir::Module` | `vinglish-hir` | The root of the typed IR |
| `SymbolTable` | `vinglish-hir` | A flat list of variables/functions, indexed by `SymbolId` |
| `MirModule<V>` | `vinglish-mir` | A collection of basic blocks and instructions |
| `Diagnostic` | `vinglish-diagnostics` | Formatted errors/warnings with suggestions |
| `CompilerContext` | `vinglish-types` | Holds the symbol table and tracks errors during compilation |

---

## 8. Limitations

A few features that are currently incomplete:
- The C backend just uses `long` (`int64_t`) for almost everything.
- Stack allocations actually just call `calloc` (heap allocation) under the hood in C right now.
- `Drop` instructions don't actually emit `free()` in C yet, they just emit a no-op `(void)0`.
- The package manager (`vng pkg`) just builds scaffolding; there's no real package registry yet.
