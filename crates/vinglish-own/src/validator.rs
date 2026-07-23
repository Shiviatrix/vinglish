use crate::diagnostics;
use crate::graph::OwnershipGraph;
use vinglish_diagnostics::Diagnostic;
use vinglish_hir::symbol::SsaValueId;
use vinglish_hir::symbol::SymbolTable;
use vinglish_mir::{Instruction, MirModule, Operand};
use std::collections::HashMap;

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
            let mut moved = HashMap::new();
            let mut mutably_borrowed = HashMap::new();

            for block in &func.blocks {
                for instr in &block.instrs {
                    let mut check_op = |op: &Operand<SsaValueId>,
                                        is_val: bool,
                                        dest: SsaValueId| {
                        if let Operand::<SsaValueId>::Var(src) = op {
                            if let Some(move_span) = moved.get(src) {
                                let use_span = get_span(dest);
                                errors.push(diagnostics::use_after_move(symbol_table, *src, dest, use_span, *move_span));
                            } else if is_val && is_move(*src) {
                                moved.insert(*src, get_span(dest));
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
                        Instruction::<SsaValueId>::BorrowMut(dest, op) => {
                            if let Operand::<SsaValueId>::Var(src) = op {
                                if let Some(_move_span) = moved.get(src) {
                                    errors.push(diagnostics::borrow_after_move(symbol_table, *src, get_span(*dest)));
                                } else if mutably_borrowed.contains_key(src) {
                                    errors.push(diagnostics::double_mutable_borrow(symbol_table, *src, get_span(*dest)));
                                } else {
                                    mutably_borrowed.insert(*src, get_span(*dest));
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
