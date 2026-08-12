# Build and Run

This guide covers how to use the `vng` CLI to compile and run Vinglish programs.

---

## Building a Native Binary

To compile a `.ving` file into an executable:

```sh
vng build <file.ving> --output <binary>
```

By default, this emits C code and passes it to your system's `cc` compiler. If you prefer to use the experimental LLVM backend, pass `--backend llvm`.

**Diagnostic Output:** The `--emit` flag stops the compiler early and outputs the internal state, which is useful for debugging:
- `--emit c`: Dump the generated C source
- `--emit mir`: Dump the optimized MIR
- `--emit ssa`: Dump the SSA form
- `--emit mir-diff`: Shows you exactly what the optimization passes changed
- `--emit ownership`: Dumps the ownership graph

---

## Running without Compiling

To execute code directly without invoking a C compiler, use the built-in tree-walk interpreter:

```sh
vng run <file.ving>
```

---

## Type-Checking

To validate code without compiling or executing it: 

```sh
vng check <file.ving>
```

This runs the lexer, parser, type inference, and ownership checks. It reports any syntax or type errors and exits silently on success.

---

## Formatting

Vinglish includes a built-in code formatter to ensure consistent style:

```sh
vng fmt <file.ving>
```

To verify formatting in CI environments (prints a diff and exits with a non-zero status if the file requires formatting), use:

```sh
vng fmt --check <file.ving>
```

---

## Environment Variables

### 1. The Standard Library Path
For the compiler to resolve `std` imports, you must provide the path to the Vinglish repository:

```sh
export VINGLISH_ROOT=/path/to/vinglish
vng build src/main.ving
```

### 2. Custom C Compiler
By default, Vinglish just calls `cc`. If you want to use `gcc` or `clang` specifically, just set the `CC` variable:

```sh
CC=clang vng build file.ving
```
