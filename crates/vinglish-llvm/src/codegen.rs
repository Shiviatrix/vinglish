use std::collections::HashMap;

use inkwell::basic_block::BasicBlock as LLVMBasicBlock;
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::types::{BasicMetadataTypeEnum, BasicTypeEnum};
use inkwell::values::{BasicValueEnum, FunctionValue, IntValue};
use inkwell::{FloatPredicate, IntPredicate};

use vinglish_hir::symbol::{FunctionId, SsaValueId, SymbolTable, TypeId};
use vinglish_hir::types::Type;
use vinglish_mir::{BasicBlock, BlockId, Instruction, MirFunction, MirModule, Operand, Terminator};
use vinglish_parser::ast::{BinOp, Literal, UnOp};

use crate::builtins::Builtins;
use crate::types::TypeLowering;

/// Controls how a function's return terminator is emitted.
#[derive(Clone, Copy)]
enum ReturnMode {
    /// Function returns void (Vinglish Unit)
    Void,
    /// Function is `main` — always emit `ret i32 0`
    MainEntrypoint,
    /// Normal value-returning function
    Value,
}

pub struct LLVMCodeGen<'ctx> {
    pub context: &'ctx Context,
    pub module: Module<'ctx>,
    pub builder: Builder<'ctx>,
    type_lowering: TypeLowering<'ctx>,
    builtins: Builtins<'ctx>,
    symbol_table: &'ctx SymbolTable,

    // Per-function state
    ssa_values: HashMap<SsaValueId, BasicValueEnum<'ctx>>,
    block_map: HashMap<BlockId, LLVMBasicBlock<'ctx>>,
    func_map: HashMap<FunctionId, FunctionValue<'ctx>>,

    // Struct type cache: TypeId -> LLVM StructType with resolved body
    struct_types: HashMap<u32, inkwell::types::StructType<'ctx>>,
}

impl<'ctx> LLVMCodeGen<'ctx> {
    pub fn new(context: &'ctx Context, module_name: &str, symbol_table: &'ctx SymbolTable) -> Self {
        let module = context.create_module(module_name);
        let builder = context.create_builder();
        let type_lowering = TypeLowering::new(context);
        let builtins = Builtins::declare(context, &module);

        Self {
            context,
            module,
            builder,
            type_lowering,
            builtins,
            symbol_table,
            ssa_values: HashMap::new(),
            block_map: HashMap::new(),
            func_map: HashMap::new(),
            struct_types: HashMap::new(),
        }
    }

    /// Compile an entire MIR module to LLVM IR.
    pub fn compile_module(&mut self, mir: &MirModule<SsaValueId>) -> Result<(), String> {
        // First pass: declare all functions
        for func in &mir.functions {
            self.declare_function(func)?;
        }

        // Second pass: compile function bodies
        for func in &mir.functions {
            self.compile_function(func)?;
        }

        // Verify the module
        self.module
            .verify()
            .map_err(|e| format!("LLVM verification failed: {}", e.to_string()))
    }

    fn get_or_create_struct_type(&mut self, type_id: TypeId) -> inkwell::types::StructType<'ctx> {
        let key = type_id.0 .0;
        if let Some(&st) = self.struct_types.get(&key) {
            return st;
        }
        let st = self
            .type_lowering
            .lower_struct_type(self.symbol_table, type_id);
        self.struct_types.insert(key, st);
        st
    }

    fn get_func_return_type(&self, func: &MirFunction<SsaValueId>) -> Option<BasicTypeEnum<'ctx>> {
        if let Some(fs) = self.symbol_table.get_func(func.id) {
            if let Type::Function(_, ret) = &fs.ty {
                return self.type_lowering.lower_type(ret, self.symbol_table);
            }
        }
        None // void
    }

    fn get_func_param_types(
        &self,
        func: &MirFunction<SsaValueId>,
    ) -> Vec<BasicMetadataTypeEnum<'ctx>> {
        if let Some(fs) = self.symbol_table.get_func(func.id) {
            if let Type::Function(params, _) = &fs.ty {
                return params
                    .iter()
                    .map(
                        |p| match self.type_lowering.lower_type(p, self.symbol_table) {
                            Some(t) => t.into(),
                            None => self.context.i64_type().into(),
                        },
                    )
                    .collect();
            }
        }
        vec![]
    }

    fn declare_function(&mut self, func: &MirFunction<SsaValueId>) -> Result<(), String> {
        let param_types = self.get_func_param_types(func);
        let ret_type = self.get_func_return_type(func);

        // `main` must return i32 for C runtime compatibility, regardless of Vinglish type
        let fn_type = if func.name == "main" {
            self.context.i32_type().fn_type(&[], false)
        } else {
            match ret_type {
                Some(BasicTypeEnum::IntType(t)) => t.fn_type(&param_types, false),
                Some(BasicTypeEnum::FloatType(t)) => t.fn_type(&param_types, false),
                Some(BasicTypeEnum::PointerType(t)) => t.fn_type(&param_types, false),
                Some(BasicTypeEnum::StructType(t)) => t.fn_type(&param_types, false),
                Some(BasicTypeEnum::ArrayType(t)) => t.fn_type(&param_types, false),
                Some(BasicTypeEnum::VectorType(t)) => t.fn_type(&param_types, false),
                Some(BasicTypeEnum::ScalableVectorType(t)) => t.fn_type(&param_types, false),
                None => self.context.void_type().fn_type(&param_types, false),
            }
        };

        let llvm_func = if let Some(existing) = self.module.get_function(&func.name) {
            existing
        } else {
            self.module.add_function(&func.name, fn_type, None)
        };
        self.func_map.insert(func.id, llvm_func);
        Ok(())
    }

    fn compile_function(&mut self, func: &MirFunction<SsaValueId>) -> Result<(), String> {
        let llvm_func = *self
            .func_map
            .get(&func.id)
            .ok_or_else(|| format!("Function {} not declared", func.name))?;

        if func.is_foreign {
            return Ok(());
        }

        // Determine return mode
        let return_mode = if func.name == "main" {
            ReturnMode::MainEntrypoint
        } else if self.get_func_return_type(func).is_none() {
            ReturnMode::Void
        } else {
            ReturnMode::Value
        };

        // Clear per-function state
        self.ssa_values.clear();
        self.block_map.clear();

        // Pre-create all basic blocks
        for block in &func.blocks {
            let bb = self
                .context
                .append_basic_block(llvm_func, &format!("bb{}", block.id.0));
            self.block_map.insert(block.id, bb);
        }

        // Map function parameters to SSA values
        for (i, &param_id) in func.params.iter().enumerate() {
            let param_val = llvm_func
                .get_nth_param(i as u32)
                .ok_or_else(|| format!("Missing param {} in {}", i, func.name))?;
            self.ssa_values.insert(param_id, param_val);
        }

        // Compile each basic block
        for block in &func.blocks {
            self.compile_block(block, llvm_func, return_mode)?;
        }

        Ok(())
    }

    fn compile_block(
        &mut self,
        block: &BasicBlock<SsaValueId>,
        _func: FunctionValue<'ctx>,
        mode: ReturnMode,
    ) -> Result<(), String> {
        let llvm_bb = *self
            .block_map
            .get(&block.id)
            .ok_or_else(|| format!("Block {} not found", block.id))?;
        self.builder.position_at_end(llvm_bb);

        // Compile instructions
        for instr in &block.instrs {
            self.compile_instruction(instr)?;
        }

        // Compile terminator
        self.compile_terminator(&block.terminator, mode)?;

        Ok(())
    }

    fn resolve_operand(&self, op: &Operand<SsaValueId>) -> Result<BasicValueEnum<'ctx>, String> {
        match op {
            Operand::Constant(lit) => self.lower_literal(lit),
            Operand::Var(id) => self
                .ssa_values
                .get(id)
                .copied()
                .ok_or_else(|| format!("SSA value {} not found", id)),
        }
    }

    fn lower_literal(&self, lit: &Literal) -> Result<BasicValueEnum<'ctx>, String> {
        match lit {
            Literal::Int(n) => Ok(self.context.i64_type().const_int(*n as u64, true).into()),
            Literal::Float(f) => Ok(self.context.f64_type().const_float(*f).into()),
            Literal::Bool(b) => Ok(self
                .context
                .bool_type()
                .const_int(if *b { 1 } else { 0 }, false)
                .into()),
            Literal::Text(s) => {
                let global = self
                    .builder
                    .build_global_string_ptr(s, "str")
                    .map_err(|e| e.to_string())?;
                Ok(global.as_pointer_value().into())
            }
            Literal::Unit => {
                // Unit becomes 0i64
                Ok(self.context.i64_type().const_int(0, false).into())
            }
        }
    }

    fn compile_instruction(&mut self, instr: &Instruction<SsaValueId>) -> Result<(), String> {
        match instr {
            Instruction::Assign(dest, op) => {
                let val = self.resolve_operand(op)?;
                self.ssa_values.insert(*dest, val);
            }
            Instruction::CallIntrinsic(_dest, _name, _args) => {
                // Not supported in LLVM yet, just dummy
            }

            Instruction::BinaryOp(dest, op, left, right) => {
                let lhs = self.resolve_operand(left)?;
                let rhs = self.resolve_operand(right)?;
                let result = self.compile_binop(*op, lhs, rhs, &format!("ssa_{}", dest.0))?;
                self.ssa_values.insert(*dest, result);
            }

            Instruction::UnaryOp(dest, op, operand) => {
                let val = self.resolve_operand(operand)?;
                let result = self.compile_unop(*op, val, &format!("ssa_{}", dest.0))?;
                self.ssa_values.insert(*dest, result);
            }

            Instruction::Call(dest, target, args) => {
                let func_id = match target { vinglish_mir::CallTarget::Direct(id) => *id, vinglish_mir::CallTarget::Foreign { c_symbol } => return Err(format!("foreign MIR call `{c_symbol}` is not implemented by the LLVM backend")) };
                let result = self.compile_call(func_id, args, &format!("ssa_{}", dest.0))?;
                if let Some(val) = result {
                    self.ssa_values.insert(*dest, val);
                } else {
                    // Void call — insert a dummy unit value
                    self.ssa_values
                        .insert(*dest, self.context.i64_type().const_int(0, false).into());
                }
            }

            Instruction::HeapAllocate(dest, layout) => {
                let struct_type = self.get_or_create_struct_type(layout.layout);
                let size = struct_type
                    .size_of()
                    .ok_or_else(|| "Cannot get struct size".to_string())?;
                let ptr = self
                    .builder
                    .build_call(
                        self.builtins.malloc_fn,
                        &[size.into()],
                        &format!("ssa_{}", dest.0),
                    )
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .ok_or_else(|| "malloc returned void".to_string())?;
                self.ssa_values.insert(*dest, ptr);
            }

            Instruction::StackAllocate(dest, layout) => {
                let struct_type = self.get_or_create_struct_type(layout.layout);
                let alloca = self
                    .builder
                    .build_alloca(struct_type, &format!("ssa_{}", dest.0))
                    .map_err(|e| e.to_string())?;
                self.ssa_values.insert(*dest, alloca.into());
            }

            Instruction::LoadField(dest, obj, field_id) => {
                let obj_ptr = self.resolve_operand(obj)?;
                let ptr_val = obj_ptr.into_pointer_value();

                // We need to figure out the struct type for the GEP.
                // For now, we infer it from what we know about the object.
                // In a more complete implementation, we'd carry type information through MIR.
                let field_idx = field_id.field_id.0 as u32;

                // Try to find the struct type from the obj's allocation
                let struct_type = self
                    .infer_struct_type_for_ptr(obj)
                    .unwrap_or_else(|| self.context.opaque_struct_type("unknown_field_load"));

                let field_ptr = self
                    .builder
                    .build_struct_gep(
                        struct_type,
                        ptr_val,
                        field_idx,
                        &format!("field_{}_ptr", field_idx),
                    )
                    .map_err(|e| format!("GEP failed for field {}: {}", field_idx, e))?;

                // Determine field type
                let field_type = struct_type
                    .get_field_type_at_index(field_idx)
                    .ok_or_else(|| format!("No field at index {}", field_idx))?;

                let loaded = self
                    .builder
                    .build_load(field_type, field_ptr, &format!("ssa_{}", dest.0))
                    .map_err(|e| e.to_string())?;
                self.ssa_values.insert(*dest, loaded);
            }

            Instruction::StoreField(obj, field_id, val) => {
                let obj_val = self
                    .ssa_values
                    .get(obj)
                    .copied()
                    .ok_or_else(|| format!("SSA value {} not found for store", obj))?;
                let ptr_val = obj_val.into_pointer_value();
                let store_val = self.resolve_operand(val)?;
                let field_idx = field_id.field_id.0 as u32;

                let struct_type = self
                    .infer_struct_type_for_var(*obj)
                    .unwrap_or_else(|| self.context.opaque_struct_type("unknown_field_store"));

                let field_ptr = self
                    .builder
                    .build_struct_gep(
                        struct_type,
                        ptr_val,
                        field_idx,
                        &format!("store_field_{}", field_idx),
                    )
                    .map_err(|e| format!("GEP failed for store field {}: {}", field_idx, e))?;
                self.builder
                    .build_store(field_ptr, store_val)
                    .map_err(|e| e.to_string())?;
            }

            Instruction::Borrow(dest, src) | Instruction::BorrowMut(dest, src) => {
                let val = self.resolve_operand(src)?;
                if val.is_pointer_value() {
                    // It's already a pointer (e.g. heap object or another reference)
                    self.ssa_values.insert(*dest, val);
                } else {
                    // It's a primitive value in a register. We need a memory address to borrow it.
                    let alloca = self
                        .builder
                        .build_alloca(val.get_type(), &format!("borrow_{}", dest.0))
                        .map_err(|e| e.to_string())?;
                    self.builder
                        .build_store(alloca, val)
                        .map_err(|e| e.to_string())?;
                    self.ssa_values.insert(*dest, alloca.into());
                }
            }

            Instruction::Deref(dest, src, type_id) => {
                let ptr_val = self.resolve_operand(src)?;
                if !ptr_val.is_pointer_value() {
                    return Err(format!("Cannot deref non-pointer value {:?}", ptr_val));
                }
                let ptr = ptr_val.into_pointer_value();

                let ty = self
                    .symbol_table
                    .get_interned_type(*type_id)
                    .ok_or_else(|| format!("Type {} not found in intern table", type_id.0 .0))?;

                let llvm_type = self
                    .type_lowering
                    .lower_type(ty, self.symbol_table)
                    .ok_or_else(|| format!("Could not lower type {} for deref", ty))?;

                let loaded = self
                    .builder
                    .build_load(llvm_type, ptr, &format!("deref_{}", dest.0))
                    .map_err(|e| e.to_string())?;
                self.ssa_values.insert(*dest, loaded);
            }

            Instruction::Drop(var) => {
                // For heap allocations, call free. For stack, no-op.
                // In a more complete system, we'd track which are heap vs stack.
                // For now, Drop is a no-op — the ownership system already validated correctness.
                let _ = var;
            }

            Instruction::Phi(dest, args) => {
                // Phi nodes need special handling — we build the phi first,
                // then add incoming values after all blocks are compiled.
                // But since we compile blocks sequentially, we need to
                // defer phi incoming edges.
                //
                // Inkwell requires us to build phi at the block start.
                // We handle this by building the phi node here and then
                // adding incoming values.

                // Determine the type from the first available operand
                let phi_type = self.infer_phi_type(args)?;

                let phi = self
                    .builder
                    .build_phi(phi_type, &format!("ssa_{}", dest.0))
                    .map_err(|e| e.to_string())?;

                // Add incoming values (some may reference not-yet-compiled blocks,
                // but their SSA values should already exist since SSA dominance ensures
                // defs dominate uses, except for loop back-edges which use phi)
                for (op, block_id) in args {
                    if let Ok(val) = self.resolve_operand(op) {
                        if let Some(&bb) = self.block_map.get(block_id) {
                            phi.add_incoming(&[(&val, bb)]);
                        }
                    }
                }

                self.ssa_values.insert(*dest, phi.as_basic_value());
            }
        }
        Ok(())
    }

    fn compile_terminator(
        &mut self,
        term: &Terminator<SsaValueId>,
        mode: ReturnMode,
    ) -> Result<(), String> {
        match term {
            Terminator::Return(None) => match mode {
                ReturnMode::MainEntrypoint => {
                    let zero = self.context.i32_type().const_int(0, false);
                    self.builder
                        .build_return(Some(&zero))
                        .map_err(|e| e.to_string())?;
                }
                _ => {
                    self.builder.build_return(None).map_err(|e| e.to_string())?;
                }
            },
            Terminator::Return(Some(op)) => match mode {
                ReturnMode::Void => {
                    self.builder.build_return(None).map_err(|e| e.to_string())?;
                }
                ReturnMode::MainEntrypoint => {
                    let val = self.resolve_operand(op)?;
                    if val.is_int_value() {
                        let i32_val = self
                            .builder
                            .build_int_cast(
                                val.into_int_value(),
                                self.context.i32_type(),
                                "main_ret_cast",
                            )
                            .map_err(|e| e.to_string())?;
                        self.builder
                            .build_return(Some(&i32_val))
                            .map_err(|e| e.to_string())?;
                    } else {
                        let zero = self.context.i32_type().const_int(0, false);
                        self.builder
                            .build_return(Some(&zero))
                            .map_err(|e| e.to_string())?;
                    }
                }
                ReturnMode::Value => {
                    let val = self.resolve_operand(op)?;
                    self.builder
                        .build_return(Some(&val))
                        .map_err(|e| e.to_string())?;
                }
            },
            Terminator::Jump(target) => {
                let bb = self
                    .block_map
                    .get(target)
                    .ok_or_else(|| format!("Jump target {} not found", target))?;
                self.builder
                    .build_unconditional_branch(*bb)
                    .map_err(|e| e.to_string())?;
            }
            Terminator::Branch(cond, true_block, false_block) => {
                let cond_val = self.resolve_operand(cond)?;
                let cond_int = cond_val.into_int_value();
                let true_bb = self
                    .block_map
                    .get(true_block)
                    .ok_or_else(|| format!("True branch {} not found", true_block))?;
                let false_bb = self
                    .block_map
                    .get(false_block)
                    .ok_or_else(|| format!("False branch {} not found", false_block))?;
                self.builder
                    .build_conditional_branch(cond_int, *true_bb, *false_bb)
                    .map_err(|e| e.to_string())?;
            }
        }
        Ok(())
    }

    fn compile_binop(
        &self,
        op: BinOp,
        lhs: BasicValueEnum<'ctx>,
        rhs: BasicValueEnum<'ctx>,
        name: &str,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        // Determine if we're doing integer or float arithmetic
        if lhs.is_int_value() && rhs.is_int_value() {
            let l = lhs.into_int_value();
            let r = rhs.into_int_value();
            let result: IntValue<'ctx> = match op {
                BinOp::Add => self
                    .builder
                    .build_int_add(l, r, name)
                    .map_err(|e| e.to_string())?,
                BinOp::Sub => self
                    .builder
                    .build_int_sub(l, r, name)
                    .map_err(|e| e.to_string())?,
                BinOp::Mul => self
                    .builder
                    .build_int_mul(l, r, name)
                    .map_err(|e| e.to_string())?,
                BinOp::Div => self
                    .builder
                    .build_int_signed_div(l, r, name)
                    .map_err(|e| e.to_string())?,
                BinOp::Mod => self
                    .builder
                    .build_int_signed_rem(l, r, name)
                    .map_err(|e| e.to_string())?,
                BinOp::Eq => self
                    .builder
                    .build_int_compare(IntPredicate::EQ, l, r, name)
                    .map_err(|e| e.to_string())?,
                BinOp::NotEq => self
                    .builder
                    .build_int_compare(IntPredicate::NE, l, r, name)
                    .map_err(|e| e.to_string())?,
                BinOp::Lt => self
                    .builder
                    .build_int_compare(IntPredicate::SLT, l, r, name)
                    .map_err(|e| e.to_string())?,
                BinOp::Gt => self
                    .builder
                    .build_int_compare(IntPredicate::SGT, l, r, name)
                    .map_err(|e| e.to_string())?,
                BinOp::LtEq => self
                    .builder
                    .build_int_compare(IntPredicate::SLE, l, r, name)
                    .map_err(|e| e.to_string())?,
                BinOp::GtEq => self
                    .builder
                    .build_int_compare(IntPredicate::SGE, l, r, name)
                    .map_err(|e| e.to_string())?,
                BinOp::And => self
                    .builder
                    .build_and(l, r, name)
                    .map_err(|e| e.to_string())?,
                BinOp::Or => self
                    .builder
                    .build_or(l, r, name)
                    .map_err(|e| e.to_string())?,
                BinOp::IsBelow => self
                    .builder
                    .build_int_compare(IntPredicate::SLT, l, r, name)
                    .map_err(|e| e.to_string())?,
                BinOp::IsAbove => self
                    .builder
                    .build_int_compare(IntPredicate::SGT, l, r, name)
                    .map_err(|e| e.to_string())?,
                BinOp::Exceeds => self
                    .builder
                    .build_int_compare(IntPredicate::SGT, l, r, name)
                    .map_err(|e| e.to_string())?,
            };
            Ok(result.into())
        } else if lhs.is_float_value() && rhs.is_float_value() {
            let l = lhs.into_float_value();
            let r = rhs.into_float_value();
            match op {
                BinOp::Add => Ok(self
                    .builder
                    .build_float_add(l, r, name)
                    .map_err(|e| e.to_string())?
                    .into()),
                BinOp::Sub => Ok(self
                    .builder
                    .build_float_sub(l, r, name)
                    .map_err(|e| e.to_string())?
                    .into()),
                BinOp::Mul => Ok(self
                    .builder
                    .build_float_mul(l, r, name)
                    .map_err(|e| e.to_string())?
                    .into()),
                BinOp::Div => Ok(self
                    .builder
                    .build_float_div(l, r, name)
                    .map_err(|e| e.to_string())?
                    .into()),
                BinOp::Mod => Ok(self
                    .builder
                    .build_float_rem(l, r, name)
                    .map_err(|e| e.to_string())?
                    .into()),
                BinOp::Eq => Ok(self
                    .builder
                    .build_float_compare(FloatPredicate::OEQ, l, r, name)
                    .map_err(|e| e.to_string())?
                    .into()),
                BinOp::NotEq => Ok(self
                    .builder
                    .build_float_compare(FloatPredicate::ONE, l, r, name)
                    .map_err(|e| e.to_string())?
                    .into()),
                BinOp::Lt => Ok(self
                    .builder
                    .build_float_compare(FloatPredicate::OLT, l, r, name)
                    .map_err(|e| e.to_string())?
                    .into()),
                BinOp::Gt => Ok(self
                    .builder
                    .build_float_compare(FloatPredicate::OGT, l, r, name)
                    .map_err(|e| e.to_string())?
                    .into()),
                BinOp::LtEq => Ok(self
                    .builder
                    .build_float_compare(FloatPredicate::OLE, l, r, name)
                    .map_err(|e| e.to_string())?
                    .into()),
                BinOp::GtEq => Ok(self
                    .builder
                    .build_float_compare(FloatPredicate::OGE, l, r, name)
                    .map_err(|e| e.to_string())?
                    .into()),
                _ => Err(format!("Unsupported float binop: {:?}", op)),
            }
        } else {
            Err(format!("Type mismatch in binop: {:?}", op))
        }
    }

    fn compile_unop(
        &self,
        op: UnOp,
        val: BasicValueEnum<'ctx>,
        name: &str,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        match op {
            UnOp::Neg => {
                if val.is_int_value() {
                    Ok(self
                        .builder
                        .build_int_neg(val.into_int_value(), name)
                        .map_err(|e| e.to_string())?
                        .into())
                } else if val.is_float_value() {
                    Ok(self
                        .builder
                        .build_float_neg(val.into_float_value(), name)
                        .map_err(|e| e.to_string())?
                        .into())
                } else {
                    Err("Cannot negate non-numeric value".into())
                }
            }
            UnOp::Not => {
                if val.is_int_value() {
                    Ok(self
                        .builder
                        .build_not(val.into_int_value(), name)
                        .map_err(|e| e.to_string())?
                        .into())
                } else {
                    Err("Cannot NOT non-integer value".into())
                }
            }
            UnOp::Deref => {
                if val.is_pointer_value() {
                    // We need to know the type we are loading!
                    // Unfortunately, `UnOp` doesn't know the return type directly without context.
                    // But wait! compile_unop doesn't have the type.
                    // This is why deref might need to be handled differently, or we can use an opaque type if we use opaque pointers.
                    // Actually, Inkwell requires the type to load.
                    // If we pass the type into compile_unop, we can load it.
                    // For now, let's just return an error because it's hard.
                    Err("Deref unimplemented in LLVM codegen without type info".into())
                } else {
                    Err("Cannot dereference non-pointer".into())
                }
            }
            UnOp::Borrow(_) => Err("Borrow should be an Instruction, not a UnaryOp".into()),
        }
    }

    fn compile_call(
        &mut self,
        func_id: FunctionId,
        args: &[Operand<SsaValueId>],
        name: &str,
    ) -> Result<Option<BasicValueEnum<'ctx>>, String> {
        // Check if it's a known built-in
        if let Some(fs) = self.symbol_table.get_func(func_id) {
            if fs.name == "print" || fs.name == "print_num" {
                return self.compile_print_call(args);
            }
        }

        let llvm_func = self
            .func_map
            .get(&func_id)
            .ok_or_else(|| format!("Function fn_{} not found in LLVM module", func_id.0 .0))?;

        let mut arg_values: Vec<inkwell::values::BasicMetadataValueEnum<'ctx>> = Vec::new();
        for arg in args {
            let val = self.resolve_operand(arg)?;
            arg_values.push(val.into());
        }

        let call_result = self
            .builder
            .build_call(*llvm_func, &arg_values, name)
            .map_err(|e| e.to_string())?;

        Ok(call_result.try_as_basic_value().basic())
    }

    fn compile_print_call(
        &mut self,
        args: &[Operand<SsaValueId>],
    ) -> Result<Option<BasicValueEnum<'ctx>>, String> {
        if args.is_empty() {
            return Ok(None);
        }

        let val = self.resolve_operand(&args[0])?;

        // Determine format string based on type
        let (fmt, print_args): (&str, Vec<inkwell::values::BasicMetadataValueEnum<'ctx>>) =
            if val.is_int_value() {
                ("%lld\n", vec![val.into()])
            } else if val.is_float_value() {
                ("%f\n", vec![val.into()])
            } else if val.is_pointer_value() {
                ("%s\n", vec![val.into()])
            } else {
                ("<value>\n", vec![])
            };

        let fmt_str = self
            .builder
            .build_global_string_ptr(fmt, "fmt")
            .map_err(|e| e.to_string())?;

        let mut call_args: Vec<inkwell::values::BasicMetadataValueEnum<'ctx>> =
            vec![fmt_str.as_pointer_value().into()];
        call_args.extend(print_args);

        self.builder
            .build_call(self.builtins.printf_fn, &call_args, "printf_call")
            .map_err(|e| e.to_string())?;

        Ok(None)
    }

    fn infer_phi_type(
        &self,
        args: &[(Operand<SsaValueId>, BlockId)],
    ) -> Result<BasicTypeEnum<'ctx>, String> {
        for (op, _) in args {
            if let Ok(val) = self.resolve_operand(op) {
                return Ok(val.get_type());
            }
        }
        // Default to i64 if we can't infer
        Ok(self.context.i64_type().into())
    }

    /// Try to infer the LLVM struct type for an operand that is a pointer to a struct.
    fn infer_struct_type_for_ptr(
        &self,
        op: &Operand<SsaValueId>,
    ) -> Option<inkwell::types::StructType<'ctx>> {
        if let Operand::Var(id) = op {
            self.infer_struct_type_for_var(*id)
        } else {
            None
        }
    }

    fn infer_struct_type_for_var(
        &self,
        _id: SsaValueId,
    ) -> Option<inkwell::types::StructType<'ctx>> {
        // Walk the struct_types cache — in a fully typed MIR this would be a type lookup.
        // For now return the first struct type if there's only one, or None.
        if self.struct_types.len() == 1 {
            self.struct_types.values().next().copied()
        } else {
            // With multiple struct types, we'd need type info on SSA values.
            // For the initial implementation, return the first one as a best guess.
            self.struct_types.values().next().copied()
        }
    }

    /// Get the LLVM IR as a string.
    pub fn get_ir_string(&self) -> String {
        self.module.print_to_string().to_string()
    }
}
