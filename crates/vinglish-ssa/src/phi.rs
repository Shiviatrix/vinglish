use crate::dominators::DominatorTree;
use std::collections::{HashMap, HashSet};
use vinglish_hir::symbol::VariableId;
use vinglish_mir::{BlockId, Instruction, MirFunction, Operand, Terminator};

fn find_globals(func: &MirFunction<VariableId>) -> HashSet<VariableId> {
    let mut globals = HashSet::new();

    for block in &func.blocks {
        let mut killed = HashSet::new();

        for instr in &block.instrs {
            let mut uses = Vec::new();

            match instr {
                Instruction::Assign(_, op) => uses.push(op),
                Instruction::LoadField(_, op, _) => uses.push(op),
                Instruction::StoreField(obj, _, val) => {
                    if !killed.contains(obj) { globals.insert(*obj); }
                    uses.push(val);
                }
                Instruction::Call(_, _, args) | Instruction::CallIntrinsic(_, _, args) => {
                    for arg in args { uses.push(arg); }
                }
                Instruction::BinaryOp(_, _, left, right) => {
                    uses.push(left); uses.push(right);
                }
                Instruction::UnaryOp(_, _, op) => uses.push(op),
                Instruction::Borrow(_, op) | Instruction::BorrowMut(_, op) => uses.push(op),
                Instruction::Deref(_, op, _) => uses.push(op),
                Instruction::StoreDeref(ptr, val) => { uses.push(ptr); uses.push(val); }
                Instruction::Drop(v) => { if !killed.contains(v) { globals.insert(*v); } }
                Instruction::Phi(_, args) => {
                    for (op, _) in args { uses.push(op); }
                }
                Instruction::ListNew(_, cap) => uses.push(cap),
                Instruction::ListGet(_, list, idx)
                | Instruction::ListBorrowGet(_, list, idx)
                | Instruction::ListBorrowMutGet(_, list, idx) => {
                    uses.push(list); uses.push(idx);
                }
                Instruction::ListSet(list, idx, val) => {
                    uses.push(list); uses.push(idx); uses.push(val);
                }
                Instruction::ListLen(_, list) => uses.push(list),
                Instruction::ListPush(list, val) => { uses.push(list); uses.push(val); }
                Instruction::ListPop(_, list) => uses.push(list),
                Instruction::HeapAllocate(_, _) | Instruction::StackAllocate(_, _) => {}
            }

            for op in uses {
                if let Operand::Var(v) = op {
                    if !killed.contains(v) {
                        globals.insert(*v);
                    }
                }
            }

            match instr {
                Instruction::Assign(dest, _)
                | Instruction::LoadField(dest, _, _)
                | Instruction::Call(dest, _, _)
                | Instruction::CallIntrinsic(dest, _, _)
                | Instruction::HeapAllocate(dest, _)
                | Instruction::StackAllocate(dest, _)
                | Instruction::BinaryOp(dest, _, _, _)
                | Instruction::UnaryOp(dest, _, _)
                | Instruction::Borrow(dest, _)
                | Instruction::BorrowMut(dest, _)
                | Instruction::Deref(dest, _, _)
                | Instruction::ListNew(dest, _)
                | Instruction::ListGet(dest, _, _)
                | Instruction::ListBorrowGet(dest, _, _)
                | Instruction::ListBorrowMutGet(dest, _, _)
                | Instruction::ListLen(dest, _)
                | Instruction::ListPop(dest, _)
                | Instruction::Phi(dest, _) => {
                    killed.insert(*dest);
                }
                _ => {}
            }
        }

        let mut term_uses = Vec::new();
        match &block.terminator {
            Terminator::Return(Some(op)) => term_uses.push(op),
            Terminator::Branch(cond, _, _) => term_uses.push(cond),
            _ => {}
        }
        for op in term_uses {
            if let Operand::Var(v) = op {
                if !killed.contains(v) {
                    globals.insert(*v);
                }
            }
        }
    }

    globals
}

pub fn insert_phi_nodes(func: &mut MirFunction<VariableId>, dom_tree: &DominatorTree) {
    let mut defs: HashMap<VariableId, HashSet<BlockId>> = HashMap::new();

    for block in &func.blocks {
        for instr in &block.instrs {
            match instr {
                Instruction::<VariableId>::Assign(dest, _)
                | Instruction::<VariableId>::LoadField(dest, _, _)
                | Instruction::<VariableId>::Call(dest, _, _)
                | Instruction::<VariableId>::CallIntrinsic(dest, _, _)
                | Instruction::<VariableId>::HeapAllocate(dest, _)
                | Instruction::<VariableId>::StackAllocate(dest, _)
                | Instruction::<VariableId>::BinaryOp(dest, _, _, _)
                | Instruction::<VariableId>::UnaryOp(dest, _, _)
                | Instruction::<VariableId>::Borrow(dest, _)
                | Instruction::<VariableId>::BorrowMut(dest, _)
                | Instruction::<VariableId>::Deref(dest, _, _)
                | Instruction::<VariableId>::ListNew(dest, _)
                | Instruction::<VariableId>::ListGet(dest, _, _)
                | Instruction::<VariableId>::ListBorrowGet(dest, _, _)
                | Instruction::<VariableId>::ListBorrowMutGet(dest, _, _)
                | Instruction::<VariableId>::ListLen(dest, _)
                | Instruction::<VariableId>::ListPop(dest, _) => {
                    defs.entry(*dest).or_default().insert(block.id);
                }
                Instruction::<VariableId>::StoreField(_, _, _)
                | Instruction::<VariableId>::ListSet(_, _, _)
                | Instruction::<VariableId>::ListPush(_, _)
                | Instruction::<VariableId>::StoreDeref(_, _)
                | Instruction::<VariableId>::Drop(_)
                | Instruction::<VariableId>::Phi(_, _) => {}
            }
        }
    }

    let globals = find_globals(func);

    // 2. Iterate each variable and insert Phi nodes using Iterated Dominance Frontiers
    for (&var, var_defs) in &defs {
        if !globals.contains(&var) {
            continue;
        }
        let mut worklist: Vec<BlockId> = var_defs.iter().copied().collect();
        let mut in_worklist: HashSet<BlockId> = var_defs.clone();
        let mut has_phi: HashSet<BlockId> = HashSet::new();

        while let Some(x) = worklist.pop() {
            in_worklist.remove(&x);

            if let Some(df) = dom_tree.dominance_frontiers.get(&x) {
                for &y in df {
                    if !has_phi.contains(&y) {
                        // Insert Phi node in block y
                        if let Some(block) = func.blocks.iter_mut().find(|b| b.id == y) {
                            // We initialize with empty predecessors; they will be filled during renaming.
                            block
                                .instrs
                                .insert(0, Instruction::<VariableId>::Phi(var, Vec::new()));
                        }
                        has_phi.insert(y);

                        // If y doesn't originally define the variable, we need to add it to the worklist
                        // because this Phi node is a new definition.
                        if !var_defs.contains(&y) && !in_worklist.contains(&y) {
                            worklist.push(y);
                            in_worklist.insert(y);
                        }
                    }
                }
            }
        }
    }
}
