use crate::{OptimizationPass, PassStats};
use eng_mir::{Instruction, MirModule, Operand, Terminator};
use std::collections::HashSet;
use std::fmt::Display;
use std::hash::Hash;

pub struct DeadCodeEliminationPass;

impl<V: Clone + Copy + Display + Eq + Hash> OptimizationPass<V> for DeadCodeEliminationPass {
    fn name(&self) -> &'static str {
        "Dead Code Elimination"
    }

    fn run(&mut self, module: &mut MirModule<V>) -> PassStats {
        let mut stats = PassStats::default();

        for func in &mut module.functions {
            // Step 1: Reachability analysis
            let mut reachable = HashSet::new();
            if !func.blocks.is_empty() {
                let mut worklist = vec![func.blocks[0].id];
                reachable.insert(func.blocks[0].id);

                while let Some(block_id) = worklist.pop() {
                    let block = func.blocks.iter().find(|b| b.id == block_id).unwrap();
                    let successors = match &block.terminator {
                        Terminator::<V>::Jump(target) => vec![target],
                        Terminator::<V>::Branch(_, true_target, false_target) => {
                            vec![true_target, false_target]
                        }
                        Terminator::<V>::Return(_) => vec![],
                    };

                    for succ in successors {
                        if reachable.insert(*succ) {
                            worklist.push(*succ);
                        }
                    }
                }
            }

            // Remove unreachable blocks
            let _initial_blocks = func.blocks.len();
            func.blocks.retain(|b| reachable.contains(&b.id));
            // Instead of merged_blocks, we can just say removed blocks. But we only have merged_blocks in stats.

            // Loop until no more instructions can be eliminated
            loop {
                let mut used_vars: HashSet<V> = HashSet::new();

                // Find all uses
                for block in &func.blocks {
                    for instr in &block.instrs {
                        match instr {
                            Instruction::<V>::Assign(_, Operand::<V>::Var(id))
                            | Instruction::<V>::LoadField(_, Operand::<V>::Var(id), _)
                            | Instruction::<V>::UnaryOp(_, _, Operand::<V>::Var(id)) => {
                                used_vars.insert(*id);
                            }
                            Instruction::<V>::StoreField(obj, _, val) => {
                                used_vars.insert(*obj);
                                if let Operand::<V>::Var(id) = val {
                                    used_vars.insert(*id);
                                }
                            }
                            Instruction::<V>::Call(_, _func_id, args) => {
                                for arg in args {
                                    if let Operand::<V>::Var(id) = arg {
                                        used_vars.insert(*id);
                                    }
                                }
                            }
                            Instruction::<V>::BinaryOp(_, _, left, right) => {
                                if let Operand::<V>::Var(id) = left {
                                    used_vars.insert(*id);
                                }
                                if let Operand::<V>::Var(id) = right {
                                    used_vars.insert(*id);
                                }
                            }
                            Instruction::<V>::Borrow(_, Operand::<V>::Var(id))
                            | Instruction::<V>::BorrowMut(_, Operand::<V>::Var(id))
                            | Instruction::<V>::Deref(_, Operand::<V>::Var(id), _) => {
                                used_vars.insert(*id);
                            }
                            Instruction::<V>::Drop(id) => {
                                used_vars.insert(*id);
                            }
                            Instruction::<V>::Phi(_, args) => {
                                for (op, _) in args {
                                    if let Operand::<V>::Var(id) = op {
                                        used_vars.insert(*id);
                                    }
                                }
                            }
                            _ => {}
                        }
                    }

                    match &block.terminator {
                        Terminator::<V>::Return(Some(Operand::<V>::Var(id))) => {
                            used_vars.insert(*id);
                        }
                        Terminator::<V>::Branch(Operand::<V>::Var(id), _, _) => {
                            used_vars.insert(*id);
                        }
                        _ => {}
                    }
                }

                let mut changed = false;

                // Remove pure instructions whose destination is unused
                for block in &mut func.blocks {
                    let initial_len = block.instrs.len();
                    block.instrs.retain(|instr| {
                        match instr {
                            Instruction::<V>::Assign(dest, _)
                            | Instruction::<V>::LoadField(dest, _, _)
                            | Instruction::<V>::Borrow(dest, _)
                            | Instruction::<V>::BorrowMut(dest, _)
                            | Instruction::<V>::Deref(dest, _, _)
                            | Instruction::<V>::HeapAllocate(dest, _)
                            | Instruction::<V>::StackAllocate(dest, _)
                            | Instruction::<V>::BinaryOp(dest, _, _, _)
                            | Instruction::<V>::UnaryOp(dest, _, _)
                            | Instruction::<V>::Phi(dest, _) => used_vars.contains(dest),
                            Instruction::<V>::StoreField(_, _, _)
                            | Instruction::<V>::Drop(_)
                            | Instruction::<V>::Call(_, _, _) => true, // Side effects!
                        }
                    });

                    let removed = initial_len - block.instrs.len();
                    if removed > 0 {
                        stats.removed_instructions += removed;
                        changed = true;
                    }
                }

                if !changed {
                    break;
                }
            }
        }

        stats
    }
}
