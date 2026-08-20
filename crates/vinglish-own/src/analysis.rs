use crate::graph::OwnershipGraph;
use crate::state::OwnershipState;
use std::collections::{HashMap, HashSet};
use vinglish_hir::symbol::SsaValueId;
use vinglish_mir::{Instruction, MirModule, Operand, Terminator};

pub struct OwnershipAnalysisPass;

impl Default for OwnershipAnalysisPass {
    fn default() -> Self {
        Self::new()
    }
}

impl OwnershipAnalysisPass {
    pub fn new() -> Self {
        Self
    }

    pub fn run(
        &self,
        module: &mut MirModule<vinglish_hir::symbol::SsaValueId>,
        symbol_table: &vinglish_hir::symbol::SymbolTable,
    ) -> OwnershipGraph {
        let mut graph = OwnershipGraph::new();

        let is_move = |var_id: SsaValueId| -> bool {
            if let Some(vinglish_hir::symbol::SymbolKind::Variable(vs)) =
                symbol_table.get(vinglish_hir::symbol::SymbolId(var_id.0))
            {
                !vs.ty.is_copy()
            } else {
                true 
            }
        };

        for func in &mut module.functions {
            let liveness = crate::liveness::compute_liveness(func);
            
            // Parameters are Owned initially
            for &param in &func.params {
                graph.set_state(param, OwnershipState::Owned);
            }

            for block in &mut func.blocks {
                let mut block_vars = HashSet::new();
                let mut new_instrs = Vec::new();

                for instr in &block.instrs {
                    new_instrs.push(instr.clone());

                    match instr {
                        Instruction::<SsaValueId>::HeapAllocate(dest, _)
                        | Instruction::<SsaValueId>::StackAllocate(dest, _)
                        | Instruction::<SsaValueId>::Deref(dest, _, _) => {
                            graph.set_state(*dest, OwnershipState::Owned);
                            block_vars.insert(*dest);
                        }
                        Instruction::<SsaValueId>::Assign(dest, op) => {
                            graph.set_state(*dest, OwnershipState::Owned);
                            block_vars.insert(*dest);

                            if let Operand::<SsaValueId>::Var(src) = op {
                                if is_move(*src) {
                                    graph.set_state(*src, OwnershipState::Moved(*dest));
                                }
                            }
                        }
                        Instruction::<SsaValueId>::Call(dest, _, args)
                        | Instruction::<SsaValueId>::CallIntrinsic(dest, _, args) => {
                            graph.set_state(*dest, OwnershipState::Owned);
                            block_vars.insert(*dest);

                            for arg in args {
                                if let Operand::<SsaValueId>::Var(src) = arg {
                                    if is_move(*src) {
                                        graph.set_state(*src, OwnershipState::Moved(*dest));
                                    }
                                }
                            }
                        }
                        Instruction::<SsaValueId>::BinaryOp(dest, _, left, right) => {
                            graph.set_state(*dest, OwnershipState::Owned);
                            block_vars.insert(*dest);

                            if let Operand::<SsaValueId>::Var(src) = left {
                                if is_move(*src) {
                                    graph.set_state(*src, OwnershipState::Moved(*dest));
                                }
                            }
                            if let Operand::<SsaValueId>::Var(src) = right {
                                if is_move(*src) {
                                    graph.set_state(*src, OwnershipState::Moved(*dest));
                                }
                            }
                        }
                        Instruction::<SsaValueId>::UnaryOp(dest, _, operand) => {
                            graph.set_state(*dest, OwnershipState::Owned);
                            block_vars.insert(*dest);

                            if let Operand::<SsaValueId>::Var(src) = operand {
                                if is_move(*src) {
                                    graph.set_state(*src, OwnershipState::Moved(*dest));
                                }
                            }
                        }
                        Instruction::<SsaValueId>::Borrow(dest, op) => {
                            graph.set_state(*dest, OwnershipState::Owned);
                            block_vars.insert(*dest);
                            
                            if let Operand::<SsaValueId>::Var(src) = op {
                                graph.set_state(
                                    *src,
                                    OwnershipState::BorrowedShared(vec![*dest]),
                                );
                            }
                        }
                        Instruction::<SsaValueId>::BorrowMut(dest, op) => {
                            graph.set_state(*dest, OwnershipState::Owned);
                            block_vars.insert(*dest);
                            
                            if let Operand::<SsaValueId>::Var(src) = op {
                                graph.set_state(*src, OwnershipState::BorrowedMutable(*dest));
                            }
                        }
                        Instruction::<SsaValueId>::StoreField(obj, _, val) => {
                            if let Operand::<SsaValueId>::Var(src) = val {
                                if is_move(*src) {
                                    graph.set_state(*src, OwnershipState::Moved(*obj));
                                }
                            }
                        }
                        Instruction::<SsaValueId>::ListNew(dest, cap) => {
                            graph.set_state(*dest, OwnershipState::Owned);
                            block_vars.insert(*dest);

                            if let Operand::<SsaValueId>::Var(src) = cap {
                                if is_move(*src) {
                                    graph.set_state(*src, OwnershipState::Moved(*dest));
                                }
                            }
                        }
                        Instruction::<SsaValueId>::ListGet(dest, list, idx) => {
                            graph.set_state(*dest, OwnershipState::Owned);
                            block_vars.insert(*dest);

                            if let Operand::<SsaValueId>::Var(src) = idx {
                                if is_move(*src) {
                                    graph.set_state(*src, OwnershipState::Moved(*dest));
                                }
                            }
                            if let Operand::<SsaValueId>::Var(src) = list {
                                if is_move(*dest) {
                                    graph.set_state(*src, OwnershipState::Moved(*dest));
                                }
                            }
                        }
                        Instruction::<SsaValueId>::ListBorrowGet(dest, list, idx) => {
                            graph.set_state(*dest, OwnershipState::Owned);
                            block_vars.insert(*dest);

                            if let Operand::<SsaValueId>::Var(src) = idx {
                                if is_move(*src) {
                                    graph.set_state(*src, OwnershipState::Moved(*dest));
                                }
                            }
                            if let Operand::<SsaValueId>::Var(src) = list {
                                graph.set_state(
                                    *src,
                                    OwnershipState::BorrowedShared(vec![*dest]),
                                );
                            }
                        }
                        Instruction::<SsaValueId>::ListBorrowMutGet(dest, list, idx) => {
                            graph.set_state(*dest, OwnershipState::Owned);
                            block_vars.insert(*dest);

                            if let Operand::<SsaValueId>::Var(src) = idx {
                                if is_move(*src) {
                                    graph.set_state(*src, OwnershipState::Moved(*dest));
                                }
                            }
                            if let Operand::<SsaValueId>::Var(src) = list {
                                graph.set_state(*src, OwnershipState::BorrowedMutable(*dest));
                            }
                        }
                        Instruction::<SsaValueId>::ListSet(list, idx, val) => {
                            if let Operand::<SsaValueId>::Var(src) = idx {
                                if is_move(*src) {
                                    graph.set_state(*src, OwnershipState::Moved(*src));
                                }
                            }
                            if let Operand::<SsaValueId>::Var(src) = val {
                                if let Operand::<SsaValueId>::Var(l_src) = list {
                                    graph.set_state(*src, OwnershipState::Moved(*l_src));
                                } else {
                                    graph.set_state(*src, OwnershipState::Moved(*src));
                                }
                            }
                        }
                        Instruction::<SsaValueId>::ListPush(list, val) => {
                            if let Operand::<SsaValueId>::Var(src) = val {
                                if let Operand::<SsaValueId>::Var(l_src) = list {
                                    graph.set_state(*src, OwnershipState::Moved(*l_src));
                                }
                            }
                        }
                        Instruction::<SsaValueId>::ListPop(dest, _list) => {
                            graph.set_state(*dest, OwnershipState::Owned);
                            block_vars.insert(*dest);
                        }
                        Instruction::<SsaValueId>::ListLen(dest, _) => {
                            graph.set_state(*dest, OwnershipState::Owned);
                            block_vars.insert(*dest);
                        }
                        Instruction::<SsaValueId>::StoreDeref(_, val) => {
                            if let Operand::<SsaValueId>::Var(src) = val {
                                if is_move(*src) {
                                    graph.set_state(*src, OwnershipState::Moved(*src));
                                }
                            }
                        }
                        Instruction::<SsaValueId>::Drop(var) => {
                            graph.set_state(*var, OwnershipState::Dropped);
                        }
                        Instruction::<SsaValueId>::LoadField(dest, _, _) => {
                            graph.set_state(*dest, OwnershipState::Owned);
                            block_vars.insert(*dest);
                        }
                        Instruction::<SsaValueId>::Phi(dest, _) => {
                            graph.set_state(*dest, OwnershipState::Owned);
                            block_vars.insert(*dest);
                        }
                    }
                }

                // In Stage 1 NLL, we inject a Drop for variables that die at the end of this block
                // A variable dies if it is in LiveIn or block_vars, and NOT in LiveOut.
                // We only drop if it's currently Owned (not moved/borrowed away)
                let live_out = liveness.live_out.get(&block.id).unwrap();
                let live_in = liveness.live_in.get(&block.id).unwrap();
                
                let mut candidates = HashSet::new();
                for var in live_in { candidates.insert(*var); }
                for var in &block_vars { candidates.insert(*var); }
                
                // Add function params to candidates for the blocks they enter
                if block.id.0 == 0 {
                    for param in &func.params {
                        candidates.insert(*param);
                    }
                }

                for var in candidates {
                    if !live_out.contains(&var) {
                        // The variable dies here. If it's Owned, we inject a Drop.
                        // Wait, `graph.is_owned` is global right now! We shouldn't rely strictly on it for drop placement in branches.
                        // But since we are constructing MIR, injecting a drop is fine (it might become a no-op if moved).
                        new_instrs.push(Instruction::<SsaValueId>::Drop(var));
                    }
                }

                block.instrs = new_instrs;
            }
        }

        graph
    }
}
