<div align="center">
  <img src="logos/vinglish-icon-color.svg" alt="Vinglish Icon" width="128" height="128" />
</div>

**Vinglish** is a statically-typed programming language with an English-inspired syntax.

I built this compiler in Rust. It implements a standard compiler pipeline: lexing, parsing, name resolution, type inference, building an SSA-form MIR, optimizing it (DCE, constant folding, GVN), and generating C code. 

### Why Vinglish?
I wanted a language that reads naturally but still gives you low-level control. The compiler uses a custom C runtime for standard libraries (like networking and IO), and it even ships with an experimental LLVM backend and a built-in LSP.

---

## 🛠 What's working

- **Frontend:** Custom lexer (indentation-sensitive), recursive descent parser, and a full AST.
- **Middle-end:** Type inference, symbol resolution, and automatic type healing (e.g., auto-dereferencing or inserting `to_text()` calls when a string is expected).
- **Optimizer:** Lowers AST to HIR, then to an SSA-form MIR. It runs dead code elimination, constant propagation, copy propagation, and global value numbering.
- **Backend:** Emits C code. It embeds the MIR payload inside the generated C as a base64 comment with a SHA-256 hash so it can be decompiled later.
- **Tooling:** Comes with a CLI (`vng`), a formatter, a tree-walk interpreter, and an LSP server for editor support.

## 📦 Project Structure

The compiler is split across multiple crates for modularity:

| Crate | Function |
|---|---|
| `vinglish-lexer` / `parser` | Turns `.ving` files into an AST. |
| `vinglish-hir` / `types` | Resolves names, infers types, and lowers to MIR. |
| `vinglish-mir` / `ssa` / `opt` | Data structures for the IR, SSA conversion, and the optimization passes. |
| `vinglish-codegen` | The C backend and the tree-walk interpreter. |
| `vinglish-own` / `ownership` | Memory safety checks (AST and MIR levels). |
| `vinglish-diagnostics` | Makes the error messages look nice. |
| `vinglish-lsp` / `fmt` | Language server and source code formatter. |
| `vinglish-cli` | The main `vng` binary entry point. |

The repository also includes the standard library (`std/`), the C runtime (`rt/`), and integration tests (`tests/`).

## Quick Start

### Installation

To install the Vinglish compiler globally so you can use the `vng` command from anywhere:

```sh
cargo install --path crates/vinglish
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
