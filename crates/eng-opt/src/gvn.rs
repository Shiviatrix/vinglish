use crate::{OptimizationPass, PassStats};
use eng_mir::{MirModule, Instruction, Operand, Terminator};
use eng_hir::symbol::SsaValueId;
use std::collections::HashMap;

pub struct GlobalValueNumberingPass;

impl OptimizationPass<SsaValueId> for GlobalValueNumberingPass {
    fn name(&self) -> &'static str {
        "Global Value Numbering"
    }

    fn run(&mut self, module: &mut MirModule<SsaValueId>) -> PassStats {
        let mut stats = PassStats::default();

        for func in &mut module.functions {
            #[derive(PartialEq, Eq, Hash)]
            enum ValueExpr {
                Operand(Operand<SsaValueId>),
                BinaryOp(eng_parser::ast::BinOp, Operand<SsaValueId>, Operand<SsaValueId>),
                UnaryOp(eng_parser::ast::UnOp, Operand<SsaValueId>),
            }

            let mut value_table: HashMap<ValueExpr, SsaValueId> = HashMap::new();
            let mut replacements: HashMap<SsaValueId, SsaValueId> = HashMap::new();

            for block in &mut func.blocks {
                let mut new_instrs = Vec::new();

                for mut instr in std::mem::take(&mut block.instrs) {
                    let replace_operand = |op: &mut Operand<SsaValueId>| {
                        if let Operand::Var(v) = op {
                            if let Some(&new_v) = replacements.get(v) {
                                *v = new_v;
                            }
                        }
                    };

                    match &mut instr {
                        Instruction::Assign(_, op) => replace_operand(op),
                        Instruction::BinaryOp(_, _, left, right) => {
                            replace_operand(left);
                            replace_operand(right);
                        }
                        Instruction::Call(_, _, args) => {
                            for arg in args {
                                replace_operand(arg);
                            }
                        }
                        Instruction::UnaryOp(_, _, val) |
                        Instruction::Deref(_, val, _) => replace_operand(val),
                        Instruction::Borrow(_, _) |
                        Instruction::BorrowMut(_, _) => {}
                        Instruction::HeapAllocate(_, _) |
                        Instruction::StackAllocate(_, _) => {}
                        Instruction::LoadField(_, obj, _) => replace_operand(obj),
                        Instruction::StoreField(obj, _, val) => {
                            if let Some(&new_v) = replacements.get(obj) {
                                *obj = new_v;
                            }
                            replace_operand(val);
                        }
                        Instruction::Drop(v) => {
                            if let Some(&new_v) = replacements.get(v) {
                                *v = new_v;
                            }
                        }
                        Instruction::Phi(_, args) => {
                            for (op, _) in args {
                                replace_operand(op);
                            }
                        }
                    }

                    let (dest, expr) = match &instr {
                        Instruction::Assign(dest, op) => (Some(*dest), Some(ValueExpr::Operand(op.clone()))),
                        Instruction::BinaryOp(dest, op_kind, left, right) => (Some(*dest), Some(ValueExpr::BinaryOp(*op_kind, left.clone(), right.clone()))),
                        Instruction::UnaryOp(dest, op_kind, val) => (Some(*dest), Some(ValueExpr::UnaryOp(*op_kind, val.clone()))),
                        _ => (None, None),
                    };

                    let mut keep = true;
                    if let (Some(d), Some(e)) = (dest, expr) {
                        if let Some(&existing_val) = value_table.get(&e) {
                            replacements.insert(d, existing_val);
                            stats.gvn_eliminated += 1;
                            keep = false;
                        } else {
                            value_table.insert(e, d);
                        }
                    }

                    if keep {
                        new_instrs.push(instr);
                    }
                }

                block.instrs = new_instrs;

                match &mut block.terminator {
                    Terminator::Return(Some(op)) => {
                        if let Operand::Var(v) = op {
                            if let Some(&new_v) = replacements.get(v) {
                                *v = new_v;
                            }
                        }
                    }
                    Terminator::Branch(cond, _, _) => {
                        if let Operand::Var(v) = cond {
                            if let Some(&new_v) = replacements.get(v) {
                                *v = new_v;
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        stats
    }
}
