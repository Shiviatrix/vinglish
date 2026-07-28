# Compiler Pipeline

The Vinglish compiler transforms `.ving` source files into native binaries through a sequence of lowering passes, each consuming the output of the previous stage. This document explains the purpose and behavior of each stage.

---

## Purpose

Describe the complete path from source text to native binary, as implemented in `compile_project()` and `cmd_build()` in `vinglish-cli/src/main.rs`.

---

## Stages

### 1. Module Loading

`load_module_graph()` recursively resolves `use` declarations. For each referenced module:
1. If the path starts with `std`, resolve relative to `$VINGLISH_ROOT/std/`.
2. Otherwise, check `.ving_modules/<package>/` first.
3. Fall back to a path relative to the current file.
4. The file extension `.ving` is appended automatically.

Parsed modules are stored in a `HashMap<String, (Module, String, PathBuf)>`. Dependencies are tracked in a separate `HashMap<String, Vec<String>>`.

### 2. Topological Sort

`topological_sort()` orders modules so that dependencies are compiled before dependents. Cyclic dependencies produce an error.

### 3. Per-Module Compilation

For each module in topological order:

#### 3a. Lexing

`vinglish_lexer::tokenize(&src)` returns `(Vec<Spanned<Token>>, Vec<LexError>)`.

The lexer operates line-by-line:
- Measures leading whitespace per line (tab = 4 spaces).
- Emits `Indent` when indentation increases, `Dedent` when it decreases.
- Emits `Newline` at the end of each physical line.
- Closes remaining open indent blocks at end of input.
- Appends `EOF`.

Within each line, `lex_line()` produces tokens for: string literals (with escape sequences), integer/float literals (with `_` separators), identifiers and keywords (via `Token::from_word()`), operators, and punctuation. Comments (`--` or `#`) terminate a line.

#### 3b. Parsing

`vinglish_parser::parse(&tokens)` returns `(ast::Module, Vec<ParseError>)`.

The parser is recursive-descent. Parse errors are enriched with diagnostics via `vinglish_diagnostics::Diagnostic` and rendered to stderr.

#### 3c. Name Resolution

`NameResolutionPass::run(&ast, &mut ctx)` populates the `CompilerContext` with symbol definitions.

#### 3d. Type Inference

`TypeInferencePass::run_with_healing(&mut ast, &mut ctx)` performs type inference and returns a `hir::Module`. On type mismatches, the healer attempts bounded AST repairs (auto-deref, to_text insertion). Successful repairs are recorded as `HealingWarning` values in the context.

#### 3e. HIR Validation

`HirValidatorPass::validate(&mut ctx, &hir)` checks the typed HIR for consistency.

#### 3f. Ownership Checking (AST)

`vinglish_ownership::check_module(&ast)` performs ownership checking on the AST. Returns a list of ownership errors.

#### 3g. MIR Lowering

`MirLowerer::lower_module(&hir)` converts the HIR into `MirModule<VariableId>`. Functions from all modules are accumulated into a single `MirModule`.

### 4. MIR Validation

`MirValidatorPass::validate(&symbol_table, &mir_module)` checks the MIR for structural correctness.

### 5. Pre-SSA Optimization

`vinglish_opt::pre_ssa_pipeline()` runs:
1. Dead Code Elimination
2. CFG Simplification

MIR validation runs after each pass.

### 6. SSA Conversion

`SSAConversionPass::run(mir_module, &mut symbol_table)` converts `MirModule<VariableId>` to `MirModule<SsaValueId>`:
1. Compute dominator tree per function.
2. Insert phi nodes at dominance frontiers.
3. Rename variables to SSA form.
4. Convert all `VariableId` references to `SsaValueId`.

### 7. SSA Validation

`SSAValidator::validate(&ssa_module)` verifies SSA properties hold.

### 8. Post-SSA Optimization

`vinglish_opt::post_ssa_pipeline()` runs:
1. Constant Folding
2. Constant Propagation
3. Copy Propagation
4. Global Value Numbering
5. Dead Code Elimination
6. CFG Simplification

MIR validation runs after each pass.

### 9. Ownership Analysis (MIR)

`OwnershipAnalysisPass::run(&mut ssa_module, &symbol_table)` builds an `OwnershipGraph`. `OwnershipValidator::validate()` checks the graph for violations.

### 10. Code Generation

Depending on the `--backend` flag:

**C backend** (default): `emit_mir_c(&ssa_module, &symbol_table)` generates C source. The C source includes:
- `#include` directives for stdint, stdio, stdlib.
- `_Generic` macros for `print` / `println`.
- A static string pool for string literals.
- Forward declarations for all non-foreign functions.
- `extern` declarations for foreign functions.
- Function bodies with `goto`-based control flow.
- A compressed MIR payload as a trailing comment.

**LLVM backend**: `vinglish_llvm::compile_to_executable()` generates LLVM IR and invokes LLVM tools.

### 11. Native Compilation

For the C backend, the system C compiler is invoked:
```
cc -O2 -Wno-int-conversion -o <output> <generated.c> rt/*.c [-lvinglish_rt ...]
```

If `rt_rust/Cargo.toml` exists, `cargo build --release` is run first, and the resulting static library is linked.

---

## Related Components

- [Architecture](architecture.md)
- [Reference: MIR](../reference/mir.md)
- [Reference: Optimizations](../reference/optimizations.md)
- [Reference: Code Generation](../reference/codegen.md)
