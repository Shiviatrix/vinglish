use crate::diagnostics;
use crate::graph::OwnershipGraph;
use eng_diagnostics::Diagnostic;
use eng_hir::symbol::SsaValueId;
use eng_hir::symbol::SymbolTable;
use eng_mir::{Instruction, MirModule, Operand};
use std::collections::HashSet;

pub struct OwnershipValidator;

impl Default for OwnershipValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl OwnershipValidator {
    pub fn new() -> Self {
        Self
    }

    pub fn validate(
        &self,
        symbol_table: &SymbolTable,
        module: &MirModule<eng_hir::symbol::SsaValueId>,
        _graph: &OwnershipGraph,
    ) -> Result<(), Vec<Diagnostic>> {
        let mut errors = Vec::new();

        let is_move = |var_id: SsaValueId| -> bool {
            if let Some(eng_hir::symbol::SymbolKind::Variable(vs)) =
                symbol_table.get(eng_hir::symbol::SymbolId(var_id.0))
            {
                !vs.ty.is_copy()
            } else {
                true
            }
        };

        for func in &module.functions {
            let mut moved = HashSet::new();

            for block in &func.blocks {
                for instr in &block.instrs {
                    let mut check_op = |op: &Operand<SsaValueId>,
                                        is_val: bool,
                                        dest: SsaValueId| {
                        if let Operand::<SsaValueId>::Var(src) = op {
                            if moved.contains(src) {
                                errors.push(diagnostics::use_after_move(symbol_table, *src, dest));
                            } else if is_val && is_move(*src) {
                                moved.insert(*src);
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
                        Instruction::<SsaValueId>::Call(dest, _, args) => {
                            for arg in args {
                                check_op(arg, true, *dest);
                            }
                        }
                        Instruction::<SsaValueId>::Borrow(dest, op) => {
                            check_op(op, false, *dest);
                        }
                        Instruction::<SsaValueId>::BorrowMut(_dest, op) => {
                            if let Operand::<SsaValueId>::Var(src) = op {
                                if moved.contains(src) {
                                    errors.push(diagnostics::borrow_after_move(symbol_table, *src));
                                }
                            }
                        }
                        _ => {}
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
