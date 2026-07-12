use std::collections::{HashMap, HashSet};
use eng_mir::{MirFunction, BlockId, Instruction};
use eng_hir::symbol::VariableId;
use crate::dominators::DominatorTree;

pub fn insert_phi_nodes(func: &mut MirFunction<VariableId>, dom_tree: &DominatorTree) {
    // 1. Find blocks where each variable is assigned
    let mut defs: HashMap<VariableId, HashSet<BlockId>> = HashMap::new();
    
    for block in &func.blocks {
        for instr in &block.instrs {
            match instr {
                Instruction::<VariableId>::Assign(dest, _) |
                Instruction::<VariableId>::LoadField(dest, _, _) |
                Instruction::<VariableId>::Call(dest, _, _) |
                Instruction::<VariableId>::HeapAllocate(dest, _) |
                Instruction::<VariableId>::StackAllocate(dest, _) |
                Instruction::<VariableId>::BinaryOp(dest, _, _, _) |
                Instruction::<VariableId>::UnaryOp(dest, _, _) |
                Instruction::<VariableId>::Borrow(dest, _) |
                Instruction::<VariableId>::BorrowMut(dest, _) |
                Instruction::<VariableId>::Deref(dest, _, _) => {
                    defs.entry(*dest).or_default().insert(block.id);
                }
                Instruction::<VariableId>::StoreField(_, _, _) |
                Instruction::<VariableId>::Drop(_) |
                Instruction::<VariableId>::Phi(_, _) => {}
            }
        }
    }

    // 2. Iterate each variable and insert Phi nodes using Iterated Dominance Frontiers
    for (&var, var_defs) in &defs {
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
                            block.instrs.insert(0, Instruction::<VariableId>::Phi(var, Vec::new()));
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
