# Vinglish Architecture

<div align="center">
  <img src="../Vinglish%20Icon.png" alt="Vinglish Logo" width="100" />
</div>

The Vinglish compiler is architected as a sequence of independent passes that transform high-level source code into heavily optimized machine code. This document outlines the fundamental stages of the compilation pipeline.

## 1. Frontend: Lexing and Parsing
The initial phase involves tokenizing the source file and parsing it into an Abstract Syntax Tree (AST). 
- **Modularity**: The frontend resolves `use std.X` directives into a Module Resolver graph. This structural approach preserves distinct compilation units, effectively preventing symbol collisions across the program.

## 2. HIR (High-Level Intermediate Representation)
The AST is subsequently lowered into the High-Level Intermediate Representation (HIR), where comprehensive semantic analysis is performed.
- **Type Checking:** All variables, generics, and return types are rigorously validated.
- **Symbol Resolution:** Functions and structs are mapped securely within the global Symbol Table.
- **Alias Analysis:** This pass provides a foundational analysis to support future IDE tooling (e.g., LSP, Diagnostics, Code Actions).

## 3. MIR (Mid-Level Intermediate Representation)
The validated HIR is flattened into the Mid-Level Intermediate Representation (MIR), structured in **Static Single Assignment (SSA)** form.
- In SSA form, every variable is assigned exactly once, which significantly simplifies the implementation of data-flow analysis and optimization algorithms.
- Basic Blocks are utilized to accurately model the Control Flow Graph (CFG) of the program.

## 4. Optimization Passes
Several optimization passes operate directly on the MIR to maximize execution efficiency:
- **CFG Simplification (`cfg_simplify.rs`)**: Removes unreachable blocks and merges linear block sequences.
- **Constant Propagation (`constant_prop.rs`)**: Evaluates constant expressions statically at compile-time.
- **Constant Folding**: Combines and simplifies algebraic expressions.
- **Global Value Numbering (GVN)**: Identifies and eliminates redundant calculations.

## 5. Codegen
The optimized MIR (and sometimes the AST directly, depending on the backend) is translated into machine code via a target backend.
- **LLVM Backend (`crates/eng-llvm`)**: Consumes the optimized Mid-Level IR (MIR) to generate highly optimized, native LLVM IR.
- **C Backend (`--backend c`)**: Currently serves as a rapid bootstrapping backend that consumes the AST directly (bypassing MIR). It generates secure, high-performance C code, utilizing `__auto_type` for reliable generic inference. This output is linked with a minimal runtime and compiled using the host C compiler (`clang` or `gcc`) to produce the final executable binary.
