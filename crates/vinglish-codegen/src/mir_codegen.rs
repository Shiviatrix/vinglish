//! MIR-only C backend. AST nodes cannot enter this API.
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;
use thiserror::Error;
use vinglish_decompile::{emit_c_tag, MirInstructionId, MirTag, ReconstructionIndex};
use vinglish_hir::symbol::{FunctionId, SsaValueId, SymbolTable, VariableId};
use vinglish_hir::types::Type;
use vinglish_mir::{Instruction, MirFunction, MirModule, Operand, Terminator};
use vinglish_parser::ast::{BinOp, Literal, UnOp};

#[derive(Debug, Error)]
pub enum MirCEmitError { #[error("formatting C output failed")] Fmt(#[from] std::fmt::Error) }

/// The C backend only needs the stable numeric SSA identity; it deliberately
/// does not need an AST name or type annotation.
pub trait CValueId: Copy + std::fmt::Display + Eq { fn raw(self) -> u32; }
impl CValueId for VariableId { fn raw(self) -> u32 { self.0.0 } }
impl CValueId for SsaValueId { fn raw(self) -> u32 { self.0 } }

/// Emit C exclusively from optimized SSA MIR. Tags are C comments, removed by
/// standard preprocessing and therefore have zero runtime/object-code cost.
pub fn emit_mir_c<V: CValueId>(module: &MirModule<V>, symbols: &SymbolTable) -> Result<String, MirCEmitError> {
    let pool = StringPool::collect(module);
    let mut out = String::from("/* Generated from Vinglish SSA MIR. */\n#include <stdint.h>\n#include <stdio.h>\n#include <stdlib.h>\n#define print(x) _Generic((x), const char*: printf(\"%s\", x), char*: printf(\"%s\", x), default: printf(\"%ld\", (long)(x)))\n#define println(x) _Generic((x), const char*: printf(\"%s\\n\", x), char*: printf(\"%s\\n\", x), default: printf(\"%ld\\n\", (long)(x)))\n\n");
    for (text, index) in &pool.entries { writeln!(out, "static const char *const string_literal_{index} = \"{}\";", escape_c_string(text))?; }
    if !pool.entries.is_empty() { out.push('\n'); }
    for function in &module.functions {
        if !function.is_foreign {
            if function.name == "main" { out.push_str("int main(void);\n"); }
            else { write!(out, "static long fn_{}(", function.id.0.0)?; for (index, param) in function.params.iter().enumerate() { if index != 0 { out.push_str(", "); } out.push_str(c_value_type(*param, symbols)); } out.push_str(");\n"); }
        }
    }
    let foreign_symbols: BTreeSet<String> = module.functions.iter().flat_map(|function| function.blocks.iter()).flat_map(|block| block.instrs.iter()).filter_map(|instruction| match instruction { Instruction::Call(_, vinglish_mir::CallTarget::Foreign { c_symbol }, _) => Some(c_ident(c_symbol)), _ => None }).collect();
    for symbol in foreign_symbols { if symbol != "print" && symbol != "println" { writeln!(out, "extern long {}();", symbol)?; } }
    out.push('\n');
    for function in &module.functions { if !function.is_foreign { emit_function(&mut out, function, symbols, module, &pool)?; } }
    Ok(out)
}

fn emit_function<V: CValueId>(out: &mut String, function: &MirFunction<V>, symbols: &SymbolTable, module: &MirModule<V>, pool: &StringPool) -> Result<(), MirCEmitError> {
    if function.name == "main" { out.push_str("int main("); } else { write!(out, "static long fn_{}(", function.id.0.0)?; }
    for (index, param) in function.params.iter().enumerate() { if index != 0 { out.push_str(", "); } write!(out, "{} v_{}", c_value_type(*param, symbols), param.raw())?; }
    out.push_str(") {\n");
    for local in &function.locals { if !function.params.contains(local) { writeln!(out, "    {} v_{} = {};", c_value_type(*local, symbols), local.raw(), c_zero(*local, symbols))?; } }
    for block in &function.blocks {
        emit_tag(out, function.id.0.0, block.id.0 as u32, u32::MAX, "Block", &format!("bb={}", block.id.0))?;
        writeln!(out, "bb_{}_{}:", function.id.0.0, block.id.0)?;
        for (index, instruction) in block.instrs.iter().enumerate() {
            emit_tag(out, function.id.0.0, block.id.0 as u32, index as u32, opcode(instruction), &format!("{instruction}"))?;
            writeln!(out, "    {};", instruction_to_c(instruction, symbols, module, pool))?;
        }
        emit_tag(out, function.id.0.0, block.id.0 as u32, u32::MAX - 1, "Terminator", &format!("{}", block.terminator))?;
        emit_terminator(out, function, &block.terminator, pool)?;
    }
    out.push_str("}\n\n"); Ok(())
}

fn emit_tag(out: &mut String, function: u32, block: u32, instruction: u32, opcode: &str, payload: &str) -> Result<(), MirCEmitError> {
    let payload = payload.as_bytes().iter().map(|byte| format!("{byte:02x}")).collect::<String>();
    let tag = MirTag { format_version: ReconstructionIndex::FORMAT_VERSION, module_fingerprint: "mir-v1".into(), id: MirInstructionId { function, block, instruction }, opcode: opcode.into(), payload };
    writeln!(out, "    {}", emit_c_tag(&tag))?; Ok(())
}

fn opcode<V: CValueId>(i: &Instruction<V>) -> &'static str { match i { Instruction::Assign(..) => "Assign", Instruction::LoadField(..) => "LoadField", Instruction::StoreField(..) => "StoreField", Instruction::Call(..) => "Call", Instruction::CallIntrinsic(..) => "CallIntrinsic", Instruction::HeapAllocate(..) => "HeapAllocate", Instruction::StackAllocate(..) => "StackAllocate", Instruction::BinaryOp(..) => "BinaryOp", Instruction::UnaryOp(..) => "UnaryOp", Instruction::Borrow(..) => "Borrow", Instruction::BorrowMut(..) => "BorrowMut", Instruction::Deref(..) => "Deref", Instruction::Drop(..) => "Drop", Instruction::Phi(..) => "Phi" } }

fn instruction_to_c<V: CValueId>(i: &Instruction<V>, symbols: &SymbolTable, module: &MirModule<V>, pool: &StringPool) -> String { match i {
    Instruction::Assign(d, v) => format!("v_{} = {}", d.raw(), operand(v, pool)),
    Instruction::BinaryOp(d, op, l, r) => format!("v_{} = {} {} {}", d.raw(), operand(l, pool), binop(*op), operand(r, pool)),
    Instruction::UnaryOp(d, op, v) => format!("v_{} = {}{}", d.raw(), unop(*op), operand(v, pool)),
    Instruction::Call(d, f, a) => format!("v_{} = {}({})", d.raw(), match f { vinglish_mir::CallTarget::Direct(id) => call_name(*id, symbols, module), vinglish_mir::CallTarget::Foreign { c_symbol } => c_ident(c_symbol) }, a.iter().map(|value| operand(value, pool)).collect::<Vec<_>>().join(", ")),
    Instruction::CallIntrinsic(d, name, a) => format!("v_{} = {}({})", d.raw(), c_ident(name), a.iter().map(|value| operand(value, pool)).collect::<Vec<_>>().join(", ")),
    Instruction::Phi(d, values) => format!("v_{} = {}", d.raw(), values.first().map(|(v, _)| operand(v, pool)).unwrap_or_else(|| "0".into())),
    Instruction::LoadField(d, object, access) => format!("v_{} = *(long *)((unsigned char *)(uintptr_t){} + {})", d.raw(), operand(object, pool), access.byte_offset),
    Instruction::StoreField(object, access, value) => format!("*(long *)((unsigned char *)(uintptr_t)v_{} + {}) = {}", object.raw(), access.byte_offset, operand(value, pool)),
    Instruction::HeapAllocate(d, layout) => format!("v_{} = (long)(uintptr_t)calloc(1, {})", d.raw(), layout.size),
    Instruction::StackAllocate(d, layout) => format!("v_{} = (long)(uintptr_t)calloc(1, {})", d.raw(), layout.size),
    Instruction::Borrow(d, v) | Instruction::BorrowMut(d, v) | Instruction::Deref(d, v, _) => format!("v_{} = {}", d.raw(), operand(v, pool)),
    Instruction::Drop(..) => "(void)0".into(),
} }
fn emit_terminator<V: CValueId>(out: &mut String, f: &MirFunction<V>, t: &Terminator<V>, pool: &StringPool) -> Result<(), MirCEmitError> { match t { Terminator::Return(Some(v)) => writeln!(out, "    return {};", operand(v, pool))?, Terminator::Return(None) => writeln!(out, "    return 0;")?, Terminator::Jump(b) => writeln!(out, "    goto bb_{}_{};", f.id.0.0, b.0)?, Terminator::Branch(c, yes, no) => writeln!(out, "    if ({}) goto bb_{}_{}; else goto bb_{}_{};", operand(c, pool), f.id.0.0, yes.0, f.id.0.0, no.0)?, }; Ok(()) }
fn operand<V: CValueId>(v: &Operand<V>, pool: &StringPool) -> String { match v { Operand::Var(id) => format!("v_{}", id.raw()), Operand::Constant(value) => literal(value, pool) } }
fn literal(v: &Literal, pool: &StringPool) -> String { match v { Literal::Int(v) => v.to_string(), Literal::Float(v) => format!("{v}"), Literal::Bool(v) => (*v as i64).to_string(), Literal::Text(text) => format!("string_literal_{}", pool.entries[text]), Literal::Unit => "0".into() } }
fn binop(op: BinOp) -> &'static str { match op { BinOp::Add => "+", BinOp::Sub => "-", BinOp::Mul => "*", BinOp::Div => "/", BinOp::Mod => "%", BinOp::Eq => "==", BinOp::NotEq => "!=", BinOp::Lt | BinOp::IsBelow => "<", BinOp::Gt | BinOp::IsAbove | BinOp::Exceeds => ">", BinOp::LtEq => "<=", BinOp::GtEq => ">=", BinOp::And => "&&", BinOp::Or => "||" } }
fn unop(op: UnOp) -> &'static str { match op { UnOp::Neg => "-", UnOp::Not => "!", UnOp::Deref | UnOp::Borrow(_) => "" } }
fn call_name<V: CValueId>(id: FunctionId, symbols: &SymbolTable, module: &MirModule<V>) -> String { if module.functions.iter().any(|f| f.id == id) { format!("fn_{}", id.0.0) } else { symbols.get_func(id).map(|f| c_ident(&f.name)).unwrap_or_else(|| format!("fn_{}", id.0.0)) } }
fn c_ident(name: &str) -> String { name.chars().map(|c| if c.is_ascii_alphanumeric() || c == '_' { c } else { '_' }).collect() }

fn to_c_type(ty: &Type) -> &'static str { match ty { Type::Int | Type::Bool | Type::Unit => "int64_t", Type::Float => "double", Type::Text => "const char *", Type::Reference(_, _) | Type::Pointer(_) | Type::List(_) | Type::Dict(_, _) | Type::Optional(_) | Type::Result(_, _) | Type::Named(_, _) | Type::Function(_, _) | Type::Var(_) => "uintptr_t" } }
fn c_value_type<V: CValueId>(value: V, symbols: &SymbolTable) -> &'static str { symbols.get_var(VariableId(vinglish_hir::symbol::SymbolId(value.raw()))).map(|symbol| to_c_type(&symbol.ty)).unwrap_or("int64_t") }
fn c_zero<V: CValueId>(value: V, symbols: &SymbolTable) -> &'static str { if c_value_type(value, symbols) == "const char *" { "NULL" } else if c_value_type(value, symbols) == "double" { "0.0" } else { "0" } }

struct StringPool { entries: BTreeMap<String, usize> }
impl StringPool {
    fn collect<V: CValueId>(module: &MirModule<V>) -> Self {
        let mut entries = BTreeMap::new();
        for literal in module.functions.iter().flat_map(|function| function.blocks.iter()).flat_map(|block| block.instrs.iter()).flat_map(instruction_literals) {
            if let Literal::Text(text) = literal { let next = entries.len(); entries.entry(text.clone()).or_insert(next); }
        }
        Self { entries }
    }
}
fn instruction_literals<V: CValueId>(instruction: &Instruction<V>) -> Vec<&Literal> { let operands: Vec<&Operand<V>> = match instruction { Instruction::Assign(_, value) | Instruction::UnaryOp(_, _, value) | Instruction::Borrow(_, value) | Instruction::BorrowMut(_, value) | Instruction::Deref(_, value, _) => vec![value], Instruction::BinaryOp(_, _, left, right) => vec![left, right], Instruction::Call(_, _, values) | Instruction::CallIntrinsic(_, _, values) => values.iter().collect(), Instruction::LoadField(_, object, _) => vec![object], Instruction::StoreField(_, _, value) => vec![value], Instruction::Phi(_, values) => values.iter().map(|(value, _)| value).collect(), _ => vec![] }; operands.into_iter().filter_map(|operand| match operand { Operand::Constant(literal) => Some(literal), Operand::Var(_) => None }).collect() }
fn escape_c_string(text: &str) -> String { text.chars().flat_map(|character| match character { '\\' => "\\\\".chars().collect::<Vec<_>>(), '"' => "\\\"".chars().collect(), '\n' => "\\n".chars().collect(), '\r' => "\\r".chars().collect(), '\t' => "\\t".chars().collect(), character if character.is_control() => format!("\\x{:02x}", character as u32).chars().collect(), character => vec![character] }).collect() }

#[cfg(test)]
mod tests { use super::*; use vinglish_hir::symbol::{FunctionId, SymbolId}; use vinglish_mir::{BasicBlock, BlockId};
    #[test] fn metadata_preserves_instruction_ids() { let value = VariableId(SymbolId(1)); let module = MirModule { functions: vec![MirFunction { id: FunctionId(SymbolId(9)), is_foreign: false, name: "f".into(), params: vec![], locals: vec![value], blocks: vec![BasicBlock { id: BlockId(0), instrs: vec![Instruction::Assign(value, Operand::Constant(Literal::Int(7)))], terminator: Terminator::Return(Some(Operand::Var(value))) }] }] }; let c = emit_mir_c(&module, &SymbolTable::new()).unwrap(); let index = vinglish_decompile::reconstruct_mir(&c).unwrap(); assert!(index.records.contains_key(&MirInstructionId { function: 9, block: 0, instruction: 0 })); }
}
