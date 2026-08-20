use crate::diagnostics;
use crate::graph::OwnershipGraph;
use std::collections::{HashMap, HashSet};
use vinglish_diagnostics::Diagnostic;
use vinglish_hir::symbol::SsaValueId;
use vinglish_hir::symbol::SymbolTable;
use vinglish_mir::{BlockId, Instruction, MirModule, Operand, Terminator};

pub struct OwnershipValidator;

impl Default for OwnershipValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Default, PartialEq, Eq)]
struct BlockState {
    moved: HashMap<SsaValueId, vinglish_lexer::Span>,
    mutably_borrowed: HashMap<SsaValueId, vinglish_lexer::Span>,
    borrow_sources: HashMap<SsaValueId, SsaValueId>,
}

impl OwnershipValidator {
    pub fn new() -> Self {
        Self
    }

    pub fn validate(
        &self,
        symbol_table: &SymbolTable,
        module: &MirModule<vinglish_hir::symbol::SsaValueId>,
        _graph: &OwnershipGraph,
    ) -> Result<(), Vec<Diagnostic>> {
        let mut errors = Vec::new();

        let get_span = |id: SsaValueId| -> vinglish_lexer::Span {
            if let Some(vinglish_hir::symbol::SymbolKind::Variable(vs)) =
                symbol_table.get(vinglish_hir::symbol::SymbolId(id.0))
            {
                vs.span.unwrap_or_default()
            } else {
                vinglish_lexer::Span::default()
            }
        };

        let is_move = |var_id: SsaValueId| -> bool {
            if let Some(vinglish_hir::symbol::SymbolKind::Variable(vs)) =
                symbol_table.get(vinglish_hir::symbol::SymbolId(var_id.0))
            {
                !vs.ty.is_copy()
            } else {
                true
            }
        };

        for func in &module.functions {
            let mut block_states: HashMap<BlockId, BlockState> = HashMap::new();
            let mut worklist = Vec::new();
            
            if let Some(first_block) = func.blocks.first() {
                block_states.insert(first_block.id, BlockState::default());
                worklist.push(first_block.id);
            }

            let mut processed = HashSet::new();

            while let Some(block_id) = worklist.pop() {
                if processed.contains(&block_id) {
                    continue;
                }
                processed.insert(block_id);

                let block = func.blocks.iter().find(|b| b.id == block_id).unwrap();
                let mut state = block_states.get(&block_id).unwrap().clone();

                for (idx, instr) in block.instrs.iter().enumerate() {
                    let instr_span = block.spans.get(idx).copied().unwrap_or_default();
                    let mut check_op =
                        |op: &Operand<SsaValueId>, is_val: bool, dest: SsaValueId| {
                            if let Operand::<SsaValueId>::Var(src) = op {
                                if let Some(move_span) = state.moved.get(src) {
                                    errors.push(diagnostics::use_after_move(
                                        symbol_table,
                                        *src,
                                        dest,
                                        instr_span,
                                        *move_span,
                                    ));
                                } else if is_val && is_move(*src) {
                                    state.moved.insert(*src, instr_span);
                                    if let Some(underlying) = state.borrow_sources.get(src) {
                                        state.mutably_borrowed.remove(underlying);
                                    }
                                }
                            }
                        };

                    match instr {
                        Instruction::<SsaValueId>::Assign(dest, op)
                        | Instruction::<SsaValueId>::UnaryOp(dest, _, op) => {
                            check_op(op, true, *dest);
                        }
                        Instruction::<SsaValueId>::LoadField(dest, op, _) => {
                            check_op(op, false, *dest);
                        }
                        Instruction::<SsaValueId>::StoreField(obj, _, val) => {
                            check_op(val, true, *obj);
                        }
                        Instruction::<SsaValueId>::BinaryOp(dest, _, left, right) => {
                            check_op(left, true, *dest);
                            check_op(right, true, *dest);
                        }
                        Instruction::<SsaValueId>::Call(dest, _, args) | Instruction::<SsaValueId>::CallIntrinsic(dest, _, args) => {
                            for arg in args {
                                check_op(arg, true, *dest);
                            }
                        }
                        Instruction::<SsaValueId>::Borrow(dest, op) => {
                            check_op(op, false, *dest);
                        }
                        Instruction::<SsaValueId>::BorrowMut(
                            dest,
                            Operand::<SsaValueId>::Var(src),
                        ) => {
                            if let Some(_move_span) = state.moved.get(src) {
                                errors.push(diagnostics::borrow_after_move(
                                    symbol_table,
                                    *src,
                                    instr_span,
                                ));
                            } else if state.mutably_borrowed.contains_key(src) {
                                errors.push(diagnostics::double_mutable_borrow(
                                    symbol_table,
                                    *src,
                                    instr_span,
                                ));
                            } else {
                                state.mutably_borrowed.insert(*src, instr_span);
                                state.borrow_sources.insert(*dest, *src);
                            }
                        }
                        Instruction::<SsaValueId>::ListNew(dest, cap) => {
                            check_op(cap, true, *dest);
                        }
                        Instruction::<SsaValueId>::ListGet(dest, list, idx) => {
                            check_op(idx, true, *dest);
                            check_op(list, false, *dest);
                            if is_move(*dest) {
                                errors.push(diagnostics::move_from_collection(instr_span));
                            }
                        }
                        Instruction::<SsaValueId>::ListBorrowGet(dest, list, idx) => {
                            check_op(idx, true, *dest);
                            check_op(list, false, *dest);
                        }
                        Instruction::<SsaValueId>::ListBorrowMutGet(dest, list, idx) => {
                            check_op(idx, true, *dest);
                            if let Operand::<SsaValueId>::Var(src) = list {
                                if let Some(_move_span) = state.moved.get(src) {
                                    errors.push(diagnostics::borrow_after_move(
                                        symbol_table,
                                        *src,
                                        instr_span,
                                    ));
                                } else if state.mutably_borrowed.contains_key(src) {
                                    errors.push(diagnostics::double_mutable_borrow(
                                        symbol_table,
                                        *src,
                                        instr_span,
                                    ));
                                } else {
                                    state.mutably_borrowed.insert(*src, instr_span);
                                }
                            }
                        }
                        Instruction::<SsaValueId>::ListSet(_list, idx, val) => {
                            check_op(idx, true, SsaValueId(0)); 
                            check_op(val, true, SsaValueId(0));
                        }
                        Instruction::<SsaValueId>::StoreDeref(_ptr, val) => {
                            check_op(val, true, SsaValueId(0));
                        }
                        Instruction::<SsaValueId>::ListPush(_list, val) => {
                            check_op(val, true, SsaValueId(0));
                        }
                        Instruction::<SsaValueId>::ListPop(dest, list) => {
                            check_op(list, false, *dest);
                        }
                        Instruction::<SsaValueId>::ListLen(dest, list) => {
                            check_op(list, false, *dest);
                        }
                        Instruction::<SsaValueId>::Drop(var) => {
                            if let Some(src) = state.borrow_sources.get(var) {
                                state.mutably_borrowed.remove(src);
                            }
                        }
                        _ => {}
                    }
                }

                // Handle propagation
                let mut propagate = |target: BlockId, st: &BlockState| {
                    if let Some(existing) = block_states.get_mut(&target) {
                        let mut changed = false;
                        for (k, v) in &st.moved {
                            if !existing.moved.contains_key(k) {
                                existing.moved.insert(*k, *v);
                                changed = true;
                            }
                        }
                        // For mutably_borrowed, a merge is union (if borrowed on one path, it's borrowed)
                        for (k, v) in &st.mutably_borrowed {
                            if !existing.mutably_borrowed.contains_key(k) {
                                existing.mutably_borrowed.insert(*k, *v);
                                changed = true;
                            }
                        }
                        for (k, v) in &st.borrow_sources {
                            if !existing.borrow_sources.contains_key(k) {
                                existing.borrow_sources.insert(*k, *v);
                                changed = true;
                            }
                        }
                        if changed {
                            worklist.push(target);
                            processed.remove(&target);
                        }
                    } else {
                        block_states.insert(target, st.clone());
                        worklist.push(target);
                    }
                };

                match &block.terminator {
                    Terminator::Jump(target) => propagate(*target, &state),
                    Terminator::Branch(cond, t_target, f_target) => {
                        if let Operand::<SsaValueId>::Var(src) = cond {
                            if let Some(move_span) = state.moved.get(src) {
                                errors.push(diagnostics::use_after_move(
                                    symbol_table,
                                    *src,
                                    SsaValueId(0), // dummy
                                    vinglish_lexer::Span::dummy(),
                                    *move_span,
                                ));
                            }
                        }
                        propagate(*t_target, &state);
                        propagate(*f_target, &state);
                    }
                    _ => {}
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}
