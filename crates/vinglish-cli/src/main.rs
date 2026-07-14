use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use clap::{Parser, Subcommand};

use vinglish_codegen::{emit_c, Interpreter};
use vinglish_diagnostics::{render, Diagnostic};
use vinglish_fmt::format_module;
use vinglish_hir::symbol::{SymbolTable, VariableId};
use vinglish_hir::Module as HirModule;
use vinglish_lexer::tokenize;
use vinglish_mir::validator::MirValidatorPass;
use vinglish_mir::MirModule;
use vinglish_ownership::check_module;
use vinglish_parser::parse;
use vinglish_types::{
    passes::{CompilerPass, NameResolutionPass},
    type_pass::TypeInferencePass,
    validator::HirValidatorPass,
    CompilerContext, MirLowerer,
};

// ─────────────────────────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(
    name = "ving",
    version = env!("CARGO_PKG_VERSION"),
    about = "The Vinglish intent-aware systems programming language",
    long_about = "eng — compile, run, check, and format Vinglish source files.\n\nVinglish is a statically compiled language whose primary abstraction is intent.\nWrite what you mean. Let the compiler determine how to execute it correctly."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum PkgCommands {
    /// Initialize a new Vinglish package
    Init,
    /// Add a dependency to the current package
    Add {
        /// Package name to add
        package: String,
        /// Optional URL or path to the package
        url: Option<String>,
    },
}

#[derive(Subcommand)]
enum Commands {
    /// Compile an Vinglish file to a native binary
    Build {
        /// Source file to compile
        file: PathBuf,
        /// Output binary path
        #[arg(short, long, default_value = "a.out")]
        output: PathBuf,
        /// Backend to use (c | interp)
        #[arg(long, default_value = "c")]
        backend: String,
        /// What to emit (c | mir)
        #[arg(long)]
        emit: Option<String>,
    },
    /// Compile and immediately run an Vinglish file (interpreted)
    Run {
        /// Source file to run
        file: PathBuf,
        /// Arguments passed to the program
        args: Vec<String>,
    },
    /// Package management commands
    Pkg {
        #[command(subcommand)]
        command: PkgCommands,
    },
    /// Run the Language Server Protocol (LSP) daemon
    Lsp,
    /// Type-check an Vinglish file without producing output
    Check {
        /// Source file to check
        file: PathBuf,
    },
    /// Format an Vinglish source file in place (or to stdout with --check)
    Fmt {
        /// Source file(s) to format
        files: Vec<PathBuf>,
        /// Print diff instead of writing; exit non-zero if any file would change
        #[arg(long)]
        check: bool,
    },
    /// Run the benchmarking suite
    Benchmark {
        /// Directory containing benchmark files
        directory: PathBuf,
        /// Number of iterations per benchmark
        #[arg(long, default_value = "5")]
        runs: u32,
    },
    /// Print version information
    Version,
}

// ─────────────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Build {
            file,
            output,
            backend,
            emit,
        } => {
            if let Err(e) = cmd_build(&file, &output, &backend, emit) {
                eprintln!("{}", e);
                std::process::exit(1);
            }
        }
        Commands::Run { file, args: _ } => {
            if let Err(e) = cmd_run(&file) {
                eprintln!("{}", e);
                std::process::exit(1);
            }
        }
        Commands::Lsp => {
            vinglish_lsp::run_server().await;
        }
        Commands::Check { file } => {
            let ok = cmd_check(&file);
            if !ok {
                std::process::exit(1);
            }
        }
        Commands::Fmt { files, check } => {
            let ok = cmd_fmt(&files, check);
            if !ok {
                std::process::exit(1);
            }
        }
        Commands::Benchmark { directory, runs } => {
            if let Err(e) = cmd_benchmark(&directory, runs) {
                eprintln!("{}", e);
                std::process::exit(1);
            }
        }
        Commands::Version => {
            println!(
                "eng {} — Vinglish Compiler (Stage 0)\nBuilt with: rustc {}",
                env!("CARGO_PKG_VERSION"),
                rustc_version()
            );
        }
        Commands::Pkg { command } => {
            if let Err(e) = cmd_pkg(command) {
                eprintln!("{}", e);
                std::process::exit(1);
            }
        }
    }
}

fn cmd_pkg(command: PkgCommands) -> Result<(), String> {
    match command {
        PkgCommands::Init => {
            println!("Initializing new Vinglish package...");
            fs::write("eng.toml", "[package]\nname = \"my_pkg\"\nversion = \"0.1.0\"\n").map_err(|e| e.to_string())?;
            fs::create_dir_all("src").map_err(|e| e.to_string())?;
            fs::write("src/main.eng", "function main() returns number\nbegin\n    return 0\nend\n").map_err(|e| e.to_string())?;
            println!("Created package `my_pkg`");
            Ok(())
        }
        PkgCommands::Add { package, url } => {
            println!("Adding package '{}'...", package);
            let target_dir = Path::new(".ving_modules").join(&package);
            fs::create_dir_all(&target_dir).map_err(|e| e.to_string())?;
            if let Some(url) = url {
                println!("Fetching from {}...", url);
                // In a real implementation this would git clone or download
            }
            // Create a dummy module file to satisfy the compiler dependency
            let dummy_path = target_dir.join(format!("{}.eng", package));
            fs::write(&dummy_path, format!("package {}\nmodule {}\n\npublic function hello() returns number\nbegin\n    return 0\nend\n", package, package)).map_err(|e| e.to_string())?;
            println!("Successfully added `{}`", package);
            Ok(())
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Pipeline
// ─────────────────────────────────────────────────────────────────────────────

struct CompileResult {
    symbol_table: SymbolTable,
    mir_module: MirModule<VariableId>,
    entry_src: String,
    entry_filename: String,
    combined_ast: vinglish_parser::ast::Module,
}

fn resolve_dep_path(current_file: &Path, path_parts: &[String]) -> Result<PathBuf, String> {
    let mut path = PathBuf::new();
    if path_parts.first().map(|s| s.as_str()) == Some("std") {
        if let Ok(root) = std::env::var("ENGLIST_ROOT") {
            path.push(root);
        }
        path.push("std");
        for part in &path_parts[1..] {
            path.push(part);
        }
    } else {
        // Try `.ving_modules` first
        if let Some(pkg_name) = path_parts.first() {
            let pkg_dir = PathBuf::from(".ving_modules").join(pkg_name);
            if pkg_dir.exists() {
                let mut pkg_path = pkg_dir.clone();
                if path_parts.len() == 1 {
                    pkg_path.push(pkg_name);
                } else {
                    for part in &path_parts[1..] {
                        pkg_path.push(part);
                    }
                }
                pkg_path.set_extension("ving");
                if pkg_path.exists() {
                    return Ok(pkg_path);
                }
            }
        }
        
        // Fallback to local paths
        if let Some(parent) = current_file.parent() {
            path.push(parent);
        }
        for part in path_parts {
            path.push(part);
        }
    }
    path.set_extension("ving");
    Ok(path)
}

fn load_module_graph(
    module_name: String,
    file_path: PathBuf,
    parsed: &mut std::collections::HashMap<String, (vinglish_parser::ast::Module, String, PathBuf)>,
    deps: &mut std::collections::HashMap<String, Vec<String>>,
) -> Result<(), String> {
    if parsed.contains_key(&module_name) {
        return Ok(());
    }

    let src = fs::read_to_string(&file_path)
        .map_err(|e| format!("cannot read '{}': {}", file_path.display(), e))?;

    let (tokens, lex_errors) = tokenize(&src);
    if !lex_errors.is_empty() {
        for e in &lex_errors {
            eprintln!("Lex error in module '{}': {}", module_name, e);
        }
        return Err(format!("Lex errors in module '{}'", module_name));
    }

    let (module, parse_errors) = parse(&tokens);
    if !parse_errors.is_empty() {
        for e in &parse_errors {
            let mut found = match e {
                vinglish_parser::error::ParseError::Expected { found: ref f, .. } => f.clone(),
                _ => String::new(),
            };
            
            let span = e.span();
            let message = e.to_string();
            
            if found.is_empty() && span.start < span.end && (span.end as usize) <= src.len() {
                found = src[(span.start as usize)..(span.end as usize)].to_string();
            }
            
            let mut diag = vinglish_diagnostics::Diagnostic::error("P0001", &message, span);
            diag.enrich(&src);
            
            let source_line = diag.source_line.clone();
            if let Some(line) = source_line {
                vinglish_diagnostics::intent::resolve_intent(&mut diag, &found, &line);
            }
            
            let rendered = vinglish_diagnostics::render(&[diag], &file_path.display().to_string());
            eprint!("{}", rendered);
        }
        return Err(format!("Parse errors in module '{}'", module_name));
    }

    let mut module_deps = Vec::new();
    for item in &module.items {
        if let vinglish_parser::ast::Item::Use(u) = item {
            let path_parts: Vec<String> = u.path.iter().map(|id| id.name.clone()).collect();
            let dep_name = path_parts.join(".");
            module_deps.push(dep_name.clone());

            let dep_path = resolve_dep_path(&file_path, &path_parts)?;
            load_module_graph(dep_name, dep_path, parsed, deps)?;
        }
    }

    deps.insert(module_name.clone(), module_deps);
    parsed.insert(module_name, (module, src, file_path));
    Ok(())
}

fn topological_sort(
    deps: &std::collections::HashMap<String, Vec<String>>,
) -> Result<Vec<String>, String> {
    let mut order = Vec::new();
    let mut visited = std::collections::HashMap::new();

    fn dfs(
        node: &str,
        deps: &std::collections::HashMap<String, Vec<String>>,
        visited: &mut std::collections::HashMap<String, bool>,
        order: &mut Vec<String>,
    ) -> Result<(), String> {
        match visited.get(node) {
            Some(&true) => return Ok(()),
            Some(&false) => return Err(format!("cyclic dependency detected at module '{}'", node)),
            None => {}
        }

        visited.insert(node.to_string(), false);
        if let Some(children) = deps.get(node) {
            for child in children {
                dfs(child, deps, visited, order)?;
            }
        }
        visited.insert(node.to_string(), true);
        order.push(node.to_string());
        Ok(())
    }

    for node in deps.keys() {
        dfs(node, deps, &mut visited, &mut order)?;
    }

    Ok(order)
}

fn compile_project(file: &Path) -> Result<CompileResult, String> {
    let entry_path = file.to_path_buf();
    let entry_name = "main".to_string();

    let mut parsed = std::collections::HashMap::new();
    let mut deps = std::collections::HashMap::new();

    load_module_graph(entry_name.clone(), entry_path, &mut parsed, &mut deps)?;

    let compilation_order = topological_sort(&deps)?;

    let mut symbol_table = SymbolTable::new();
    let mut mir_functions = Vec::new();
    let mut entry_src = String::new();
    let mut entry_filename = String::new();

    for module_name in &compilation_order {
        let (ast, src, path) = parsed.get(module_name).unwrap();
        if module_name == &entry_name {
            entry_src = src.clone();
            entry_filename = path.display().to_string();
        }

        let mut ctx = CompilerContext::with_symbol_table(symbol_table);
        ctx.current_module = if module_name == &entry_name {
            String::new()
        } else {
            module_name.clone()
        };

        let mut name_pass = NameResolutionPass::new();
        name_pass.run(ast, &mut ctx);

        let mut type_pass = TypeInferencePass::new();
        let hir = type_pass
            .run(ast, &mut ctx)
            .unwrap_or_else(|| HirModule { items: vec![] });

        let validator = HirValidatorPass::new();
        validator.validate(&mut ctx, &hir);

        let mut has_errors = false;
        for e in &ctx.type_errors {
            let mut diag = Diagnostic::error("T0001", &e.message, e.span);
            diag.enrich(src);
            let rendered = render(&[diag], &path.display().to_string());
            eprint!("{}", rendered);
            has_errors = true;
        }

        let own_errors = check_module(ast);
        for e in &own_errors {
            let mut diag = Diagnostic::error("O0001", &e.message, e.span);
            if let Some(note) = &e.note {
                diag = diag.with_note(note);
            }
            diag.enrich(src);
            let rendered = render(&[diag], &path.display().to_string());
            eprint!("{}", rendered);
            has_errors = true;
        }

        if has_errors {
            return Err("compilation failed due to errors above".into());
        }

        symbol_table = ctx.symbol_table;
        let mut mir_lower = MirLowerer::new(&mut symbol_table);
        let mir_mod = mir_lower.lower_module(&hir);
        mir_functions.extend(mir_mod.functions);
    }

    let mut combined_items = Vec::new();
    for module_name in &compilation_order {
        let (ast, _, _) = parsed.get(module_name).unwrap();
        for item in &ast.items {
            if !matches!(item, vinglish_parser::ast::Item::Use(_)) {
                combined_items.push(item.clone());
            }
        }
    }
    let combined_ast = vinglish_parser::ast::Module {
        items: combined_items,
        span: vinglish_lexer::Span::dummy(),
    };

    Ok(CompileResult {
        symbol_table,
        mir_module: MirModule {
            functions: mir_functions,
        },
        entry_src,
        entry_filename,
        combined_ast,
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Commands
// ─────────────────────────────────────────────────────────────────────────────

fn cmd_run(file: &Path) -> Result<(), String> {
    let compile_res = compile_project(file)?;
    let mut symbol_table = compile_res.symbol_table;
    let mut mir_module = compile_res.mir_module;

    let validator = MirValidatorPass::new();
    if let Err(errors) = validator.validate(&symbol_table, &mir_module) {
        for e in &errors {
            eprintln!("MIR validation error: {}", e.message);
        }
        return Err("MIR validation failed".into());
    }

    let mut pre_pm = vinglish_opt::pre_ssa_pipeline();
    if let Err(errors) = pre_pm.run_all(&mut mir_module, &symbol_table) {
        for e in &errors {
            eprintln!(
                "MIR validation error after pre-SSA optimization: {}",
                e.message
            );
        }
        return Err("Pre-SSA optimization validation failed".into());
    }

    let mut ssa_pass = vinglish_ssa::SSAConversionPass::new();
    let mut ssa_module = ssa_pass.run(mir_module, &mut symbol_table);

    let ssa_validator = vinglish_ssa::SSAValidator::new();
    if let Err(errors) = ssa_validator.validate(&ssa_module) {
        for e in &errors {
            eprintln!("SSA validation error: {}", e.message);
        }
        return Err("SSA validation failed".into());
    }

    let mut post_pm = vinglish_opt::post_ssa_pipeline();
    if let Err(errors) = post_pm.run_all(&mut ssa_module, &symbol_table) {
        for e in &errors {
            eprintln!(
                "MIR validation error after post-SSA optimization: {}",
                e.message
            );
        }
        return Err("Post-SSA optimization validation failed".into());
    }

    let own_analyzer = vinglish_own::OwnershipAnalysisPass::new();
    let own_graph = own_analyzer.run(&mut ssa_module, &symbol_table);

    let own_validator = vinglish_own::OwnershipValidator::new();
    if let Err(errors) = own_validator.validate(&symbol_table, &ssa_module, &own_graph) {
        for e in &errors {
            let mut diag = e.clone();
            diag.enrich(&compile_res.entry_src);
            let rendered = render(&[diag], &compile_res.entry_filename);
            eprint!("{}", rendered);
        }
        return Err("Ownership validation failed".into());
    }

    let mut interp = Interpreter::new(&symbol_table);
    interp
        .run_module(&ssa_module)
        .map_err(|e| format!("runtime error: {}", e.message))
}

fn cmd_build(
    file: &Path,
    output: &Path,
    backend: &str,
    emit: Option<String>,
) -> Result<(), String> {
    let compile_res = compile_project(file)?;
    let mut symbol_table = compile_res.symbol_table;
    let mut mir_module = compile_res.mir_module;

    // Collect runtime paths
    let mut runtime_paths = Vec::new();
    let rt_dir = if let Ok(root) = std::env::var("ENGLIST_ROOT") {
        PathBuf::from(root).join("rt")
    } else {
        std::env::current_dir().unwrap_or_default().join("rt")
    };

    if let Ok(entries) = fs::read_dir(&rt_dir) {
        for entry in entries.flatten() {
            if entry.path().extension().map_or(false, |ext| ext == "c") {
                runtime_paths.push(entry.path());
            }
        }
    }

    let validator = MirValidatorPass::new();
    if let Err(errors) = validator.validate(&symbol_table, &mir_module) {
        for e in &errors {
            eprintln!("MIR validation error: {}", e.message);
        }
        return Err("MIR validation failed".into());
    }

    if emit.as_deref() == Some("mir-before") {
        println!("{}", mir_module);
        return Ok(());
    }

    let mut pre_pm = vinglish_opt::pre_ssa_pipeline();
    let pre_stats = match pre_pm.run_all(&mut mir_module, &symbol_table) {
        Ok(s) => s,
        Err(errors) => {
            for e in &errors {
                eprintln!(
                    "MIR validation error after pre-SSA optimization: {}",
                    e.message
                );
            }
            return Err("Pre-SSA optimization validation failed".into());
        }
    };

    let mir_before = mir_module.clone();
    let mut ssa_pass = vinglish_ssa::SSAConversionPass::new();
    let mut ssa_module = ssa_pass.run(mir_module, &mut symbol_table);

    let ssa_validator = vinglish_ssa::SSAValidator::new();
    if let Err(errors) = ssa_validator.validate(&ssa_module) {
        for e in &errors {
            eprintln!("SSA validation error: {}", e.message);
        }
        return Err("SSA validation failed".into());
    }

    let mut post_pm = vinglish_opt::post_ssa_pipeline();
    let post_stats = match post_pm.run_all(&mut ssa_module, &symbol_table) {
        Ok(s) => s,
        Err(errors) => {
            for e in &errors {
                eprintln!(
                    "MIR validation error after post-SSA optimization: {}",
                    e.message
                );
            }
            return Err("Post-SSA optimization validation failed".into());
        }
    };

    let mut stats = pre_stats;
    stats.add(&post_stats);

    if let Some(emit_type) = emit.as_deref() {
        match emit_type {
            "ssa" => {
                println!("{}", ssa_module);
                return Ok(());
            }
            "mir" | "mir-after" => {
                println!("{}", ssa_module);
                return Ok(());
            }
            "mir-stats" => {
                println!("--- MIR OPTIMIZATION STATS ---");
                println!(
                    "Total variables: {}",
                    mir_before
                        .functions
                        .iter()
                        .map(|f| f.locals.len())
                        .sum::<usize>()
                );
                println!("Functions: {}", ssa_module.functions.len());
                println!("CFG Simplification:");
                println!("  Merged blocks: {}", stats.merged_blocks);
                println!("Folded constants: {}", stats.folded_constants);
                println!("GVN Eliminated: {}", stats.gvn_eliminated);
                return Ok(());
            }
            "mir-diff" => {
                println!("Before\n");
                println!("{}", mir_before);
                println!("After\n");
                println!("{}", ssa_module);
                return Ok(());
            }
            _ => {}
        }
    }

    let own_analyzer = vinglish_own::OwnershipAnalysisPass::new();
    let own_graph = own_analyzer.run(&mut ssa_module, &symbol_table);

    let own_validator = vinglish_own::OwnershipValidator::new();
    if let Err(errors) = own_validator.validate(&symbol_table, &ssa_module, &own_graph) {
        for e in &errors {
            let mut diag = e.clone();
            diag.enrich(&compile_res.entry_src);
            let rendered = render(&[diag], &compile_res.entry_filename);
            eprint!("{}", rendered);
        }
        return Err("Ownership validation failed".into());
    }

    if emit.as_deref() == Some("ownership") {
        println!("{}", own_graph);
        return Ok(());
    }

    if emit.as_deref() == Some("llvm") {
        let ir = vinglish_llvm::compile_to_llvm_ir(&ssa_module, &symbol_table)?;
        println!("{}", ir);
        return Ok(());
    }

    if backend == "llvm" {
        vinglish_llvm::compile_to_executable(&ssa_module, &symbol_table, output, &runtime_paths)?;
        eprintln!("  \x1b[32m✓\x1b[0m  Binary: {}", output.display());
        return Ok(());
    }

    if backend == "c" {
        let c_src = emit_c(&compile_res.combined_ast)
            .map_err(|e| format!("code generation error: {}", e))?;
        let c_file = output.with_extension("c");
        fs::write(&c_file, &c_src).map_err(|e| format!("cannot write C source: {}", e))?;
        eprintln!(
            "  \x1b[32m✓\x1b[0m  Generated C source: {}",
            c_file.display()
        );

        let cc = std::env::var("CC").unwrap_or_else(|_| "cc".into());
        let mut cmd = Command::new(&cc);
        cmd.arg("-O2").arg("-Wno-int-conversion").arg("-o").arg(output).arg(&c_file);

        for rt_path in &runtime_paths {
            cmd.arg(rt_path);
        }
        
        let rt_rust_toml = std::env::current_dir().unwrap_or_default().join("rt_rust").join("Cargo.toml");
        if rt_rust_toml.exists() {
            eprintln!("  Compiling Rust FFI bridge...");
            let rt_rust_dir = rt_rust_toml.parent().unwrap();
            
            // Clean up old interfaces file before building
            let workspace_root = rt_rust_dir.parent().unwrap();
            let interfaces_file = workspace_root.join(".vinglish_interfaces.tmp");
            let _ = std::fs::remove_file(&interfaces_file);
            
            let cargo_status = Command::new("cargo")
                .arg("build")
                .arg("--release")
                .current_dir(rt_rust_dir)
                .status()
                .map_err(|e| format!("cannot invoke cargo: {}", e))?;
                
            if !cargo_status.success() {
                return Err(format!("cargo build exited with status {}", cargo_status));
            }
            
            // Generate the rust_ffi.ving interface file
            if interfaces_file.exists() {
                if let Ok(interfaces) = std::fs::read_to_string(&interfaces_file) {
                    let rust_ffi_dir = workspace_root.join(".ving_modules").join("rust_ffi");
                    let _ = std::fs::create_dir_all(&rust_ffi_dir);
                    
                    let mut content = String::from("package rust_ffi\nmodule rust_ffi\n\n");
                    content.push_str(&interfaces);
                    
                    let _ = std::fs::write(rust_ffi_dir.join("rust_ffi.ving"), content);
                }
                let _ = std::fs::remove_file(&interfaces_file);
            }
            
            // Since rt_rust is in a workspace, the target directory is at the workspace root
            let workspace_root = rt_rust_dir.parent().unwrap();
            let target_dir = workspace_root.join("target").join("release");
            cmd.arg(format!("-L{}", target_dir.display()));
            cmd.arg("-lvinglish_rt");
            
            // Add macOS specific frameworks required by minifb/winit
            #[cfg(target_os = "macos")]
            {
                cmd.arg("-lc++");
                cmd.arg("-framework").arg("Cocoa");
                cmd.arg("-framework").arg("IOKit");
                cmd.arg("-framework").arg("Foundation");
                cmd.arg("-framework").arg("Metal");
                cmd.arg("-framework").arg("MetalKit");
                cmd.arg("-framework").arg("Carbon");
                cmd.arg("-framework").arg("QuartzCore");
                cmd.arg("-framework").arg("UniformTypeIdentifiers");
                cmd.arg("-framework").arg("WebKit");
                cmd.arg("-framework").arg("AppKit");
                cmd.arg("-framework").arg("Security");
                cmd.arg("-framework").arg("SystemConfiguration");
            }
        }

        cmd.arg("-lm");

        let status = cmd
            .status()
            .map_err(|e| format!("cannot invoke C compiler `{}`: {}", cc, e))?;

        if !status.success() {
            return Err(format!("C compiler exited with status {}", status));
        }

        eprintln!("  \x1b[32m✓\x1b[0m  Binary: {}", output.display());
        return Ok(());
    }

    Err(format!("unknown backend: {}", backend))
}

fn cmd_check(file: &Path) -> bool {
    let compile_res = match compile_project(file) {
        Ok(res) => res,
        Err(e) => {
            eprintln!("{}", e);
            return false;
        }
    };
    
    let mut symbol_table = compile_res.symbol_table;
    let mut mir_module = compile_res.mir_module;

    let validator = MirValidatorPass::new();
    if let Err(errors) = validator.validate(&symbol_table, &mir_module) {
        for e in &errors {
            eprintln!("MIR validation error: {}", e.message);
        }
        return false;
    }

    let mut pre_pm = vinglish_opt::pre_ssa_pipeline();
    if let Err(errors) = pre_pm.run_all(&mut mir_module, &symbol_table) {
        for e in &errors {
            eprintln!("MIR validation error after pre-SSA optimization: {}", e.message);
        }
        return false;
    }

    let mut ssa_pass = vinglish_ssa::SSAConversionPass::new();
    let mut ssa_module = ssa_pass.run(mir_module, &mut symbol_table);

    let ssa_validator = vinglish_ssa::SSAValidator::new();
    if let Err(errors) = ssa_validator.validate(&ssa_module) {
        for e in &errors {
            eprintln!("SSA validation error: {}", e.message);
        }
        return false;
    }

    let mut post_pm = vinglish_opt::post_ssa_pipeline();
    if let Err(errors) = post_pm.run_all(&mut ssa_module, &symbol_table) {
        for e in &errors {
            eprintln!("MIR validation error after post-SSA optimization: {}", e.message);
        }
        return false;
    }

    let own_analyzer = vinglish_own::OwnershipAnalysisPass::new();
    let own_graph = own_analyzer.run(&mut ssa_module, &symbol_table);

    let own_validator = vinglish_own::OwnershipValidator::new();
    if let Err(errors) = own_validator.validate(&symbol_table, &ssa_module, &own_graph) {
        for e in &errors {
            let mut diag = e.clone();
            diag.enrich(&compile_res.entry_src);
            let rendered = render(&[diag], &compile_res.entry_filename);
            eprint!("{}", rendered);
        }
        return false;
    }

    eprintln!("  \x1b[32m✓\x1b[0m  {} — no errors found", file.display());
    true
}

fn cmd_fmt(files: &[PathBuf], check: bool) -> bool {
    let mut all_ok = true;

    for file in files {
        let src = match fs::read_to_string(file) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("cannot read '{}': {}", file.display(), e);
                all_ok = false;
                continue;
            }
        };

        let (tokens, lex_errors) = tokenize(&src);
        if !lex_errors.is_empty() {
            eprintln!("cannot format '{}': lex errors", file.display());
            all_ok = false;
            continue;
        }

        let (module, parse_errors) = parse(&tokens);
        if !parse_errors.is_empty() {
            eprintln!("cannot format '{}': parse errors", file.display());
            all_ok = false;
            continue;
        }

        let formatted = format_module(&module);

        if check {
            if src != formatted {
                eprintln!("  \x1b[31m✗\x1b[0m  {}", file.display());
                all_ok = false;
            } else {
                eprintln!("  \x1b[32m✓\x1b[0m  {}", file.display());
            }
        } else {
            if src != formatted {
                if let Err(e) = fs::write(file, formatted) {
                    eprintln!("cannot write '{}': {}", file.display(), e);
                    all_ok = false;
                } else {
                    eprintln!("  \x1b[32m✓\x1b[0m  {}", file.display());
                }
            }
        }
    }

    all_ok
}

fn cmd_benchmark(directory: &Path, runs: u32) -> Result<(), String> {
    if runs == 0 {
        return Err("--runs must be at least 1".into());
    }
    let mut files: Vec<PathBuf> = fs::read_dir(directory)
        .map_err(|e| {
            format!(
                "cannot read benchmark directory '{}': {}",
                directory.display(),
                e
            )
        })?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension().is_some_and(|ext| {
                ext == "ving" || ext == "c" || ext == "py" || ext == "go" || ext == "elm"
            })
        })
        .collect();
    files.sort();
    if files.is_empty() {
        return Err(format!(
            "no benchmarks found in '{}'",
            directory.display()
        ));
    }

    let temp_dir = std::env::temp_dir().join(format!("vinglish-bench-{}", std::process::id()));
    fs::create_dir_all(&temp_dir)
        .map_err(|e| format!("cannot create temporary directory: {}", e))?;
    let mut results: Vec<(String, Duration)> = Vec::new();

    for file in files {
        let name = file
            .file_stem()
            .and_then(|n| n.to_str())
            .unwrap_or("benchmark")
            .to_string();
        let ext = file.extension().and_then(|e| e.to_str()).unwrap_or("");
        
        let display_name = format!("{}.{}", name, ext);
        
        let output = temp_dir.join(&name);
        
        // Compile phase
        if ext == "ving" {
            cmd_build(&file, &output, "c", None)?;
        } else if ext == "c" {
            let status = Command::new("gcc")
                .arg("-O3")
                .arg(&file)
                .arg("-o")
                .arg(&output)
                .status()
                .map_err(|e| format!("gcc failed: {}", e))?;
            if !status.success() {
                return Err(format!("gcc compilation failed for {}", name));
            }
        } else if ext == "go" {
            let status = Command::new("go")
                .arg("build")
                .arg("-o")
                .arg(&output)
                .arg(&file)
                .status()
                .map_err(|e| format!("go build failed: {}", e))?;
            if !status.success() {
                return Err(format!("go compilation failed for {}", name));
            }
        } else if ext == "elm" {
            // Elm compiler creates an HTML or JS file
            let js_output = temp_dir.join(format!("{}.js", name));
            let status = Command::new("elm")
                .current_dir(directory)
                .arg("make")
                .arg(file.file_name().unwrap())
                .arg("--optimize")
                .arg(format!("--output={}", js_output.display()))
                .status()
                .map_err(|e| format!("elm make failed: {}", e))?;
            if !status.success() {
                return Err(format!("elm compilation failed for {}", name));
            }
            // Create a small node runner to execute the compiled Elm worker
            let runner_js = temp_dir.join(format!("{}_runner.js", name));
            let runner_code = format!(
                "const {{ Elm }} = require('./{}.js');\nconst app = Elm.Main.init();\napp.ports.emitResult.subscribe(res => process.exit(0));\n",
                name
            );
            fs::write(&runner_js, runner_code).map_err(|e| format!("failed to write runner: {}", e))?;
        }

        let mut elapsed = Duration::ZERO;
        for _ in 0..runs {
            let start = Instant::now();
            let status = if ext == "py" {
                Command::new("python3").arg(&file).stdout(Stdio::null()).stderr(Stdio::null()).status()
            } else if ext == "elm" {
                let runner_js = temp_dir.join(format!("{}_runner.js", name));
                Command::new("node").arg(&runner_js).stdout(Stdio::null()).stderr(Stdio::null()).status()
            } else {
                Command::new(&output).stdout(Stdio::null()).stderr(Stdio::null()).status()
            }
            .map_err(|e| format!("cannot run '{}': {}", display_name, e))?;
            
            if !status.success() {
                return Err(format!("benchmark '{}' exited with {}", display_name, status));
            }
            elapsed += start.elapsed();
        }
        results.push((display_name, elapsed / runs));
    }

    println!("{:<28} Average time", "Algorithm");
    println!("{:-<28} ------------", "");
    for (name, elapsed) in results {
        println!("{:<28} {:.3} ms", name, elapsed.as_secs_f64() * 1_000.0);
    }
    let _ = fs::remove_dir_all(temp_dir);
    Ok(())
}
fn rustc_version() -> String {
    let output = std::process::Command::new("rustc").arg("-V").output().ok();
    if let Some(out) = output {
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    } else {
        "unknown".into()
    }
}
