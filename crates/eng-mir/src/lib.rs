pub mod validator;

use std::fmt;
use eng_hir::symbol::{FunctionId, FieldId, TypeId};
use eng_parser::ast::{BinOp, UnOp, Literal};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)]
pub struct BlockId(pub usize);

impl fmt::Display for BlockId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "bb{}", self.0)
    }
}

#[derive(Debug, Clone)]
pub struct MirModule<V: Clone + Copy + fmt::Display> {
    pub functions: Vec<MirFunction<V>>,
}

#[derive(Debug, Clone)]
pub struct MirFunction<V: Clone + Copy + fmt::Display> {
    pub id: FunctionId,
    pub is_foreign: bool,
    pub name: String,
    pub params: Vec<V>,
    pub blocks: Vec<BasicBlock<V>>,
    pub locals: Vec<V>, // includes parameters and synthesized temporaries
}

#[derive(Debug, Clone)]
pub struct BasicBlock<V: Clone + Copy + fmt::Display> {
    pub id: BlockId,
    pub instrs: Vec<Instruction<V>>,
    pub terminator: Terminator<V>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Operand<V: Clone + Copy + fmt::Display> {
    Constant(Literal),
    Var(V),
}

impl<V: Clone + Copy + fmt::Display> fmt::Display for Operand<V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Operand::Constant(lit) => write!(f, "{:?}", lit),
            Operand::Var(id) => write!(f, "{}", id),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Instruction<V: Clone + Copy + fmt::Display> {
    Assign(V, Operand<V>),
    LoadField(V, Operand<V>, FieldId),
    StoreField(V, FieldId, Operand<V>),
    Call(V, FunctionId, Vec<Operand<V>>),
    HeapAllocate(V, TypeId),
    StackAllocate(V, TypeId),
    BinaryOp(V, BinOp, Operand<V>, Operand<V>),
    UnaryOp(V, UnOp, Operand<V>),
    Borrow(V, Operand<V>),
    BorrowMut(V, Operand<V>),
    Deref(V, Operand<V>, TypeId),
    Drop(V),
    Phi(V, Vec<(Operand<V>, BlockId)>),
}

impl<V: Clone + Copy + fmt::Display> fmt::Display for Instruction<V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Instruction::Assign(dest, op) => write!(f, "{} = {}", dest, op),
            Instruction::LoadField(dest, obj, field) => write!(f, "{} = {}.field_{}", dest, obj, field.0),
            Instruction::StoreField(obj, field, val) => write!(f, "{}.field_{} = {}", obj, field.0, val),
            Instruction::Call(dest, func, args) => {
                let args_str = args.iter().map(|a| a.to_string()).collect::<Vec<_>>().join(", ");
                write!(f, "{} = call fn_{}({})", dest, func.0.0, args_str)
            }
            Instruction::HeapAllocate(dest, ty) => write!(f, "{} = heap_allocate type_{}", dest, ty.0.0),
            Instruction::StackAllocate(dest, ty) => write!(f, "{} = stack_allocate type_{}", dest, ty.0.0),
            Instruction::BinaryOp(dest, op, left, right) => write!(f, "{} = {} {:?} {}", dest, left, op, right),
            Instruction::UnaryOp(dest, op, operand) => write!(f, "{} = {:?} {}", dest, op, operand),
            Instruction::Borrow(dest, src) => write!(f, "{} = &{}", dest, src),
            Instruction::BorrowMut(dest, src) => write!(f, "{} = &mut {}", dest, src),
            Instruction::Deref(dest, src, _) => write!(f, "{} = *{}", dest, src),
            Instruction::Drop(var) => write!(f, "drop({})", var),
            Instruction::Phi(dest, args) => {
                let args_str = args.iter().map(|(op, block)| format!("{}: {}", block, op)).collect::<Vec<_>>().join(", ");
                write!(f, "{} = phi({})", dest, args_str)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Terminator<V: Clone + Copy + fmt::Display> {
    Return(Option<Operand<V>>),
    Jump(BlockId),
    Branch(Operand<V>, BlockId, BlockId),
}

impl<V: Clone + Copy + fmt::Display> fmt::Display for Terminator<V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Terminator::Return(Some(op)) => write!(f, "return {}", op),
            Terminator::Return(None) => write!(f, "return"),
            Terminator::Jump(block) => write!(f, "jump {}", block),
            Terminator::Branch(cond, true_block, false_block) => write!(f, "branch {} ? {} : {}", cond, true_block, false_block),
        }
    }
}

impl<V: Clone + Copy + fmt::Display> fmt::Display for BasicBlock<V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}:", self.id)?;
        for instr in &self.instrs {
            writeln!(f, "    {}", instr)?;
        }
        writeln!(f, "    {}", self.terminator)?;
        Ok(())
    }
}

impl<V: Clone + Copy + fmt::Display> fmt::Display for MirFunction<V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "fn {} (fn_{}) {{", self.name, self.id.0.0)?;
        for block in &self.blocks {
            write!(f, "{}", block)?;
        }
        writeln!(f, "}}")?;
        Ok(())
    }
}

impl<V: Clone + Copy + fmt::Display> fmt::Display for MirModule<V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for func in &self.functions {
            writeln!(f, "{}", func)?;
        }
        Ok(())
    }
}

impl<V: Clone + Copy + std::fmt::Display + Eq + std::hash::Hash> std::cmp::Eq for Operand<V> {}
impl<V: Clone + Copy + std::fmt::Display + Eq + std::hash::Hash> std::hash::Hash for Operand<V> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            Operand::Constant(c) => c.hash(state),
            Operand::Var(v) => v.hash(state),
        }
    }
}
