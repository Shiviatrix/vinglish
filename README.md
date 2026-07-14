<div align="center">
  <img src="logos/vinglish-icon-color.svg" alt="Vinglish Logo" width="200" />
</div>

# Vinglish (v0.1.0)

**Programming that reads like English. Built on mathematics.**

Vinglish is a modern, statically-typed systems programming language developed as a comprehensive academic project. It is built upon a core philosophy: **programming should read like English, but the compiler understands the intent of the user.**

By combining an intent-aware semantic analysis pipeline, an optimizing SSA-form Mid-Level Intermediate Representation (MIR), and a zero-overhead C-codegen backend, Vinglish successfully merges high-level syntactical expression with low-level systems control.

---

## Core Philosophy

* **Intent over Syntax:** The compiler actively models data ownership and types to minimize syntactic boilerplate.
* **Human Readability:** The language grammar is designed to closely resemble English prose.
* **Mathematical Foundation:** The compiler leverages a robust intermediate representation (MIR) to perform safe, mathematically-grounded optimizations.
* **Zero Overhead:** The language compiles directly to low-level C. The runtime is extremely minimal, allowing the majority of the standard library to be implemented natively in Vinglish.
* **Native Rust Ecosystem Integration:** Vinglish includes a custom `#[vinglish_export]` macro that generates seamless C-FFI bridges for any Rust crate automatically, allowing you to use world-class libraries natively without writing glue code.
* **Native UI:** Utilize the built-in `std.ui` module, powered by Rust's `minifb`, to build cross-platform desktop UI experiences powered directly by Vinglish logic.

## Architecture Highlights

The Vinglish compiler implements a multi-stage pipeline:

1. **HIR (High-Level IR):** Performs comprehensive type resolution, modular symbol graph construction, and alias analysis.
2. **MIR (Mid-Level IR):** A mathematically pure, Static Single Assignment (SSA) form utilized for advanced optimization.
3. **Optimizations:** Includes Constant Folding, Global Value Numbering (GVN), Dead Code Elimination (DCE), and Control Flow Graph (CFG) Simplification.
4. **Code Generation:** Translates the optimized MIR into highly performant C code.

## Documentation

- [Language Guide](./docs/language_guide.md): An overview of the language syntax, functions, structs, and generics.
- [Architecture](./docs/architecture.md): A detailed examination of the compiler internals, including the HIR, MIR, and Codegen phases.
- [Standard Library](./docs/standard_library.md): An analysis of the native `std` library, detailing generic vectors, hash maps, and I/O implementations.

## Getting Started

### Installation

You can install the Vinglish compiler globally on any machine with a single terminal command. This script will automatically download the source, build the optimized binary, and configure your standard library paths:

```bash
curl -fsSL https://raw.githubusercontent.com/Shiviatrix/vinglish/main/install.sh | bash
```

*Note: This requires [Rust](https://rustup.rs/) and Git to be installed on your system.*

### Compiling Vinglish Source

To compile an Vinglish source file to an executable binary, execute the following command:

```bash
cargo run --bin eng-cli -- compile my_file.eng
```

This command initiates the full compilation pipeline (Lex, Parse, HIR, MIR, Optimize, C-Codegen), links the generated code with the minimal C runtime (`rt/eng_runtime.c`), and outputs the final binary executable.
