# Vinglish Compiler Architecture

Vinglish turns readable, statically typed `.ving` source into native code through
explicit compiler representations. Each stage establishes syntax, proves meaning,
makes control and data flow analyzable, optimizes safely, or emits a low-level
artifact.

```text
.ving source
    │
    ├── lexer and parser ──────── AST + module graph
    ├── semantic analysis ────── HIR + deterministic healing
    ├── MIR lowering ─────────── MIR
    ├── SSA construction ─────── optimized analysis form
    ├── optimization passes ──── simplified MIR
    ├── MIR C code generation ── typed C + provenance metadata → native binary
    └── decompilation ────────── generated C metadata → MIR identity graph
```

The language remains English-like at the source level while the optimizer works
with a precise control-flow and data-flow model.

## Front end: source, lexer, and parser

The front end accepts `.ving` files and tokenizes keywords, identifiers,
literals, operators, punctuation, and source locations. The parser builds an
abstract syntax tree (AST) for declarations, expressions, type forms, control
flow, imports, macro calls, and record literals.

Imports form a module graph rather than one flattened source file. This keeps
namespaces explicit and lets the compiler reason about each module before
constructing a complete program view.

```vinglish
use std.collections.vector

type Sample
begin
    value: number
end

function main()
begin
    let sample be Sample { value: 42 }
end
```

The parser recognizes modern record and method syntax directly: `type` replaces
the retired `struct` keyword, fields use `name: type`, values use braces, and
methods use `function name on Type (...)`.

## HIR: semantic meaning

The high-level intermediate representation (HIR) is the compiler's typed,
name-resolved view of a program. It establishes meaning before considering
low-level implementation details.

HIR construction and validation include:

- Symbol and module resolution for types, values, functions, and imports.
- Type inference and unification for variables, generic parameters, and calls.
- Validation of `Result of T` signatures, expressions, and return paths.
- Ownership, borrowing, alias, escape, and promotion analysis.
- Diagnostics with source spans and intent-aware suggestions.
- A bounded `healer.rs` recovery loop for structured type mismatches. It tests
  deterministic AST rewrites against a clean re-check and emits a warning only
  when the rebuilt program passes ordinary type checking.

A `Vector<number>` resolves to a concrete generic instantiation, while a
`Result of number` function carries its explicit success-or-error contract
throughout type checking.

## MIR and SSA

Validated HIR lowers into the mid-level intermediate representation (MIR). MIR
makes basic blocks, branches, temporaries, calls, and returns explicit. It is the
representation used for data-flow analysis and optimization.

The compiler then constructs Static Single Assignment (SSA) form. In SSA, each
logical value is defined once; when control-flow paths merge, phi nodes describe
which incoming value is selected. This makes def-use chains direct and supports
reliable value-based optimization.

```text
entry
  ├── condition ── true  → then block
  └── condition ── false → otherwise block
                         │
                    merge (phi values)
```

SSA validation, dominance computation, variable renaming, and phi insertion are
dedicated stages. A broken invariant is found near the pass that introduced it
rather than surfacing later as malformed generated code.

## Optimization pipeline

Optimization operates on typed, SSA-oriented MIR—not source spelling. This lets
Vinglish retain a readable surface syntax without paying for it at runtime.

- **Control-flow graph simplification** removes unreachable blocks and merges
  linear control flow where possible.
- **Constant propagation and folding** evaluate stable expressions at compile
  time.
- **Copy propagation** replaces needless temporary copies with their source.
- **Global value numbering (GVN)** identifies equivalent computations and reuses
  an existing value instead of calculating it again.
- **Dead code elimination (DCE)** removes work that cannot affect observable
  program behavior.

Passes report statistics to the driver, making optimization work observable
during compiler development without coupling user programs to a specific IR.

## Code generation

The default backend lowers optimized SSA MIR to C, combines it with the minimal runtime,
and invokes the host C toolchain to produce a native binary. This gives Vinglish
predictable low-level interoperability and a transparent bootstrap path.

```bash
vng build app.ving --output app
vng build app.ving --emit c
vng build app.ving --emit mir
```

An LLVM-oriented backend is also present for native IR generation work. Both
backends consume compiler-validated forms rather than reinterpreting source
text. The C backend is the practical default for inspectable output and direct
access to platform toolchains. It embeds versioned `vinglish:mir` comments for
every emitted block, instruction, and terminator. Standard C preprocessing
discards the comments; `vinglish-decompile` reads them to recover the MIR
identity graph without inferring semantics from arbitrary C syntax.

## Rust FFI and native capabilities

Vinglish exposes native capabilities through `foreign function` declarations.
On the Rust side, `#[vinglish_export]` generates a C-compatible bridge for
supported functions, avoiding hand-written glue at every call site.

The built-in UI module follows this model: Vinglish imports `std.ui`, while the
Rust runtime owns native windows and pixel buffers.

```vinglish
use std.ui

function open_window() returns number
begin
    return create_window("Vinglish", 640, 400)
end
```

## Tooling and semantic export

The compiler emits a stable JSON semantic document for external tools:

```bash
vng --emit-ir examples/fibonacci.ving > fibonacci.vinglish-export.json
```

This JSON document is deliberately not HIR. HIR remains an internal compiler
detail; the export format is a versioned boundary for integrations. See the
[Semantic Export Contract](architecture/semantic-export.md) for its vocabulary,
versioning rules, and ownership model.

## Design principles

- Keep semantic correctness ahead of optimization.
- Keep intermediate representations explicit and independently verifiable.
- Preserve readable source forms without imposing a runtime tax.
- Treat generated C and foreign boundaries as transparent systems interfaces.
- Version external contracts while keeping internal IRs free to evolve.
