<div align="center">
  <img src="logos/vinglish-icon-color.svg" alt="Vinglish Icon" width="128" height="128" />
</div>

**Vinglish** is a systems programming language with an English-inspired syntax. 

The compiler is implemented in Rust to leverage its memory safety guarantees. The compilation pipeline lowers Vinglish source code to a Static Single Assignment (SSA) MIR, executes optimization passes (DCE, GVN, constant folding), and emits C source code.

### Architectural Decisions

**C Backend Integration:** The backend currently emits C source code rather than LLVM IR. While an LLVM backend was considered, emitting C provided a simpler integration path without the complexity of maintaining Rust bindings to LLVM.

**Embedded MIR Payloads:** The compiler takes the optimized MIR payload, compresses it with zlib, signs it with a SHA-256 hash, and embeds it as a base64 comment at the end of the generated `.c` file. This decision was made to allow the `vng decompile` command to function without requiring a separate metadata artifact alongside the binary. While this increases the size of the generated C file, it ensures the decompilation process remains hermetic.

**Type Healing:** The type system implements a mechanism known as "type healing." If a type mismatch occurs (e.g., passing an `int` where a `string` is expected), the type checker performs a bounded search and mutates the AST to insert a `to_text()` call. It will also auto-dereference pointers to resolve mismatches. This approach prioritizes developer ergonomics by implicitly resolving common type discrepancies.

### Example

The following demonstrates the auto-healing behavior in practice:

```vinglish
let entropy be 0.82

if entropy is above 0.50 {
    # The compiler auto-heals the float into a string here
    print("Entropy is " + entropy)
}
```

### Current Limitations

The compiler is currently in an experimental state and is not suitable for production use. The C backend currently treats most primitives as `int64_t` (`long`). Additionally, stack allocations currently fall back to `calloc` under the hood. While the MIR includes a `Drop` instruction for memory management, the C backend currently emits a no-op `(void)0` for it, resulting in memory leaks across all compiled programs. Finally, the package manager (`vng pkg`) builds project scaffolding, but a package registry has not yet been implemented.

### Compilation Instructions

Using the compiler requires Rust (2024 edition) and a C compiler (GCC or Clang).

```sh
cargo install --path crates/vinglish
vng run path/to/file.ving
```

To compile to a native binary via the C backend:
```sh
vng build path/to/file.ving --output my_program --backend c
```

### Repository Structure

The compiler architecture is modularized across 19 crates. The execution pipeline follows: `Parser -> HIR -> MIR -> SSA -> C backend`. 

Documentation has been exported to HTML. For full technical details on the architecture and passes, please refer to:
- [Architecture Notes](docs/explanation/architecture.html)
- [Pipeline details](docs/explanation/compiler-pipeline.html)

License is MIT.
