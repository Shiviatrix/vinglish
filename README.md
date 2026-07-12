<div align="center">
  <img src="./Englist%20Icon.png" alt="Englist Logo" width="200" />
</div>

# Englist

**Programming that reads like English. Built on mathematics.**

Englist is a modern, statically-typed systems programming language developed as a comprehensive academic project. It is built upon a core philosophy: **programming should read like English, but the compiler understands the intent of the user.**

By combining an intent-aware semantic analysis pipeline, an optimizing SSA-form Mid-Level Intermediate Representation (MIR), and a zero-overhead C-codegen backend, Englist successfully merges high-level syntactical expression with low-level systems control.

---

## Core Philosophy

* **Intent over Syntax:** The compiler actively models data ownership and types to minimize syntactic boilerplate.
* **Human Readability:** The language grammar is designed to closely resemble English prose.
* **Mathematical Foundation:** The compiler leverages a robust intermediate representation (MIR) to perform safe, mathematically-grounded optimizations.
* **Zero Overhead:** The language compiles directly to low-level C. The runtime is extremely minimal, allowing the majority of the standard library to be implemented natively in Englist.

## Architecture Highlights

The Englist compiler implements a multi-stage pipeline:

1. **HIR (High-Level IR):** Performs comprehensive type resolution, modular symbol graph construction, and alias analysis.
2. **MIR (Mid-Level IR):** A mathematically pure, Static Single Assignment (SSA) form utilized for advanced optimization.
3. **Optimizations:** Includes Constant Folding, Global Value Numbering (GVN), Dead Code Elimination (DCE), and Control Flow Graph (CFG) Simplification.
4. **Code Generation:** Translates the optimized MIR into highly performant C code.

## Documentation

- [Language Guide](./docs/language_guide.md): An overview of the language syntax, functions, structs, and generics.
- [Architecture](./docs/architecture.md): A detailed examination of the compiler internals, including the HIR, MIR, and Codegen phases.
- [Standard Library](./docs/standard_library.md): An analysis of the native `std` library, detailing generic vectors, hash maps, and I/O implementations.

## Getting Started

### Building the Compiler

The Englist compiler is implemented in Rust. The `cargo` build system is required for compilation.

```bash
cargo build --release
```

### Compiling Englist Source

To compile an Englist source file to an executable binary, execute the following command:

```bash
cargo run --bin eng-cli -- compile my_file.eng
```

This command initiates the full compilation pipeline (Lex, Parse, HIR, MIR, Optimize, C-Codegen), links the generated code with the minimal C runtime (`rt/eng_runtime.c`), and outputs the final binary executable.
