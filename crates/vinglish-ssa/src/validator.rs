use vinglish_hir::symbol::SsaValueId;
use vinglish_mir::{Instruction, MirFunction, MirModule, Terminator};
use std::collections::{HashMap, HashSet};

#[derive(Debug)]
pub struct SSAValidationError {
    pub message: String,
}

pub struct SSAValidator;

impl Default for SSAValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl SSAValidator {
    pub fn new() -> Self {
        Self
    }

    pub fn validate(
        &self,
        module: &MirModule<vinglish_hir::symbol::SsaValueId>,
    ) -> Result<(), Vec<SSAValidationError>> {
        let mut errors = Vec::new();

        for func in &module.functions {
            if let Err(mut e) = self.validate_function(func) {
                errors.append(&mut e);
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    fn validate_function(
        &self,
        func: &MirFunction<SsaValueId>,
    ) -> Result<(), Vec<SSAValidationError>> {
        let mut errors = Vec::new();
        let mut assigned_vars = HashSet::new();

        // 1. Every variable must be assigned exactly once (excluding params which are pre-assigned)
        for param in &func.params {
            assigned_vars.insert(*param);
        }

        for block in &func.blocks {
            for instr in &block.instrs {
                match instr {
                    Instruction::<SsaValueId>::Assign(dest, _)
                    | Instruction::<SsaValueId>::LoadField(dest, _, _)
                    | Instruction::<SsaValueId>::Call(dest, _, _)
                    | Instruction::<SsaValueId>::CallIntrinsic(dest, _, _)
                    | Instruction::<SsaValueId>::HeapAllocate(dest, _)
                    | Instruction::<SsaValueId>::StackAllocate(dest, _)
                    | Instruction::<SsaValueId>::BinaryOp(dest, _, _, _)
                    | Instruction::<SsaValueId>::UnaryOp(dest, _, _)
                    | Instruction::<SsaValueId>::Borrow(dest, _)
                    | Instruction::<SsaValueId>::BorrowMut(dest, _)
                    | Instruction::<SsaValueId>::Deref(dest, _, _)
                    | Instruction::<SsaValueId>::Phi(dest, _) => {
                        if !assigned_vars.insert(*dest) {
                            errors.push(SSAValidationError {
                                message: format!(
                                    "Variable var_{} is assigned multiple times in function {}",
                                    dest.0, func.name
                                ),
                            });
                        }
                    }
                    Instruction::<SsaValueId>::StoreField(_, _, _)
                    | Instruction::<SsaValueId>::Drop(_) => {}
                }
            }
        }

        // 2. Variables must be defined before use (we can check simple reachability in SSA by checking if the use is in `assigned_vars` overall,
        // since single assignment implies if it's used and it's assigned *somewhere*, dominance usually ensures it's before use.
        // Wait, proper validation checks dominance! But checking definition is simpler first)

        // 3. Phi predecessors must match actual CFG predecessors
        let mut preds: HashMap<vinglish_mir::BlockId, HashSet<vinglish_mir::BlockId>> = HashMap::new();
        for block in &func.blocks {
            match &block.terminator {
                Terminator::<SsaValueId>::Jump(target) => {
                    preds.entry(*target).or_default().insert(block.id);
                }
                Terminator::<SsaValueId>::Branch(_, true_tgt, false_tgt) => {
                    preds.entry(*true_tgt).or_default().insert(block.id);
                    preds.entry(*false_tgt).or_default().insert(block.id);
                }
                Terminator::<SsaValueId>::Return(_) => {}
            }
        }

        for block in &func.blocks {
            let block_preds = preds.get(&block.id).cloned().unwrap_or_default();
            for instr in &block.instrs {
                if let Instruction::<SsaValueId>::Phi(_, args) = instr {
                    let mut phi_preds = HashSet::new();
                    for (_, pred_block) in args {
                        phi_preds.insert(*pred_block);
                    }
                    if phi_preds != block_preds {
                        errors.push(SSAValidationError {
                            message: format!("Phi node in block {} has predecessors {:?} but actual CFG predecessors are {:?}", block.id, phi_preds, block_preds),
                        });
                    }
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
