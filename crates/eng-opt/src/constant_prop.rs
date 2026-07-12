use std::fmt::Display;
use std::hash::Hash;
use std::collections::HashMap;
use eng_mir::{Instruction, MirModule, Operand, Terminator};
use crate::{OptimizationPass, PassStats};

pub struct ConstantPropagationPass;

impl<V: Clone + Copy + Display + Eq + Hash> OptimizationPass<V> for ConstantPropagationPass {
    fn name(&self) -> &'static str {
        "Constant Propagation"
    }

    fn run(&mut self, module: &mut MirModule<V>) -> PassStats {
        let mut stats = PassStats::default();

        for func in &mut module.functions {
            // Step 1: Count assignments to each variable
            let mut assign_counts = HashMap::new();
            for block in &func.blocks {
                for instr in &block.instrs {
                    match instr {
                        Instruction::<V>::Assign(dest, _) |
                        Instruction::<V>::LoadField(dest, _, _) |
                        Instruction::<V>::Call(dest, _, _) |
                        Instruction::<V>::Borrow(dest, _) |
                        Instruction::<V>::BorrowMut(dest, _) |
                        Instruction::<V>::Deref(dest, _, _) |
                        Instruction::<V>::HeapAllocate(dest, _) |
                        Instruction::<V>::StackAllocate(dest, _) |
                        Instruction::<V>::BinaryOp(dest, _, _, _) |
                        Instruction::<V>::UnaryOp(dest, _, _) |
                        Instruction::<V>::Phi(dest, _) => {
                            *assign_counts.entry(*dest).or_insert(0) += 1;
                        }
                        Instruction::<V>::StoreField(_, _, _) |
                        Instruction::<V>::Drop(_) => {} // doesn't assign to a var
                    }
                }
            }

            // Step 2: Find variables assigned exactly once to a constant
            let mut constant_vars = HashMap::new();
            for block in &func.blocks {
                for instr in &block.instrs {
                    if let Instruction::<V>::Assign(dest, Operand::<V>::Constant(lit)) = instr {
                        if assign_counts.get(dest) == Some(&1) {
                            constant_vars.insert(*dest, lit.clone());
                        }
                    }
                }
            }

            // Step 3: Replace uses
            let mut replace_operand = |op: &mut Operand<V>| {
                if let Operand::<V>::Var(id) = op {
                    if let Some(lit) = constant_vars.get(id) {
                        *op = Operand::<V>::Constant(lit.clone());
                        stats.folded_constants += 1;
                    }
                }
            };

            for block in &mut func.blocks {
                for instr in &mut block.instrs {
                    match instr {
                        Instruction::<V>::Assign(_, op) => replace_operand(op),
                        Instruction::<V>::LoadField(_, obj, _) => replace_operand(obj),
                        Instruction::<V>::StoreField(_, _, val) => replace_operand(val),
                        Instruction::<V>::Call(_, _, args) => {
                            for arg in args {
                                replace_operand(arg);
                            }
                        }
                        Instruction::<V>::Borrow(_, _) | Instruction::<V>::BorrowMut(_, _) => {}
                        Instruction::<V>::Deref(_, op, _) => replace_operand(op),
                        Instruction::<V>::Drop(_) => {}
                        Instruction::<V>::HeapAllocate(_, _) |
                        Instruction::<V>::StackAllocate(_, _) => {}
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
