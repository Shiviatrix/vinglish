pub mod builtins;
pub mod codegen;
pub mod emit;
pub mod types;

use std::path::Path;

use eng_hir::symbol::{SsaValueId, SymbolTable};
use eng_mir::MirModule;
use inkwell::context::Context;

/// Compile a MIR module to LLVM IR text.
pub fn compile_to_llvm_ir(
    module: &MirModule<SsaValueId>,
    symbol_table: &SymbolTable,
) -> Result<String, String> {
    let context = Context::create();
    let mut codegen = codegen::LLVMCodeGen::new(&context, "englist_module", symbol_table);
    codegen.compile_module(module)?;
    Ok(codegen.get_ir_string())
}

/// Compile a MIR module to a native object file.
pub fn compile_to_object(
    module: &MirModule<SsaValueId>,
    symbol_table: &SymbolTable,
    path: &Path,
) -> Result<(), String> {
    emit::initialize_targets();
    let context = Context::create();
    let mut codegen = codegen::LLVMCodeGen::new(&context, "englist_module", symbol_table);
    codegen.compile_module(module)?;
    emit::emit_object_file(&codegen.module, path)
}

/// Compile a MIR module to a native executable.
pub fn compile_to_executable(
    module: &MirModule<SsaValueId>,
    symbol_table: &SymbolTable,
    output: &Path,
    runtime_paths: &[std::path::PathBuf],
) -> Result<(), String> {
    emit::initialize_targets();
    let context = Context::create();
    let mut codegen = codegen::LLVMCodeGen::new(&context, "englist_module", symbol_table);
    codegen.compile_module(module)?;
    emit::emit_executable(&codegen.module, output, runtime_paths)
}
