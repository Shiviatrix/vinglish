# CLI

The `vng` command-line tool compiles, runs, checks, and formats Vinglish source files.

---

## Purpose

Provide the primary user interface to the Vinglish compiler.

---

## Commands

| Command | Description |
|---|---|
| `vng build <file>` | Compile to native binary |
| `vng run <file>` | Compile and execute via interpreter |
| `vng check <file>` | Type-check without producing output |
| `vng fmt <files...>` | Format source files |
| `vng lsp` | Start the LSP server |
| `vng pkg init` | Initialize a new package |
| `vng pkg add <name> [url]` | Add a dependency |
| `vng benchmark <dir>` | Run benchmarks |
| `vng version` | Print version |
| `vng --emit-ir <file>` | Export stable semantic interchange JSON |

---

## Build Options

| Flag | Default | Description |
|---|---|---|
| `-o`, `--output` | `a.out` | Output binary path |
| `--backend` | `c` | Backend to use: `c`, `llvm`, or `interp` |
| `--emit` | — | Emit intermediate form: `c`, `mir`, `mir-before`, `mir-after`, `mir-stats`, `mir-diff`, `ssa`, `ownership`, `llvm` |

---

## Format Options

| Flag | Description |
|---|---|
| `--check` | Print diff instead of writing; exit non-zero if any file would change |

---

## Benchmark Options

| Flag | Default | Description |
|---|---|---|
| `--runs` | 5 | Number of iterations per benchmark |

---

## Environment Variables

| Variable | Description |
|---|---|
| `VINGLISH_ROOT` | Root directory for `std/` resolution |
| `CC` | C compiler to use (default: `cc`) |

---

## Exit Codes

| Code | Meaning |
|---|---|
| 0 | Success |
| 1 | Compilation error or runtime error |
| 2 | No command provided |

---

## Package Management

`vng pkg init` creates:
- `ving.toml` with `[package]` section
- `src/main.ving` with a stub main function

`vng pkg add <name> [url]` creates:
- `.ving_modules/<name>/` directory
- A stub module file `.ving_modules/<name>/<name>.ving`

No registry or version resolution exists.

---

## Related Components

- [Compiler Pipeline](../explanation/compiler-pipeline.md)
- [Architecture](../explanation/architecture.md)
