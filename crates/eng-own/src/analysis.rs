use crate::graph::OwnershipGraph;
use crate::state::OwnershipState;
use eng_hir::symbol::SsaValueId;
use eng_mir::{Instruction, MirModule, Operand};
use std::collections::HashSet;

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
        module: &mut MirModule<eng_hir::symbol::SsaValueId>,
        symbol_table: &eng_hir::symbol::SymbolTable,
    ) -> OwnershipGraph {
        let mut graph = OwnershipGraph::new();

        let is_move = |var_id: SsaValueId| -> bool {
            if let Some(eng_hir::symbol::SymbolKind::Variable(vs)) =
                symbol_table.get(eng_hir::symbol::SymbolId(var_id.0))
            {
                !vs.ty.is_copy()
            } else {
                true // default to move if unknown
            }
        };

        // Very basic block-local analysis for now
        for func in &mut module.functions {
            // function parameters are Owned
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
                        Instruction::<SsaValueId>::Call(dest, _, args) => {
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
                            graph.set_state(*dest, OwnershipState::Owned); // the borrow itself is owned
                            block_vars.insert(*dest);

                            if let Operand::<SsaValueId>::Var(src) = op {
                                let mut current = graph.get_state(*src);
                                match current {
                                    OwnershipState::Owned => {
                                        graph.set_state(
                                            *src,
                                            OwnershipState::BorrowedShared(vec![*dest]),
                                        );
                                    }
                                    OwnershipState::BorrowedShared(ref mut by) => {
                                        by.push(*dest);
                                        graph.set_state(
                                            *src,
                                            OwnershipState::BorrowedShared(by.clone()),
                                        );
                                    }
                                    _ => {
                                        // Validator will catch invalid transitions
                                        graph.set_state(
                                            *src,
                                            OwnershipState::BorrowedShared(vec![*dest]),
                                        );
                                    }
                                }
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
                                graph.set_state(*src, OwnershipState::Moved(*obj));
                            }
                        }
                        Instruction::<SsaValueId>::LoadField(d, _, _) => {
                            graph.set_state(*d, OwnershipState::Owned);
                            block_vars.insert(*d);
                        }
                        Instruction::<SsaValueId>::Drop(_) => {
                            // drop already handled
                        }
                        Instruction::<SsaValueId>::Phi(dest, args) => {
                            graph.set_state(*dest, OwnershipState::Owned);
                            block_vars.insert(*dest);
                            for (op, _) in args {
                                if let Operand::<SsaValueId>::Var(src) = op {
                                    graph.set_state(*src, OwnershipState::Moved(*dest));
                                }
                            }
                        }
                    }
                }

                // Implicit Drop Injection at end of block
                for var in &block_vars {
                    if graph.is_owned(*var) {
                        new_instrs.push(Instruction::<SsaValueId>::Drop(*var));
                        graph.set_state(*var, OwnershipState::Dropped);
                    }
                }

                block.instrs = new_instrs;
            }
        }

        graph
    }
}
