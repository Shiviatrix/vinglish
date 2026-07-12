use eng_hir::symbol::VariableId;
pub mod dominators;
pub mod phi;
pub mod rename;
pub mod validator;

use eng_mir::MirModule;

pub use dominators::DominatorTree;
pub use validator::SSAValidator;

pub struct SSAConversionPass;

impl Default for SSAConversionPass {
    fn default() -> Self {
        Self::new()
    }
}

impl SSAConversionPass {
    pub fn new() -> Self {
        Self
    }

    pub fn run(&mut self, mut module: MirModule<VariableId>, symbol_table: &mut eng_hir::symbol::SymbolTable) -> MirModule<eng_hir::symbol::SsaValueId> {
        for func in &mut module.functions {
            let dom_tree = DominatorTree::new(func);
            phi::insert_phi_nodes(func, &dom_tree);
            rename::rename_variables(func, &dom_tree, symbol_table);
        }
        convert_to_ssa_types(module)
    }
}

fn convert_to_ssa_types(module: MirModule<VariableId>) -> MirModule<eng_hir::symbol::SsaValueId> {
    use eng_mir::{MirFunction, BasicBlock, Instruction, Terminator, Operand};
    use eng_hir::symbol::SsaValueId;

    let convert_var = |v: VariableId| SsaValueId(v.0.0);

    let convert_operand = |op: Operand<VariableId>| -> Operand<SsaValueId> {
        match op {
            Operand::Constant(lit) => Operand::Constant(lit),
            Operand::Var(v) => Operand::Var(convert_var(v)),
        }
    };

    let convert_instr = |instr: Instruction<VariableId>| -> Instruction<SsaValueId> {
        match instr {
            Instruction::Assign(dest, op) => Instruction::Assign(convert_var(dest), convert_operand(op)),
            Instruction::BinaryOp(dest, op_kind, left, right) => Instruction::BinaryOp(convert_var(dest), op_kind, convert_operand(left), convert_operand(right)),
            Instruction::UnaryOp(dest, op_kind, val) => Instruction::UnaryOp(convert_var(dest), op_kind, convert_operand(val)),
            Instruction::Call(dest, func_id, args) => Instruction::Call(convert_var(dest), func_id, args.into_iter().map(convert_operand).collect()),
            Instruction::HeapAllocate(dest, type_id) => Instruction::HeapAllocate(convert_var(dest), type_id),
            Instruction::StackAllocate(dest, type_id) => Instruction::StackAllocate(convert_var(dest), type_id),
            Instruction::LoadField(dest, obj, field) => Instruction::LoadField(convert_var(dest), convert_operand(obj), field),
            Instruction::StoreField(obj, field, val) => Instruction::StoreField(convert_var(obj), field, convert_operand(val)),
            Instruction::Borrow(dest, op) => Instruction::Borrow(convert_var(dest), convert_operand(op)),
            Instruction::BorrowMut(dest, op) => Instruction::BorrowMut(convert_var(dest), convert_operand(op)),
            Instruction::Deref(dest, op, ty) => Instruction::Deref(convert_var(dest), convert_operand(op), ty),
            Instruction::Drop(dest) => Instruction::Drop(convert_var(dest)),
            Instruction::Phi(dest, args) => Instruction::Phi(convert_var(dest), args.into_iter().map(|(op, b)| (convert_operand(op), b)).collect()),
        }
    };

    let convert_term = |term: Terminator<VariableId>| -> Terminator<SsaValueId> {
        match term {
            Terminator::Return(Some(op)) => Terminator::Return(Some(convert_operand(op))),
            Terminator::Return(None) => Terminator::Return(None),
            Terminator::Jump(tgt) => Terminator::Jump(tgt),
            Terminator::Branch(cond, t_tgt, f_tgt) => Terminator::Branch(convert_operand(cond), t_tgt, f_tgt),
        }
    };

    let convert_block = |block: BasicBlock<VariableId>| -> BasicBlock<SsaValueId> {
        BasicBlock {
            id: block.id,
            instrs: block.instrs.into_iter().map(convert_instr).collect(),
            terminator: convert_term(block.terminator),
        }
    };

    let convert_func = |func: MirFunction<VariableId>| -> MirFunction<SsaValueId> {
        MirFunction {
            id: func.id,
            is_foreign: func.is_foreign,
            name: func.name,
            params: func.params.into_iter().map(convert_var).collect(),
            locals: func.locals.into_iter().map(convert_var).collect(),
            blocks: func.blocks.into_iter().map(convert_block).collect(),
        }
    };

    MirModule {
        functions: module.functions.into_iter().map(convert_func).collect(),
    }
}
