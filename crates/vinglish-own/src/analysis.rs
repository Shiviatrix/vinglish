use crate::graph::OwnershipGraph;
use crate::state::OwnershipState;
use std::collections::HashSet;
use vinglish_hir::symbol::SsaValueId;
use vinglish_mir::{Instruction, MirModule, Operand};

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

                            if let Operand::<SsaValueId>::Var(src) = op
                                && is_move(*src)
                            {
                                graph.set_state(*src, OwnershipState::Moved(*dest));
                            }
                        }
                        Instruction::<SsaValueId>::Call(dest, _, args)
                        | Instruction::<SsaValueId>::CallIntrinsic(dest, _, args) => {
                            graph.set_state(*dest, OwnershipState::Owned);
                            block_vars.insert(*dest);

                            for arg in args {
                                if let Operand::<SsaValueId>::Var(src) = arg
                                    && is_move(*src)
                                {
                                    graph.set_state(*src, OwnershipState::Moved(*dest));
                                }
                            }
                        }
                        Instruction::<SsaValueId>::BinaryOp(dest, _, left, right) => {
                            graph.set_state(*dest, OwnershipState::Owned);
                            block_vars.insert(*dest);

                            if let Operand::<SsaValueId>::Var(src) = left
                                && is_move(*src)
                            {
                                graph.set_state(*src, OwnershipState::Moved(*dest));
                            }
                            if let Operand::<SsaValueId>::Var(src) = right
                                && is_move(*src)
                            {
                                graph.set_state(*src, OwnershipState::Moved(*dest));
                            }
                        }
                        Instruction::<SsaValueId>::UnaryOp(dest, _, operand) => {
                            graph.set_state(*dest, OwnershipState::Owned);
                            block_vars.insert(*dest);

                            if let Operand::<SsaValueId>::Var(src) = operand
                                && is_move(*src)
                            {
                                graph.set_state(*src, OwnershipState::Moved(*dest));
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
                        Instruction::<SsaValueId>::ListNew(dest, _)
                        | Instruction::<SsaValueId>::ListLen(dest, _) => {
                            graph.set_state(*dest, OwnershipState::Owned);
                            block_vars.insert(*dest);
                        }
                        Instruction::<SsaValueId>::ListGet(dest, _list, _idx)
                        | Instruction::<SsaValueId>::ListBorrowGet(dest, _list, _idx)
                        | Instruction::<SsaValueId>::ListBorrowMutGet(dest, _list, _idx) => {
                            graph.set_state(*dest, OwnershipState::Owned);
                            block_vars.insert(*dest);
                        }
                        Instruction::<SsaValueId>::ListSet(_list, _idx, val) => {
                            if let Operand::<SsaValueId>::Var(src) = val {
                                // List assumes ownership of the set value
                                // But since we don't have list element tracking, we mark it moved to some dummy state,
                                // or simply mark as moved without destination.
                                graph.set_state(*src, OwnershipState::Moved(*src));
                            }
                        }
                        Instruction::<SsaValueId>::StoreDeref(_, val) => {
                            if let Operand::<SsaValueId>::Var(src) = val {
                                // List assumes ownership of the set value
                                // But since we don't have list element tracking, we mark it moved to some dummy state,
                                // or simply mark as moved without destination.
                                graph.set_state(*src, OwnershipState::Moved(*src));
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
