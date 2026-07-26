# ADR-0001: C as Primary Code Generation Target

## Status

Accepted.

## Context

The Vinglish compiler needs a backend that produces native executables. Options considered include direct assembly emission, LLVM IR, and C source.

## Decision

The primary backend emits C source code via `emit_mir_c()` in `vinglish-codegen/src/mir_codegen.rs`. The generated C is compiled by the system C compiler (`cc`).

Evidence from the codebase:
- `cmd_build()` in `main.rs` defaults to `--backend c`.
- Generated C uses `_Generic` macros for print/println, `calloc` for allocation, `goto` for control flow, and pointer arithmetic for struct field access.
- An experimental LLVM backend exists in `vinglish-llvm/` but is not the default.

## Consequences

- **Portability**: C compiles on any platform with a C compiler.
- **Optimization**: The C compiler (`-O2`) applies its own optimization passes.
- **Debugging**: Generated C is human-readable and inspectable.
- **Limitation**: Type information is reduced to `int64_t`, `double`, `const char *`, and `uintptr_t`.
- **Limitation**: Stack allocations use `calloc` (heap allocation) because the C backend cannot express variable-length stack allocations portably.
- **Limitation**: `Drop` emits `(void)0` — no destructor calls.

## Related Files

- [mir_codegen.rs](../../crates/vinglish-codegen/src/mir_codegen.rs)
- [main.rs](../../crates/vinglish-cli/src/main.rs) (`cmd_build`)
