## High-Level Overview

The compiler follows a staged pipeline:

```text
source .ving
  -> lexer
  -> parser
  -> type inference
  -> HIR
  -> MIR
  -> SSA
  -> C / LLVM backend
  -> executable
```

Each stage narrows the program representation and makes the next stage easier to reason about. Early stages focus on syntax and types. Middle stages normalize control flow and data movement. The backend emits machine-ready output.

## Lexical Analysis

The lexer converts text to a token stream. It recognizes numbers, strings, identifiers, punctuation, and keywords. It also handles whitespace and comments. Because Vinglish uses English-like keywords, the lexer must distinguish `if`, `begin`, `is`, and similar tokens from ordinary identifiers before parsing begins.

## Parsing

The parser builds an AST from the token stream. Statements include declarations, returns, loops, and conditionals. Expressions include literals, calls, field access, and arithmetic. The parser also supports named struct literals and list literals. When it cannot continue cleanly, it recovers and reports the nearest valid error rather than failing abruptly.

## Type Checking and Inference

The type checker uses unification to connect values and functions. Type variables are introduced when a type is not yet known and then solved against later constraints. This is standard for a language that supports inference. If a value cannot satisfy a constraint, the compiler reports a mismatch along with the offending expression.

## HIR

The HIR normalizes the source into a more compiler-friendly form. It removes some user-facing syntax and makes control flow more explicit. This is the stage where the compiler begins to reason about structured transformations rather than just source text.

## MIR

The MIR stage makes control flow and data movement more explicit. It tracks how values move between blocks and where memory updates occur. This is where the runtime model begins to matter, especially for collections and pointer-like structures that need explicit allocation and access paths.

## SSA

Static Single Assignment is used to simplify optimization and dataflow analysis. Each variable has a single assignment in the control-flow graph, and values are merged through joins or phi-equivalent structures. This makes dead-code elimination and constant propagation easier to apply correctly.

## Backends

The compiler supports a C backend and a LLVM-based backend path. The C backend is useful because it keeps the generated output readable and exposes ABI details cleanly. The LLVM path emits IR and relies on LLVM's optimization and code generation. Both approaches depend on the runtime and on the language's calling conventions.

## Optimization Passes

Optimization is applied in the middle and backend stages. Common passes include constant folding, dead code elimination, inlining, and instruction simplification. These passes do not replace algorithmic work, but they do reduce redundant operations and make the generated code easier to reason about.
