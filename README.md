<div align="center">
  <img src="logos/vinglish-icon-color.svg" alt="Vinglish Icon" width="128" height="128" />
</div>

**Vinglish** is a systems programming language with an English-inspired syntax. 

I wrote the compiler in Rust because I didn't want to deal with C++ segfaults. It takes Vinglish code, lowers it to a Static Single Assignment (SSA) MIR, runs standard optimization passes (DCE, GVN, constant folding), and spits out C code.

### A note on the compiler

The backend generates C. I looked into LLVM, but the Rust bindings were miserable to set up, so I stuck with emitting C source code that GCC or Clang can compile. 

One weird decision I made: the compiler takes the optimized MIR payload, compresses it with zlib, signs it with a SHA-256 hash, and dumps it as a massive base64 comment at the very bottom of the generated `.c` file. 
Why? Because I wanted a `vng decompile` command to work without having to ship a separate metadata artifact alongside the binary. It definitely bloats the C file, but it works.

Also, the type system has "type healing." If you pass an `int` where a `string` is expected, instead of immediately failing, the type checker does a bounded search and literally mutates your AST to insert a `to_text()` call. It will also auto-dereference pointers if it thinks it can fix a mismatch. It's a bit of a hack, but it saves keystrokes.

### Example

Here's what actual code looks like. 

```vinglish
let entropy be 0.82

if entropy is above 0.50 {
    # The compiler auto-heals the float into a string here
    print("Entropy is " + entropy)
}
```

### Current state (It's rough)

Do not use this for anything serious yet. 

- The C backend currently treats almost everything as an `int64_t` (`long`). 
- "Stack" allocations actually just call `calloc` under the hood right now.
- The MIR has a `Drop` instruction for memory management, but the C backend just emits a no-op `(void)0` for it. So yes, it leaks memory.
- `vng pkg` builds project scaffolding, but there is no package registry. 

### Running it

If you still want to run it, you need Rust (2024 edition) and a C compiler. I'm not going to explain how to install Rust.

```sh
cargo install --path crates/vinglish
vng run path/to/file.ving
```

Or just compile it to C:
```sh
vng build path/to/file.ving --output my_program --backend c
```

### Architecture

If you actually want to read the compiler source, it's split across 19 crates to keep things isolated. 
The pipeline is basically: `Parser -> HIR -> MIR -> SSA -> C backend`. 

Check the [docs/](docs/) folder. Note: I converted all the markdown docs to HTML because I wanted them to match the retro website. You'll have to read the `.html` files or look at the live site. 

- [Architecture Notes](docs/explanation/architecture.html)
- [Pipeline details](docs/explanation/compiler-pipeline.html)

License is MIT.
