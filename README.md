<div align="center">
  <img src="logos/vinglish-icon-color.svg" alt="Vinglish Logo" width="200" />
</div>

# Vinglish (v0.1.0)

**Programming that reads like English. Built on mathematics.**

Vinglish is a modern, statically-typed systems programming language developed as a comprehensive academic project. It is built upon a core philosophy: **programming should read like English, but the compiler understands the intent of the user.**

By combining an intent-aware semantic analysis pipeline, an optimizing SSA-form Mid-Level Intermediate Representation (MIR), and a zero-overhead C-codegen backend, Vinglish successfully merges high-level syntactical expression with low-level systems control.

```vinglish
type Counter
begin
    value: number
end

function count_to(limit: number) returns number
begin
    let counter be Counter { value: 0 }

    repeat while counter.value is below limit
    begin
        counter.value += 1
    end

    return counter.value
end
```

---

## Cool Features

### Self-Healing Compiler Pipeline

Vinglish type checking records structured constraints rather than treating a
diagnostic as a terminal string. For a recoverable mismatch, the compiler can
evaluate a bounded, deterministic rewrite—currently including auto-dereference
and `to_text` conversion—against the expected type, actual type, and source
node. It rebuilds the affected AST in memory, re-runs ordinary inference from a
clean compiler context, and commits the rewrite only if the program validates.
Successful repairs are emitted as warnings; failed candidates preserve the
original fatal diagnostic. No heuristic model or external semantic engine is
involved.

### Bi-Directional MIR ↔ C Compilation

The production C route consumes optimized SSA MIR. Every generated basic block,
instruction, and terminator receives a versioned `/* vinglish:mir ... */`
comment carrying its deterministic MIR identity and canonical payload. Standard
C compilers discard these comments, so they have zero runtime and object-code
cost. The `vinglish-decompile` crate reads the tags to recover the MIR identity
graph and provides the boundary for canonical MIR reconstruction; reverse
lowering witnesses preserve source-level structure for AST serialization.

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
4. **MIR Code Generation:** Translates optimized SSA MIR into typed C with
   explicit foreign linkage, C-layout offsets, a static string pool, and
   zero-cost reconstruction metadata.

## Documentation

- [Language Guide](./docs/language_guide.md): An overview of the language syntax, functions, structs, and generics.
- [Architecture](./docs/architecture.md): A detailed examination of the compiler internals, including the HIR, MIR, and Codegen phases.
- [Standard Library](./docs/standard_library.md): An analysis of the native `std` library, detailing generic vectors, hash maps, and I/O implementations.
- [Playground](./docs/playground.md): The native Skyline Runner example and its browser preview.

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
cargo run -p vinglish-cli --bin vng -- build my_file.ving --backend c
```

This command initiates the full compilation pipeline (Lex, Parse, HIR, MIR, Optimize, C-Codegen), links the generated code with the minimal C runtime (`rt/eng_runtime.c`), and outputs the final binary executable.
