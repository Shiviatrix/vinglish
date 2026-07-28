# Build and Run

How to compile and run Vinglish programs.

---

## Build a Native Binary

```sh
vng build <file.ving> --output <binary> --backend c
```

The `--backend` flag selects the code generation backend:

| Backend | Description |
|---|---|
| `c` (default) | Emit C, compile with system `cc` |
| `llvm` | Emit LLVM IR, compile with LLVM tools |

The `--emit` flag stops early and prints an intermediate representation:

| Value | Description |
|---|---|
| `c` | Generated C source |
| `mir-before` | MIR before optimization |
| `mir` / `mir-after` | MIR after optimization |
| `ssa` | SSA form after optimization |
| `mir-stats` | Optimization statistics |
| `mir-diff` | Before and after MIR |
| `ownership` | Ownership graph |
| `llvm` | LLVM IR |

---

## Run with the Interpreter

```sh
vng run <file.ving>
```

Compiles through the full pipeline and executes via the tree-walk interpreter. No C compiler required.

---

## Type-Check Only

```sh
vng check <file.ving>
```

Runs lexing, parsing, name resolution, type inference, and ownership checking. Reports errors without producing output.

---

## Format Source Code

```sh
vng fmt <file.ving>
```

Formats the file in place.

```sh
vng fmt --check <file.ving>
```

Prints a diff and exits non-zero if the file would change.

---

## Set the Standard Library Path

```sh
export VINGLISH_ROOT=/path/to/vinglish
vng build src/main.ving
```

The `VINGLISH_ROOT` environment variable points to the repository root. This is required for resolving `use std.*` imports.

---

## Select a C Compiler

```sh
CC=gcc vng build file.ving
```
