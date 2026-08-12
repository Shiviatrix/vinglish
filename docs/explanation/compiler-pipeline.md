# Compiler Pipeline

This document explains how Vinglish takes your `.ving` code and compiles it into a native executable. We run a sequence of lowering passes, where the output of one step feeds directly into the next. 

---

## 1. Module Loading & Sorting

When you compile a project, `load_module_graph()` recursively resolves your `use` statements.
- If it sees `std`, it loads it from the standard library folder.
- Otherwise, it checks your `.ving_modules` (for packages) or just looks for local files relative to the current one.

Once we have all the modules, we run `topological_sort()`. This ensures we compile dependencies *before* the code that depends on them. If you accidentally created a circular dependency, this step reports an error.

---

## 2. Compiling Each Module

We run the frontend passes on every module one by one:

### Lexing & Parsing
The lexer (`vinglish_lexer`) processes your code line-by-line. Because Vinglish is whitespace-sensitive, the lexer counts your leading spaces (4 spaces = 1 indent) and emits synthetic `Indent` and `Dedent` tokens. It also handles string escapes, numbers with underscores, and maps English-like keywords.

The parser (`vinglish_parser`) takes those tokens and builds an Abstract Syntax Tree (AST) using standard recursive descent.

### Name Resolution & Type Inference
`NameResolutionPass` populates our `CompilerContext` with all the symbols it can find.

Then `TypeInferencePass` executes. We run it in "healing mode": if you provided a mismatched type—like passing a raw string where a `Text` object is expected—the compiler runs a bounded search to attempt to auto-fix it (e.g., by implicitly inserting a `.to_text()` call). It logs these fixes as warnings but continues compiling.

### AST-level Checks
We run `vinglish_ownership` over the AST to check for basic memory safety violations before lowering to the intermediate representation.

### Lowering to MIR
Once the AST is typed and valid, `MirLowerer` flattens it down into our Mid-level Intermediate Representation (MIR). At this point, we consolidate all the functions from all the modules into one `MirModule`.

---

## 3. The Optimizer (SSA & MIR)

Now that everything is in MIR, we can optimize it.

1. **Pre-SSA Opts:** We do a quick pass of Dead Code Elimination and CFG Simplification to reduce instruction count early.
2. **SSA Conversion:** We run `SSAConversionPass`. This is a critical transformation. We calculate the dominator tree for each function, drop `phi` nodes at the dominance frontiers, and rename all the variables so every variable is assigned exactly once (Static Single Assignment form).
3. **Post-SSA Opts:** Now that we're in SSA, the compiler runs advanced passes: Constant Folding, Constant Propagation, Copy Propagation, and Global Value Numbering (GVN), followed by another round of dead code elimination. 
4. **MIR Ownership:** We run `OwnershipAnalysisPass` to build an ownership graph and validate that you aren't committing memory violations, like double-freeing memory or using a value after you've moved it.

*(Note: We run an `SSAValidator` and `MirValidatorPass` between almost every step here just to make sure our own optimizer didn't break the code).*

---

## 4. Code Generation

If validation succeeds, the compiler emits C code.

`emit_mir_c()` generates a `.c` file that includes:
- Standard `#include`s and a static string pool.
- Forward declarations so C doesn't complain about function order.
- Function bodies packed with `goto` statements (since we lowered control flow to raw branches).

**A notable feature:** We take your entire optimized MIR payload, compress it, hash it (SHA-256), and embed it at the very bottom of the generated C file as a base64 comment. This lets us decompile the C code back into Vinglish MIR later.

---

## 5. Native Compilation

Finally, the generated C file is passed to your system's C compiler (like `clang` or `gcc`):

```bash
cc -O2 -Wno-int-conversion -o <output> <generated.c> rt/*.c
```

We link it against our C runtime (`rt/*.c`). If UI components are used, we'll also build the Rust runtime (`rt_rust`) and link that static library. 

This produces the final native binary.
