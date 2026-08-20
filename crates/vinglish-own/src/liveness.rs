use std::collections::{HashMap, HashSet};
use vinglish_hir::symbol::SsaValueId;
use vinglish_mir::{BlockId, Instruction, MirFunction, Operand, Terminator};

#[derive(Debug, Default)]
pub struct LivenessInfo {
    pub live_in: HashMap<BlockId, HashSet<SsaValueId>>,
    pub live_out: HashMap<BlockId, HashSet<SsaValueId>>,
}

/// Compute liveness for a MIR function.
pub fn compute_liveness(func: &MirFunction<SsaValueId>) -> LivenessInfo {
    let mut info = LivenessInfo::default();
    let mut defs: HashMap<BlockId, HashSet<SsaValueId>> = HashMap::new();
    let mut uses: HashMap<BlockId, HashSet<SsaValueId>> = HashMap::new();

    // 1. Compute local DEFs and USEs for each basic block
    for block in &func.blocks {
        let mut block_def = HashSet::new();
        let mut block_use = HashSet::new();

        let mut add_use = |var: SsaValueId, b_def: &HashSet<SsaValueId>, b_use: &mut HashSet<SsaValueId>| {
            if !b_def.contains(&var) {
                b_use.insert(var);
            }
        };

        let mut add_def = |var: SsaValueId, b_def: &mut HashSet<SsaValueId>| {
            b_def.insert(var);
        };

        let mut process_operand = |op: &Operand<SsaValueId>, b_def: &HashSet<SsaValueId>, b_use: &mut HashSet<SsaValueId>| {
            if let Operand::Var(var) = op {
                add_use(*var, b_def, b_use);
            }
        };

        for instr in &block.instrs {
            match instr {
                Instruction::Assign(dest, op) => {
                    process_operand(op, &block_def, &mut block_use);
                    add_def(*dest, &mut block_def);
                }
                Instruction::LoadField(dest, op, _) => {
                    process_operand(op, &block_def, &mut block_use);
                    add_def(*dest, &mut block_def);
                }
                Instruction::StoreField(obj, _, op) => {
                    add_use(*obj, &block_def, &mut block_use);
                    process_operand(op, &block_def, &mut block_use);
                }
                Instruction::Call(dest, _, args) | Instruction::CallIntrinsic(dest, _, args) => {
                    for arg in args {
                        process_operand(arg, &block_def, &mut block_use);
                    }
                    add_def(*dest, &mut block_def);
                }
                Instruction::HeapAllocate(dest, _) | Instruction::StackAllocate(dest, _) => {
                    add_def(*dest, &mut block_def);
                }
                Instruction::BinaryOp(dest, _, left, right) => {
                    process_operand(left, &block_def, &mut block_use);
                    process_operand(right, &block_def, &mut block_use);
                    add_def(*dest, &mut block_def);
                }
                Instruction::UnaryOp(dest, _, op) => {
                    process_operand(op, &block_def, &mut block_use);
                    add_def(*dest, &mut block_def);
                }
                Instruction::Borrow(dest, op) | Instruction::BorrowMut(dest, op) => {
                    process_operand(op, &block_def, &mut block_use);
                    add_def(*dest, &mut block_def);
                }
                Instruction::Deref(dest, op, _) => {
                    process_operand(op, &block_def, &mut block_use);
                    add_def(*dest, &mut block_def);
                }
                Instruction::StoreDeref(ptr, val) => {
                    process_operand(ptr, &block_def, &mut block_use);
                    process_operand(val, &block_def, &mut block_use);
                }
                Instruction::Drop(var) => {
                    add_use(*var, &block_def, &mut block_use);
                }
                Instruction::Phi(dest, args) => {
                    for (op, _) in args {
                        process_operand(op, &block_def, &mut block_use);
                    }
                    add_def(*dest, &mut block_def);
                }
                Instruction::ListNew(dest, cap) => {
                    process_operand(cap, &block_def, &mut block_use);
                    add_def(*dest, &mut block_def);
                }
                Instruction::ListGet(dest, list, idx)
                | Instruction::ListBorrowGet(dest, list, idx)
                | Instruction::ListBorrowMutGet(dest, list, idx) => {
                    process_operand(list, &block_def, &mut block_use);
                    process_operand(idx, &block_def, &mut block_use);
                    add_def(*dest, &mut block_def);
                }
                Instruction::ListSet(list, idx, val) => {
                    process_operand(list, &block_def, &mut block_use);
                    process_operand(idx, &block_def, &mut block_use);
                    process_operand(val, &block_def, &mut block_use);
                }
                Instruction::ListLen(dest, list) => {
                    process_operand(list, &block_def, &mut block_use);
                    add_def(*dest, &mut block_def);
                }
                Instruction::ListPush(list, val) => {
                    process_operand(list, &block_def, &mut block_use);
                    process_operand(val, &block_def, &mut block_use);
                }
                Instruction::ListPop(dest, list) => {
                    process_operand(list, &block_def, &mut block_use);
                    add_def(*dest, &mut block_def);
                }
            }
        }

        match &block.terminator {
            Terminator::Return(Some(op)) => process_operand(op, &block_def, &mut block_use),
            Terminator::Return(None) | Terminator::Jump(_) => {}
            Terminator::Branch(cond, _, _) => process_operand(cond, &block_def, &mut block_use),
        }

        defs.insert(block.id, block_def);
        uses.insert(block.id, block_use);
        info.live_in.insert(block.id, HashSet::new());
        info.live_out.insert(block.id, HashSet::new());
    }

    let successors: HashMap<BlockId, Vec<BlockId>> = func
        .blocks
        .iter()
        .map(|b| {
            let succ = match &b.terminator {
                Terminator::Return(_) => vec![],
                Terminator::Jump(target) => vec![*target],
                Terminator::Branch(_, t, f) => vec![*t, *f],
            };
            (b.id, succ)
        })
        .collect();

    // 2. Iterate to convergence
    let mut changed = true;
    while changed {
        changed = false;
        for block in func.blocks.iter().rev() {
            let id = block.id;
            let mut out = HashSet::new();
            if let Some(succs) = successors.get(&id) {
                for s in succs {
                    if let Some(s_in) = info.live_in.get(s) {
                        for var in s_in {
                            out.insert(*var);
                        }
                    }
                }
            }

            let mut in_set = uses.get(&id).unwrap().clone();
            for var in &out {
                if !defs.get(&id).unwrap().contains(var) {
                    in_set.insert(*var);
                }
            }

            if info.live_in.get(&id).unwrap() != &in_set {
                info.live_in.insert(id, in_set);
                changed = true;
            }
            if info.live_out.get(&id).unwrap() != &out {
                info.live_out.insert(id, out);
                changed = true;
            }
        }
    }

    info
}
