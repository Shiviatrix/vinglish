# MIR

The Mid-Level Intermediate Representation is a CFG-based IR used for optimization and code generation.

---

## Purpose

Provide a lower-level representation suitable for SSA conversion, optimization passes, ownership analysis, and code generation.

---

## Responsibilities

- Represent programs as functions containing basic blocks.
- Each basic block has a sequence of instructions and a terminator.
- Support both pre-SSA (`VariableId`) and post-SSA (`SsaValueId`) forms via a type parameter `V`.

---

## Design

Defined in `vinglish-mir/src/lib.rs`.

### Module Structure

```
MirModule<V>
  └── functions: Vec<MirFunction<V>>
        ├── id: FunctionId
        ├── name: String
        ├── is_foreign: bool
        ├── params: Vec<V>
        ├── locals: Vec<V>
        └── blocks: Vec<BasicBlock<V>>
              ├── id: BlockId
              ├── instrs: Vec<Instruction<V>>
              └── terminator: Terminator<V>
```

### Instructions

| Variant | Semantics |
|---|---|
| `Assign(dest, operand)` | `dest = operand` |
| `LoadField(dest, object, FieldAccess)` | Load field at byte offset from object |
| `StoreField(object, FieldAccess, value)` | Store value to field at byte offset |
| `Call(dest, CallTarget, args)` | Function call (direct or foreign) |
| `CallIntrinsic(dest, name, args)` | Intrinsic call by name |
| `HeapAllocate(dest, AllocationLayout)` | Heap allocation |
| `StackAllocate(dest, AllocationLayout)` | Stack allocation |
| `BinaryOp(dest, BinOp, left, right)` | Binary operation |
| `UnaryOp(dest, UnOp, operand)` | Unary operation |
| `Borrow(dest, operand)` | Immutable borrow |
| `BorrowMut(dest, operand)` | Mutable borrow |
| `Deref(dest, operand, TypeId)` | Dereference |
| `Drop(var)` | Drop a value |
| `Phi(dest, Vec<(Operand, BlockId)>)` | SSA phi node |

### Terminators

| Variant | Semantics |
|---|---|
| `Return(Option<Operand>)` | Return from function |
| `Jump(BlockId)` | Unconditional jump |
| `Branch(Operand, BlockId, BlockId)` | Conditional branch (true, false) |

### Operands

| Variant | Description |
|---|---|
| `Constant(Literal)` | Compile-time constant |
| `Var(V)` | Variable/SSA value reference |

### Call Targets

| Variant | Description |
|---|---|
| `Direct(FunctionId)` | Call to a function defined in the module |
| `Foreign { c_symbol }` | Call to a foreign (C) function |

### Field Access

`FieldAccess { field_id: FieldId, byte_offset: u32, layout: TypeId }` — carries the ABI offset computed at lowering time.

### Allocation Layout

`AllocationLayout { layout: TypeId, size: u32, align: u32 }` — carries size and alignment for allocation instructions.

---

## MIR Validation

`MirValidatorPass` in `vinglish-mir/src/validator.rs` checks structural correctness of the MIR module after each optimization pass.

---

## Display

All MIR types implement `Display`:
- Functions: `fn name (fn_N) { ... }`
- Blocks: `bbN:`
- Instructions: `dest = operand op operand`
- Terminators: `return`, `jump bbN`, `branch cond ? bbT : bbF`

The `--emit ssa`, `--emit mir`, and `--emit mir-diff` CLI flags print MIR to stdout.

---

## Related Components

- [Reference: SSA](ssa.md)
- [Reference: Optimizations](optimizations.md)
- [Reference: Code Generation](codegen.md)
