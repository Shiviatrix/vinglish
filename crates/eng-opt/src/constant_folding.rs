use std::fmt::Display;
use std::hash::Hash;
use eng_mir::{Instruction, MirModule, Operand};
use eng_parser::ast::{BinOp, Literal, UnOp};
use crate::{OptimizationPass, PassStats};

pub struct ConstantFoldingPass;

impl<V: Clone + Copy + Display + Eq + Hash> OptimizationPass<V> for ConstantFoldingPass {
    fn name(&self) -> &'static str {
        "Constant Folding"
    }

    fn run(&mut self, module: &mut MirModule<V>) -> PassStats {
        let mut stats = PassStats::default();

        for func in &mut module.functions {
            for block in &mut func.blocks {
                for instr in &mut block.instrs {
                    if let Some(new_instr) = fold_instruction(instr) {
                        *instr = new_instr;
                        stats.folded_constants += 1;
                    }
                }
            }
        }

        stats
    }
}

fn fold_instruction<V: Clone + Copy + Display + Eq + Hash>(instr: &Instruction<V>) -> Option<Instruction<V>> {
    match instr {
        Instruction::BinaryOp(dest, op, Operand::Constant(left), Operand::Constant(right)) => {
            let result = fold_binop(*op, left, right)?;
            Some(Instruction::Assign(*dest, Operand::Constant(result)))
        }
        Instruction::UnaryOp(dest, op, Operand::Constant(operand)) => {
            let result = fold_unop(*op, operand)?;
            Some(Instruction::Assign(*dest, Operand::Constant(result)))
        }
        _ => None,
    }
}

fn fold_binop(op: BinOp, left: &Literal, right: &Literal) -> Option<Literal> {
    match (left, op, right) {
        (Literal::Int(a), BinOp::Add, Literal::Int(b)) => Some(Literal::Int(a + b)),
        (Literal::Int(a), BinOp::Sub, Literal::Int(b)) => Some(Literal::Int(a - b)),
        (Literal::Int(a), BinOp::Mul, Literal::Int(b)) => Some(Literal::Int(a * b)),
        (Literal::Int(a), BinOp::Div, Literal::Int(b)) => {
            if *b != 0 { Some(Literal::Int(a / b)) } else { None }
        }
        (Literal::Int(a), BinOp::Mod, Literal::Int(b)) => {
            if *b != 0 { Some(Literal::Int(a % b)) } else { None }
        }
        (Literal::Float(a), BinOp::Add, Literal::Float(b)) => Some(Literal::Float(a + b)),
        (Literal::Float(a), BinOp::Sub, Literal::Float(b)) => Some(Literal::Float(a - b)),
        (Literal::Float(a), BinOp::Mul, Literal::Float(b)) => Some(Literal::Float(a * b)),
        (Literal::Float(a), BinOp::Div, Literal::Float(b)) => Some(Literal::Float(a / b)),
        (Literal::Int(a), BinOp::Eq, Literal::Int(b)) => Some(Literal::Bool(a == b)),
        (Literal::Int(a), BinOp::NotEq, Literal::Int(b)) => Some(Literal::Bool(a != b)),
        (Literal::Int(a), BinOp::Lt, Literal::Int(b)) | (Literal::Int(a), BinOp::IsBelow, Literal::Int(b)) => Some(Literal::Bool(a < b)),
        (Literal::Int(a), BinOp::Gt, Literal::Int(b)) | (Literal::Int(a), BinOp::IsAbove, Literal::Int(b)) | (Literal::Int(a), BinOp::Exceeds, Literal::Int(b)) => Some(Literal::Bool(a > b)),
        (Literal::Int(a), BinOp::LtEq, Literal::Int(b)) => Some(Literal::Bool(a <= b)),
        (Literal::Int(a), BinOp::GtEq, Literal::Int(b)) => Some(Literal::Bool(a >= b)),
        (Literal::Bool(a), BinOp::And, Literal::Bool(b)) => Some(Literal::Bool(*a && *b)),
        (Literal::Bool(a), BinOp::Or, Literal::Bool(b)) => Some(Literal::Bool(*a || *b)),
        (Literal::Bool(a), BinOp::Eq, Literal::Bool(b)) => Some(Literal::Bool(*a == *b)),
        (Literal::Bool(a), BinOp::NotEq, Literal::Bool(b)) => Some(Literal::Bool(*a != *b)),
        _ => None,
    }
}

fn fold_unop(op: UnOp, operand: &Literal) -> Option<Literal> {
    match (op, operand) {
        (UnOp::Neg, Literal::Int(i)) => Some(Literal::Int(-i)),
        (UnOp::Neg, Literal::Float(f)) => Some(Literal::Float(-f)),
        (UnOp::Not, Literal::Bool(b)) => Some(Literal::Bool(!b)),
        _ => None,
    }
}
