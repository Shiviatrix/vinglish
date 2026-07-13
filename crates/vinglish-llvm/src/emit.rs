use std::path::Path;

use inkwell::module::Module;
use inkwell::targets::{
    CodeModel, FileType, InitializationConfig, RelocMode, Target, TargetMachine,
};
use inkwell::OptimizationLevel;

/// Emit LLVM IR as a string.
pub fn emit_ir(module: &Module) -> String {
    module.print_to_string().to_string()
}

/// Write LLVM IR to a file.
pub fn emit_ir_to_file(module: &Module, path: &Path) -> Result<(), String> {
    module
        .print_to_file(path)
        .map_err(|e| format!("Failed to write LLVM IR: {}", e.to_string()))
}

/// Write a native object file.
pub fn emit_object_file(module: &Module, path: &Path) -> Result<(), String> {
    let target_machine = create_target_machine()?;

    target_machine
        .write_to_file(module, FileType::Object, path)
        .map_err(|e| format!("Failed to write object file: {}", e.to_string()))
}

/// Compile to a native executable by emitting an object file and linking.
pub fn emit_executable(
    module: &Module,
    output: &Path,
    runtime_paths: &[std::path::PathBuf],
) -> Result<(), String> {
    let obj_path = output.with_extension("o");

    emit_object_file(module, &obj_path)?;

    // Link with system C compiler
    let cc = std::env::var("CC").unwrap_or_else(|_| "cc".into());
    let mut cmd = std::process::Command::new(&cc);
    cmd.arg("-o").arg(output).arg(&obj_path);

    for rt_path in runtime_paths {
        cmd.arg(rt_path);
    }

    cmd.arg("-lc");

    let status = cmd
        .status()
        .map_err(|e| format!("Cannot invoke linker `{}`: {}", cc, e))?;

    if !status.success() {
        return Err(format!("Linker exited with status {}", status));
    }

    // Clean up object file
    let _ = std::fs::remove_file(&obj_path);

    Ok(())
}

/// Initialize LLVM targets and create a TargetMachine for the host.
pub fn initialize_targets() {
    Target::initialize_native(&InitializationConfig::default())
        .expect("Failed to initialize native LLVM target");
}

fn create_target_machine() -> Result<TargetMachine, String> {
    let triple = TargetMachine::get_default_triple();
    let target = Target::from_triple(&triple)
        .map_err(|e| format!("Failed to get target from triple: {}", e.to_string()))?;

    let cpu = TargetMachine::get_host_cpu_name();
    let features = TargetMachine::get_host_cpu_features();

    target
        .create_target_machine(
            &triple,
            cpu.to_str().unwrap_or("generic"),
            features.to_str().unwrap_or(""),
            OptimizationLevel::Default,
            RelocMode::Default,
            CodeModel::Default,
        )
        .ok_or_else(|| "Failed to create target machine".to_string())
}
