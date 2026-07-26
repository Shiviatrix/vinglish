# Emit and Inspect MIR

How to inspect the compiler's intermediate representations.

---

## Print MIR Before Optimization

```sh
vng build file.ving --emit mir-before
```

Prints the MIR as produced by `MirLowerer`, before any optimization passes.

---

## Print MIR After Optimization

```sh
vng build file.ving --emit mir
```

Prints the SSA MIR after all optimization passes.

---

## Print Before/After Diff

```sh
vng build file.ving --emit mir-diff
```

Prints the MIR before optimization, followed by the MIR after optimization.

---

## Print Optimization Statistics

```sh
vng build file.ving --emit mir-stats
```

Prints:
- Total variables
- Number of functions
- Merged blocks (CFG simplification)
- Folded constants
- GVN eliminated values

---

## Print SSA Form

```sh
vng build file.ving --emit ssa
```

---

## Print Ownership Graph

```sh
vng build file.ving --emit ownership
```

---

## Print LLVM IR

```sh
vng build file.ving --emit llvm
```

Requires the LLVM backend.

---

## Export Semantic IR (JSON)

```sh
vng --emit-ir file.ving
```

Prints the HIR in JSON interchange format to stdout.
