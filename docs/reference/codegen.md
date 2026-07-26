# Code Generation

The Vinglish codegen crate provides two execution backends: a C code emitter and a tree-walk interpreter.

---

## Purpose

Transform optimized SSA MIR into executable form.

---

## Responsibilities

- Emit C source code from `MirModule<SsaValueId>`.
- Embed a compressed, integrity-checked MIR payload in the C output.
- Provide a tree-walk interpreter for `MirModule<SsaValueId>`.

---

## Design

### C Backend (`mir_codegen.rs`)

`emit_mir_c(module, symbols) -> Result<String, MirCEmitError>`

#### String Pool

Before emitting functions, `StringPool::collect()` scans all instructions for `Literal::Text` operands and assigns each unique string a numeric index. Strings are emitted as `static const char *const string_literal_N = "...";` with C escaping.

#### Forward Declarations

- `main` is declared as `int main(void);`.
- Other non-foreign functions: `static long fn_N(params);`.
- Foreign symbols found in `CallTarget::Foreign` are declared as `extern long symbol();`.

#### Function Bodies

Each function body:
1. Declares local variables not in the parameter list, initialized to zero (`0`, `0.0`, or `NULL` depending on type).
2. Labels each basic block as `bb_FN_BN:`.
3. Emits each instruction as a C statement.
4. Emits the terminator (`return`, `goto`, or `if/goto/else goto`).

#### Type Mapping

| Vinglish Type | C Type |
|---|---|
| `Int`, `Bool`, `Unit` | `int64_t` |
| `Float` | `double` |
| `Text` | `const char *` |
| Everything else | `uintptr_t` |

#### Instruction Mapping

| MIR Instruction | C Code |
|---|---|
| `Assign` | `v_N = operand` |
| `BinaryOp` | `v_N = left op right` |
| `Call(Direct)` | `v_N = fn_M(args)` |
| `Call(Foreign)` | `v_N = c_symbol(args)` |
| `HeapAllocate` / `StackAllocate` | `v_N = (long)(uintptr_t)calloc(1, size)` |
| `LoadField` | `v_N = *(long *)((unsigned char *)(uintptr_t)obj + offset)` |
| `StoreField` | `*(long *)((unsigned char *)(uintptr_t)v_N + offset) = val` |
| `Borrow` / `BorrowMut` / `Deref` | Identity (pass-through) |
| `Drop` | `(void)0` (no-op) |
| `Phi` | Takes the first operand's value |

#### Print Macros

```c
#define print(x) _Generic((x), const char*: printf("%s", x), ..., default: printf("%ld", (long)(x)))
#define println(x) _Generic((x), const char*: printf("%s\n", x), ..., default: printf("%ld\n", (long)(x)))
```

### Interpreter (`interp.rs`)

`Interpreter::run_module(module)` executes MIR directly via tree-walk interpretation. Operates on `Value` types. Errors are reported as `InterpError`.

### Backend Trait (`backend.rs`)

`Backend` trait. TODO: Describe implementation.

---

## Data Flow

**Input**: `MirModule<SsaValueId>` + `SymbolTable`

**Output (C backend)**: C source string (passed to system `cc`)

**Output (interpreter)**: Direct execution, return value

---

## Limitations

- All struct field access uses pointer arithmetic with byte offsets.
- `StackAllocate` uses `calloc` (heap allocation in practice).
- `Drop` is a no-op; no destructor calls.
- Phi nodes take only the first operand in C output.

---

## Related Components

- [Reference: MIR](mir.md)
- [Decompilation](../explanation/decompilation.md)
- [Compiler Pipeline](../explanation/compiler-pipeline.md)
