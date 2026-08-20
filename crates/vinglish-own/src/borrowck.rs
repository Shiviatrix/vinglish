use std::collections::{HashMap, HashSet};

use vinglish_hir::symbol::{SymbolTable, SsaValueId};
use vinglish_mir::{BasicBlock, BlockId, Instruction, MirFunction, Operand, Terminator};

use crate::liveness::LivenessInfo;

#[derive(Debug, Clone)]
pub struct OwnershipError {
    pub message: String,
    pub span: vinglish_lexer::Span,
    pub note: Option<String>,
}

impl OwnershipError {
    pub fn new(msg: impl Into<String>, span: vinglish_lexer::Span) -> Self {
        Self {
            message: msg.into(),
            span,
            note: None,
        }
    }
    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.note = Some(note.into());
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VarState {
    Uninit,
    Init,
    Moved,
}

#[derive(Debug, Clone)]
pub struct BlockState {
    pub vars: HashMap<SsaValueId, VarState>,
}

pub fn check_mir_function(
    func: &MirFunction<SsaValueId>,
    liveness: &LivenessInfo,
    _symbol_table: &SymbolTable, // Will be used for Copy semantics later
) -> Vec<OwnershipError> {
    let mut errors = Vec::new();
    let mut block_states: HashMap<BlockId, BlockState> = HashMap::new();

    // Initialize block 0
    let mut initial_vars = HashMap::new();
    for &param in &func.params {
        initial_vars.insert(param, VarState::Init);
    }
    for &local in &func.locals {
        if !initial_vars.contains_key(&local) {
            initial_vars.insert(local, VarState::Uninit);
        }
    }

    if let Some(first_block) = func.blocks.first() {
        block_states.insert(
            first_block.id,
            BlockState {
                vars: initial_vars,
            },
        );
    }

    let mut worklist: Vec<BlockId> = vec![];
    if let Some(first_block) = func.blocks.first() {
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
        
        let _live_out = liveness.live_out.get(&block_id).unwrap();

        for (i, instr) in block.instrs.iter().enumerate() {
            let span = block.spans.get(i).copied().unwrap_or(vinglish_lexer::Span::dummy());

            // Helper to check use
            let mut check_use = |op: &Operand<SsaValueId>, is_move: bool| {
                if let Operand::Var(var) = op {
                    let var_state = state.vars.get(var).copied().unwrap_or(VarState::Uninit);
                    if var_state == VarState::Uninit {
                        errors.push(OwnershipError::new(
                            format!("use of uninitialized variable `{}`", var),
                            span,
                        ));
                    } else if var_state == VarState::Moved {
                        errors.push(OwnershipError::new(
                            format!("use of moved value `{}`", var),
                            span,
                        ));
                    } else if is_move {
                        state.vars.insert(*var, VarState::Moved);
                    }
                }
            };

            match instr {
                Instruction::Assign(dest, op) => {
                    check_use(op, true); // Move by default for now
                    state.vars.insert(*dest, VarState::Init);
                }
                Instruction::LoadField(dest, op, _) => {
                    check_use(op, false); // Borrow/Copy semantics for field load
                    state.vars.insert(*dest, VarState::Init);
                }
                Instruction::StoreField(obj, _, op) => {
                    check_use(&Operand::Var(*obj), false);
                    check_use(op, true);
                }
                Instruction::Call(dest, _, args) | Instruction::CallIntrinsic(dest, _, args) => {
                    for arg in args {
                        check_use(arg, true);
                    }
                    state.vars.insert(*dest, VarState::Init);
                }
                Instruction::HeapAllocate(dest, _) | Instruction::StackAllocate(dest, _) => {
                    state.vars.insert(*dest, VarState::Init);
                }
                Instruction::BinaryOp(dest, _, left, right) => {
                    check_use(left, true);
                    check_use(right, true);
                    state.vars.insert(*dest, VarState::Init);
                }
                Instruction::UnaryOp(dest, _, op) => {
                    check_use(op, true);
                    state.vars.insert(*dest, VarState::Init);
                }
                Instruction::Borrow(dest, op) => {
                    check_use(op, false);
                    state.vars.insert(*dest, VarState::Init);
                }
                Instruction::BorrowMut(dest, op) => {
                    check_use(op, false);
                    state.vars.insert(*dest, VarState::Init);
                }
                Instruction::Deref(dest, op, _) => {
                    check_use(op, false);
                    state.vars.insert(*dest, VarState::Init);
                }
                Instruction::StoreDeref(ptr, val) => {
                    check_use(ptr, false);
                    check_use(val, true);
                }
                Instruction::Drop(var) => {
                    // Check if it's initialized before dropping, though MIR generator might emit drops unconditionally
                    // We can just mark it moved/dropped
                    state.vars.insert(*var, VarState::Moved);
                }
                Instruction::Phi(dest, args) => {
                    for (op, _) in args {
                        check_use(op, true);
                    }
                    state.vars.insert(*dest, VarState::Init);
                }
                Instruction::ListNew(dest, cap) => {
                    check_use(cap, true);
                    state.vars.insert(*dest, VarState::Init);
                }
                Instruction::ListGet(dest, list, idx)
                | Instruction::ListBorrowGet(dest, list, idx)
                | Instruction::ListBorrowMutGet(dest, list, idx) => {
                    check_use(list, false);
                    check_use(idx, true);
                    state.vars.insert(*dest, VarState::Init);
                }
                Instruction::ListSet(list, idx, val) => {
                    check_use(list, false);
                    check_use(idx, true);
                    check_use(val, true);
                }
                Instruction::ListLen(dest, list) => {
                    check_use(list, false);
                    state.vars.insert(*dest, VarState::Init);
                }
                Instruction::ListPush(list, val) => {
                    check_use(list, false);
                    check_use(val, true);
                }
                Instruction::ListPop(dest, list) => {
                    check_use(list, false);
                    state.vars.insert(*dest, VarState::Init);
                }
            }
        }

        // Handle Terminator
        let span = block.spans.last().copied().unwrap_or(vinglish_lexer::Span::dummy());
        match &block.terminator {
            Terminator::Return(Some(op)) => {
                if let Operand::Var(var) = op {
                    let var_state = state.vars.get(var).copied().unwrap_or(VarState::Uninit);
                    if var_state == VarState::Uninit {
                        errors.push(OwnershipError::new(
                            format!("use of uninitialized variable `{}`", var),
                            span,
                        ));
                    } else if var_state == VarState::Moved {
                        errors.push(OwnershipError::new(
                            format!("use of moved value `{}`", var),
                            span,
                        ));
                    }
                }
            }
            Terminator::Return(None) => {}
            Terminator::Jump(target) => {
                propagate_state(*target, &state, &mut block_states, &mut worklist);
            }
            Terminator::Branch(cond, t_target, f_target) => {
                if let Operand::Var(var) = cond {
                    let var_state = state.vars.get(var).copied().unwrap_or(VarState::Uninit);
                    if var_state == VarState::Uninit {
                        errors.push(OwnershipError::new(
                            format!("use of uninitialized variable `{}`", var),
                            span,
                        ));
                    } else if var_state == VarState::Moved {
                        errors.push(OwnershipError::new(
                            format!("use of moved value `{}`", var),
                            span,
                        ));
                    }
                }
                propagate_state(*t_target, &state, &mut block_states, &mut worklist);
                propagate_state(*f_target, &state, &mut block_states, &mut worklist);
            }
        }
    }

    errors
}

fn propagate_state(
    target: BlockId,
    state: &BlockState,
    block_states: &mut HashMap<BlockId, BlockState>,
    worklist: &mut Vec<BlockId>,
) {
    if let Some(existing) = block_states.get_mut(&target) {
        let mut changed = false;
        for (var, var_state) in &state.vars {
            if let Some(existing_state) = existing.vars.get_mut(var) {
                if *existing_state == VarState::Init && *var_state == VarState::Moved {
                    *existing_state = VarState::Moved;
                    changed = true;
                } else if *existing_state == VarState::Init && *var_state == VarState::Uninit {
                    *existing_state = VarState::Uninit;
                    changed = true;
                }
            }
        }
        if changed {
            worklist.push(target);
        }
    } else {
        block_states.insert(target, state.clone());
        worklist.push(target);
    }
}
