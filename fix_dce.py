import re

with open('crates/vinglish-opt/src/dce.rs', 'r') as f:
    content = f.read()

old_cond = """                    if let Instruction::<V>::Assign(dest, _)
                    | Instruction::<V>::BinaryOp(dest, _, _, _)
                    | Instruction::<V>::UnaryOp(dest, _, _)
                    | Instruction::<V>::LoadField(dest, _, _)
                    | Instruction::<V>::Call(dest, _, _)
                    | Instruction::<V>::CallIntrinsic(dest, _, _)
                    | Instruction::<V>::Borrow(dest, _)
                    | Instruction::<V>::BorrowMut(dest, _)
                    | Instruction::<V>::Deref(dest, _, _)
                    | Instruction::<V>::HeapAllocate(dest, _)
                    | Instruction::<V>::StackAllocate(dest, _)
                    | Instruction::<V>::Phi(dest, _) = instr"""
new_cond = """                    if let Instruction::<V>::Assign(dest, _)
                    | Instruction::<V>::BinaryOp(dest, _, _, _)
                    | Instruction::<V>::UnaryOp(dest, _, _)
                    | Instruction::<V>::LoadField(dest, _, _)
                    | Instruction::<V>::Deref(dest, _, _)
                    | Instruction::<V>::HeapAllocate(dest, _)
                    | Instruction::<V>::StackAllocate(dest, _)
                    | Instruction::<V>::Phi(dest, _) = instr"""

content = content.replace(old_cond, new_cond)

with open('crates/vinglish-opt/src/dce.rs', 'w') as f:
    f.write(content)
