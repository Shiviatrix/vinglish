<div align="center">
  <img src="logos/vinglish-wordmark.svg" alt="Vinglish" width="400" />
</div>

A statically-typed programming language with English-inspired syntax that compiles to C.

## Features

- [x] Lexer with indent/dedent tracking and natural-language keyword aliases
- [x] Recursive-descent parser producing a typed AST
- [x] Symbol table with type interning
- [x] Name resolution and type inference
- [x] Type healing (auto-deref, `to_text` insertion via bounded candidate search)
- [x] MIR lowering from HIR
- [x] SSA construction (dominator tree, phi node insertion, variable renaming)
- [x] Optimization passes: DCE, CFG simplification, constant folding, constant propagation, copy propagation, GVN
- [x] C code generation from SSA MIR
- [x] MIR round-trip payload embedded in generated C (SHA-256 integrity check)
- [x] Ownership analysis on MIR
- [x] AST-level ownership checking
- [x] Tree-walk interpreter
- [x] Diagnostics with source spans and intent resolution
- [x] LSP server
- [x] Source code formatter
- [x] Standard library: io, string, math, file, net, thread, subprocess, term, ui, collections (vector, map)
- [x] C runtime implementations for each standard library module
- [x] Rust FFI bridge via `#[vinglish_export]` procedural macro
- [x] LLVM backend (experimental)
- [x] IR export to JSON
- [x] Module system with topological dependency resolution
- [x] Package manager scaffolding (`vng pkg init`, `vng pkg add`)

## Repository Structure

| Crate | Purpose |
|---|---|
| `vinglish-lexer` | Tokenizes `.ving` source into `Vec<Spanned<Token>>` |
| `vinglish-parser` | Parses tokens into an AST (`ast::Module`) |
| `vinglish-hir` | HIR node types, symbol table, type interning, struct layout |
| `vinglish-types` | Name resolution, type inference, type healing, MIR lowering |
| `vinglish-mir` | MIR data structures: `BasicBlock`, `Instruction`, `Terminator` |
| `vinglish-ssa` | Dominator tree computation, phi insertion, variable renaming |
| `vinglish-opt` | `PassManager` with DCE, CFG simplification, constant folding, constant propagation, copy propagation, GVN |
| `vinglish-codegen` | C code emitter (`emit_mir_c`), tree-walk interpreter |
| `vinglish-own` | MIR-level ownership graph construction and validation |
| `vinglish-ownership` | AST-level ownership checking |
| `vinglish-diagnostics` | `Diagnostic` type, renderer, intent resolution, polyglot heuristics |
| `vinglish-decompile` | Extracts MIR payload from generated C (SHA-256 + zlib + base64) |
| `vinglish-ir-export` | Serializes HIR to JSON |
| `vinglish-fmt` | Source code formatter |
| `vinglish-lsp` | Language Server Protocol server |
| `vinglish-llvm` | LLVM IR code generation (experimental) |
| `vinglish-macro` | `#[vinglish_export]` procedural macro for Rust FFI |
| `vinglish-analysis` | Alias analysis, escape analysis, lifetime analysis, promotion |
| `vinglish-cli` | CLI binary: `vng build`, `vng run`, `vng check`, `vng fmt`, `vng lsp`, `vng pkg`, `vng benchmark` |
| `rt_rust` | Rust runtime (UI via minifb, FFI bridge generation) |

Other directories:

| Path | Contents |
|---|---|
| `std/` | Standard library modules (`.ving` files) |
| `rt/` | C runtime implementations |
| `tests/` | Integration test programs (`.ving` files) |
| `examples/` | Example Vinglish programs |

## Quick Start

### Build

```sh
cargo build --release
```

Requires Rust (edition 2024) and a C compiler (cc, gcc, or clang).

### Run (interpreter)

```sh
vng run path/to/file.ving
```

### Compile to native binary

```sh
vng build path/to/file.ving --output my_program --backend c
```

### Type-check without compiling

```sh
vng check path/to/file.ving
```

### Test

```sh
cargo test
```

## Documentation

- [Architecture](docs/explanation/architecture.md)
- [Compiler Pipeline](docs/explanation/compiler-pipeline.md)
- [Reference: Lexer](docs/reference/lexer.md)
- [Reference: Parser](docs/reference/parser.md)
- [Reference: Type System](docs/reference/type-system.md)
- [Reference: MIR](docs/reference/mir.md)
- [Reference: Optimizations](docs/reference/optimizations.md)
- [Reference: Code Generation](docs/reference/codegen.md)
- [Reference: CLI](docs/reference/cli.md)
- [Architecture Decision Records](docs/adr/)

## License

MIT
