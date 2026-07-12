use crate::{MirFunction, MirModule, Terminator};
use eng_hir::symbol::SymbolTable;
use std::collections::HashSet;

#[derive(Debug)]
pub struct MirValidationError {
    pub message: String,
}

pub struct MirValidatorPass;

impl Default for MirValidatorPass {
    fn default() -> Self {
        Self::new()
    }
}

impl MirValidatorPass {
    pub fn new() -> Self {
        Self
    }

    pub fn validate<V: Clone + Copy + std::fmt::Display>(
        &self,
        symbol_table: &SymbolTable,
        module: &MirModule<V>,
    ) -> Result<(), Vec<MirValidationError>> {
        let mut errors = Vec::new();

        for func in &module.functions {
            if let Err(mut e) = self.validate_function(symbol_table, func) {
                errors.append(&mut e);
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    fn validate_function<V: Clone + Copy + std::fmt::Display>(
        &self,
        symbol_table: &SymbolTable,
        func: &MirFunction<V>,
    ) -> Result<(), Vec<MirValidationError>> {
        let mut errors = Vec::new();

        if symbol_table.get_func(func.id).is_none() {
            errors.push(MirValidationError {
                message: format!("Function {} has invalid ID {:?}", func.name, func.id),
            });
        }

        let mut block_ids = HashSet::new();
        for block in &func.blocks {
            if !block_ids.insert(block.id) {
                errors.push(MirValidationError {
                    message: format!("Duplicate block ID {} in function {}", block.id, func.name),
                });
            }
        }

        for block in &func.blocks {
            let mut seen_non_phi = false;
            for instr in &block.instrs {
                if let crate::Instruction::Phi(_, _) = instr {
                    if seen_non_phi {
                        errors.push(MirValidationError {
                            message: format!("Phi instruction found after non-phi instruction in block {} of function {}", block.id, func.name),
                        });
                    }
                } else {
                    seen_non_phi = true;
                }
            }
        }

        for block in &func.blocks {
            match &block.terminator {
                Terminator::Jump(target) => {
                    if !block_ids.contains(target) {
                        errors.push(MirValidationError {
                            message: format!(
                                "Dangling jump to {} in function {}",
                                target, func.name
                            ),
                        });
                    }
                }
                Terminator::Branch(_, true_target, false_target) => {
                    if !block_ids.contains(true_target) {
                        errors.push(MirValidationError {
                            message: format!(
                                "Dangling true branch to {} in function {}",
                                true_target, func.name
                            ),
                        });
                    }
                    if !block_ids.contains(false_target) {
                        errors.push(MirValidationError {
                            message: format!(
                                "Dangling false branch to {} in function {}",
                                false_target, func.name
                            ),
                        });
                    }
                }
                Terminator::Return(_) => {}
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}
