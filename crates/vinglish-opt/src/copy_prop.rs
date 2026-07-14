use vinglish_hir::symbol::HasSymbolId;
use crate::{OptimizationPass, PassStats};
use vinglish_mir::{Instruction, MirModule, Operand, Terminator};
use std::collections::HashMap;
use std::fmt::Display;
use std::hash::Hash;

pub struct CopyPropagationPass;

impl<V: Clone + Copy + Display + Eq + Hash + vinglish_hir::symbol::HasSymbolId> OptimizationPass<V> for CopyPropagationPass {
    fn name(&self) -> &'static str {
        "Copy Propagation"
    }

    fn run(&mut self, module: &mut MirModule<V>, symbol_table: &vinglish_hir::symbol::SymbolTable) -> PassStats {
        let stats = PassStats::default();

        for func in &mut module.functions {
            let mut assign_counts = HashMap::new();
            for block in &func.blocks {
                for instr in &block.instrs {
                    match instr {
                        Instruction::<V>::Assign(dest, _)
                        | Instruction::<V>::LoadField(dest, _, _)
                        | Instruction::<V>::Call(dest, _, _)
                        | Instruction::<V>::CallIntrinsic(dest, _, _)
                        | Instruction::<V>::Borrow(dest, _)
                        | Instruction::<V>::BorrowMut(dest, _)
                        | Instruction::<V>::Deref(dest, _, _)
                        | Instruction::<V>::HeapAllocate(dest, _)
                        | Instruction::<V>::StackAllocate(dest, _)
                        | Instruction::<V>::BinaryOp(dest, _, _, _)
                        | Instruction::<V>::UnaryOp(dest, _, _)
                        | Instruction::<V>::Phi(dest, _) => {
                            *assign_counts.entry(*dest).or_insert(0) += 1;
                        }
                        Instruction::<V>::StoreField(_, _, _) | Instruction::<V>::Drop(_) => {}
                    }
                }
            }

            // Find single-assignment copies: dest = src
            
            let is_temp = |v: V| -> bool {
                if let Some(vinglish_hir::symbol::SymbolKind::Variable(vs)) =
                    symbol_table.get(v.symbol_id())
                {
                    vs.name.starts_with("_tmp")
                } else {
                    false
                }
            };

            let mut copy_vars = HashMap::new();
            for block in &func.blocks {
                for instr in &block.instrs {
                    if let Instruction::<V>::Assign(dest, Operand::<V>::Var(src)) = instr {
                        // Both dest and src must not be reassigned (assigned <= 1 times).
                        // Parameters are assigned 0 times in MIR (they start with values).
                        if assign_counts.get(dest) == Some(&1)
                            && assign_counts.get(src).copied().unwrap_or(0) <= 1
                        {
                            // Avoid replacing a user variable with a temporary
                            if !is_temp(*dest) && is_temp(*src) {
                                // DO NOT REPLACE
                            } else {
                                copy_vars.insert(*dest, *src);
                            }
                        }
                    }
                }
            }

            // Replace uses
            let replace_operand = |op: &mut Operand<V>| {
                if let Operand::<V>::Var(id) = op {
                    // Resolve copy chain
                    let mut current = *id;
                    while let Some(&next) = copy_vars.get(&current) {
                        current = next;
                    }
                    if current != *id {
                        *op = Operand::<V>::Var(current);
                        // We track it under folded_constants for stats simplicity,
                        // but maybe we should just track replaced operands or leave stats empty.
                    }
                }
            };

            for block in &mut func.blocks {
                for instr in &mut block.instrs {
                    match instr {
                        Instruction::<V>::Assign(_, op) => replace_operand(op),
                        Instruction::<V>::LoadField(_, obj, _) => replace_operand(obj),
                        Instruction::<V>::StoreField(_, _, op) => replace_operand(op),
                        Instruction::<V>::Call(_, _, args) | Instruction::<V>::CallIntrinsic(_, _, args) => {
                            for arg in args {
                                replace_operand(arg);
                            }
                        }
                        Instruction::<V>::Borrow(_, _) | Instruction::<V>::BorrowMut(_, _) => {}
                        Instruction::<V>::Deref(_, op, _) => replace_operand(op),
                        Instruction::<V>::Drop(_) => {}
                        Instruction::<V>::HeapAllocate(_, _)
                        | Instruction::<V>::StackAllocate(_, _) => {}
                        Instruction::<V>::BinaryOp(_, _, left, right) => {
                            replace_operand(left);
                            replace_operand(right);
                        }
                        Instruction::<V>::UnaryOp(_, _, operand) => replace_operand(operand),
                        Instruction::<V>::Phi(_, args) => {
                            for (op, _) in args {
                                replace_operand(op);
                            }
                        }
                    }
                }

                match &mut block.terminator {
                    Terminator::<V>::Return(Some(op)) => replace_operand(op),
                    Terminator::<V>::Branch(cond, _, _) => replace_operand(cond),
                    _ => {}
                }
            }
        }

        stats
    }
}
