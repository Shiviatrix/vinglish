use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use clap::{Parser, Subcommand};

use vinglish_codegen::{Interpreter, emit_mir_c};
use vinglish_diagnostics::{Diagnostic, render};
use vinglish_fmt::format_module;
use vinglish_hir::Module as HirModule;
use vinglish_hir::symbol::{SymbolTable, VariableId};
use vinglish_ir_export::{ExportBuilder, to_json};
use vinglish_lexer::tokenize;
use vinglish_mir::MirModule;
use vinglish_mir::validator::MirValidatorPass;
use vinglish_ownership::check_module;
use vinglish_parser::parse;
use vinglish_types::{
    CompilerContext, MirLowerer,
    passes::{CompilerPass, NameResolutionPass, HealingMode},
    type_pass::TypeInferencePass,
    validator::HirValidatorPass,
};

// ─────────────────────────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(
    name = "ving",
    version = env!("CARGO_PKG_VERSION"),
    about = "The Vinglish intent-aware systems programming language",
    long_about = "vng — compile, run, check, and format Vinglish source files.\n\nVinglish is a statically compiled language whose primary abstraction is intent.\nWrite what you mean. Let the compiler determine how to execute it correctly."
)]
struct Cli {
    /// Export the stable semantic interchange document for a source file.
    #[arg(long, value_name = "FILE")]
    emit_ir: Option<PathBuf>,
    #[command(subcommand)]
    command: Option<Commands>,
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
        /// Automatically apply deterministic type healing in memory (output will not match source)
        #[arg(long)]
        heal: bool,
        /// Disable all type healing suggestions (useful for strict CI pipelines)
        #[arg(long)]
        deny_heal: bool,
    },
    /// Compile and immediately run an Vinglish file (interpreted)
    Run {
        /// Source file to run
        file: PathBuf,
        /// Arguments passed to the program
        args: Vec<String>,
        /// Optional path to a dynamic library containing FFI bindings to load
        #[arg(long)]
        lib: Option<PathBuf>,
    },
    /// Debug a Vinglish file in an interactive REPL
    Debug {
        /// Source file to debug
        file: PathBuf,
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
        /// Automatically apply deterministic type healing in memory
        #[arg(long)]
        heal: bool,
        /// Disable all type healing suggestions
        #[arg(long)]
        deny_heal: bool,
    },
    /// Automatically fix type errors in source files
    Fix {
        /// Source file(s) to fix
        #[arg(default_value = ".")]
        file: PathBuf,
        /// Allow fixing even if the git working tree is dirty
        #[arg(long)]
        allow_dirty: bool,
        /// Automatically accept all fixes without prompting
        #[arg(short, long)]
        yes: bool,
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
    if let Some(file) = cli.emit_ir {
        if let Err(error) = cmd_emit_ir(&file) {
            eprintln!("{}", error);
            std::process::exit(1);
        }
        return;
    }

    match cli.command {
        None => {
            eprintln!("usage: vng --emit-ir <FILE> | vng <COMMAND>");
            std::process::exit(2);
        }
        Some(Commands::Build {
            file,
            output,
            backend,
            emit,
            heal,
            deny_heal,
        }) => {
            let mode = get_healing_mode(heal, deny_heal);
            if let Err(e) = cmd_build(&file, &output, &backend, emit, mode) {
                eprintln!("{}", e);
                std::process::exit(1);
            }
        }
        Some(Commands::Run { file, args: _, lib }) => {
            if let Err(e) = cmd_run(&file, &lib) {
                eprintln!("{}", e);
                std::process::exit(1);
            }
        }
        Some(Commands::Debug { file }) => {
            if let Err(e) = cmd_debug(&file) {
                eprintln!("{}", e);
                std::process::exit(1);
            }
        }
        Some(Commands::Lsp) => {
            vinglish_lsp::run_server().await;
        }
        Some(Commands::Check { file, heal, deny_heal }) => {
            let mode = get_healing_mode(heal, deny_heal);
            let ok = cmd_check(&file, mode);
            if !ok {
                std::process::exit(1);
            }
        }
        Some(Commands::Fix { file, allow_dirty, yes }) => {
            if let Err(e) = cmd_fix(&file, allow_dirty, yes) {
                eprintln!("{}", e);
                std::process::exit(1);
            }
        }
        Some(Commands::Fmt { files, check }) => {
            let ok = cmd_fmt(&files, check);
            if !ok {
                std::process::exit(1);
            }
        }
        Some(Commands::Benchmark { directory, runs }) => {
            if let Err(e) = cmd_benchmark(&directory, runs) {
                eprintln!("{}", e);
                std::process::exit(1);
            }
        }
        Some(Commands::Version) => {
            println!(
                "vng {} — Vinglish Compiler (Stage 0)\nBuilt with: rustc {}",
                env!("CARGO_PKG_VERSION"),
                rustc_version()
            );
        }
        Some(Commands::Pkg { command }) => {
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
            vinglish_pkg::cmd_init()
        }
        PkgCommands::Add { package, url } => {
            vinglish_pkg::cmd_add(&package, url.as_deref())
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Pipeline
// ─────────────────────────────────────────────────────────────────────────────

struct CompileResult {
    symbol_table: SymbolTable,
    hir_modules: Vec<(String, HirModule)>,
    mir_module: MirModule<VariableId>,
    entry_src: String,
    entry_filename: String,
}

fn resolve_dep_path(current_file: &Path, path_parts: &[String]) -> Result<PathBuf, String> {
    let mut path = PathBuf::new();
    if path_parts.first().map(|s| s.as_str()) == Some("std") {
        if let Ok(root) = std::env::var("VINGLISH_ROOT") {
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
                if pkg_dir.join("src").exists() {
                    pkg_path.push("src");
                }
                
                if path_parts.len() == 1 {
                    let maybe_main = pkg_path.join("main").with_extension("ving");
                    if maybe_main.exists() {
                        return Ok(maybe_main);
                    }
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
                
                // Fallback for single file dummy module
                let mut fallback = pkg_dir.clone();
                if path_parts.len() == 1 {
                    fallback.push(pkg_name);
                } else {
                    for part in &path_parts[1..] {
                        fallback.push(part);
                    }
                }
                fallback.set_extension("ving");
                if fallback.exists() {
                    return Ok(fallback);
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
                vinglish_parser::error::ParseError::Expected { found: f, .. } => f.clone(),
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

fn compile_project(
    file: &Path,
    mode: HealingMode,
    mut collected_fixes: Option<&mut Vec<(PathBuf, String, vinglish_types::healer::HealingWarning)>>,
) -> Result<CompileResult, String> {
    let entry_path = file.to_path_buf();
    let entry_name = "main".to_string();

    if std::path::Path::new("ving.toml").exists() {
        if let Err(e) = vinglish_pkg::fetch_dependencies() {
            println!("Warning: Failed to fetch dependencies: {}", e);
        }
    }

    let mut parsed = std::collections::HashMap::new();
    let mut deps = std::collections::HashMap::new();

    load_module_graph(entry_name.clone(), entry_path, &mut parsed, &mut deps)?;

    let compilation_order = topological_sort(&deps)?;

    let mut symbol_table = SymbolTable::new();
    let mut mir_functions = Vec::new();
    let mut hir_modules = Vec::new();
    let mut entry_src = String::new();
    let mut entry_filename = String::new();

    for module_name in &compilation_order {
        let (parsed_ast, src, path) = parsed.get(module_name).unwrap();
        // Healing is an in-memory compilation transformation; parsed module
        // cache remains immutable for diagnostics and dependency loading.
        let mut ast = parsed_ast.clone();
        if module_name == &entry_name {
            entry_src = src.clone();
            entry_filename = path.display().to_string();
        }

        let mut ctx = CompilerContext::with_symbol_table(symbol_table);
        ctx.healing_mode = mode;
        ctx.current_module = if module_name == &entry_name {
            String::new()
        } else {
            module_name.clone()
        };

        let mut name_pass = NameResolutionPass::new();
        name_pass.run(&ast, &mut ctx);

        let mut type_pass = TypeInferencePass::new();
        let hir = type_pass.run_with_healing(&mut ast, &mut ctx);

        let validator = HirValidatorPass::new();
        validator.validate(&mut ctx, &hir);

        let mut has_errors = false;
        let is_fixing = collected_fixes.is_some();
        
        for e in &ctx.type_errors {
            let mut diag = Diagnostic::error("T0001", e.message(), e.span());
            
            if mode == HealingMode::SuggestOnly && !is_fixing {
                if let Some(warning) = ctx.healing_warnings.iter().find(|w| w.span == e.span()) {
                    diag.add_help(format!("auto-fixable — wrap in `{}` (run `vng fix` to apply)", vinglish_fmt::format_expr(&warning.replacement)));
                }
            }
            
            diag.enrich(src);
            if !is_fixing {
                let rendered = render(&[diag], &path.display().to_string());
                eprint!("{}", rendered);
            }
            has_errors = true;
        }
        
        for warning in &ctx.healing_warnings {
            let mut diag = Diagnostic::warning(
                "T1001",
                format!(
                    "Automatically healed type mismatch using {:?}",
                    warning.rule
                ),
                warning.span,
            );
            if mode == HealingMode::ApplyInMemory {
                diag.add_note("⚠ 1 TYPE ERROR WAS AUTO-HEALED IN MEMORY — output does not match source verbatim. Run `vng fix` to apply this change to source, or omit --heal to disable.");
            }
            diag.enrich(src);
            if !is_fixing && mode == HealingMode::ApplyInMemory {
                eprint!("{}", render(&[diag], &path.display().to_string()));
            }
        }
        
        if let Some(ref mut fixes) = collected_fixes {
            for warning in ctx.healing_warnings {
                fixes.push((path.clone(), src.clone(), warning));
            }
        }

        let own_errors = check_module(&ast);
        for e in &own_errors {
            let mut diag = Diagnostic::error("O0001", &e.message, e.span);
            if let Some(note) = &e.note {
                diag.add_note(note.clone());
            }
            diag.enrich(src);
            if !is_fixing {
                let rendered = render(&[diag], &path.display().to_string());
                eprint!("{}", rendered);
            }
            has_errors = true;
        }

        if has_errors {
            return Err("compilation failed due to errors above".into());
        }

        symbol_table = ctx.symbol_table;
        hir_modules.push((module_name.clone(), hir.clone()));
        let mut mir_lower = MirLowerer::new(&mut symbol_table);
        let mir_mod = mir_lower.lower_module(&hir);
        mir_functions.extend(mir_mod.functions);
    }

    Ok(CompileResult {
        symbol_table,
        hir_modules,
        mir_module: MirModule {
            functions: mir_functions,
        },
        entry_src,
        entry_filename,
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Commands
// ─────────────────────────────────────────────────────────────────────────────

fn cmd_emit_ir(file: &Path) -> Result<(), String> {
    let compilation = compile_project(file, HealingMode::Deny, None)?;
    let document = ExportBuilder::new(&compilation.symbol_table).document(
        compilation
            .hir_modules
            .iter()
            .map(|(name, module)| (name.clone(), module)),
    );
    let json = to_json(&document).map_err(|error| format!("cannot serialize export: {error}"))?;
    println!("{json}");
    Ok(())
}

fn cmd_run(file: &Path, lib: &Option<PathBuf>) -> Result<(), String> {
    let compile_res = compile_project(file, HealingMode::Deny, None)?;
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
    if let Some(lib_path) = lib {
        interp.load_dynamic_library(lib_path).map_err(|e| format!("Failed to load dynamic library: {}", e))?;
    }
    interp
        .run_module(&ssa_module)
        .map_err(|e| format!("runtime error: {}", e.message))
}

use std::io::{self, Write};
use vinglish_codegen::interp::DebuggerHook;
use vinglish_mir::{MirFunction, BasicBlock};
use vinglish_hir::symbol::SsaValueId;
use vinglish_codegen::interp::Value;
use std::collections::HashMap;

struct ReplDebugger {
    source: String,
    stepping: bool,
}

impl DebuggerHook for ReplDebugger {
    fn on_instruction(
        &mut self,
        _func: &MirFunction<SsaValueId>,
        block: &BasicBlock<SsaValueId>,
        instr_idx: usize,
        locals: &HashMap<SsaValueId, Value>,
    ) -> Result<(), vinglish_codegen::interp::InterpError> {
        if !self.stepping {
            return Ok(());
        }

        let span = &block.spans[instr_idx];
        let start = span.start as usize;
        let end = span.end as usize;

        // Naively extract the line containing the span
        let mut line_start = start;
        while line_start > 0 && self.source.as_bytes()[line_start - 1] != b'\n' {
            line_start -= 1;
        }
        let mut line_end = end;
        while line_end < self.source.len() && self.source.as_bytes()[line_end] != b'\n' {
            line_end += 1;
        }

        let line_text = &self.source[line_start..line_end];
        let line_num = self.source[..start].chars().filter(|&c| c == '\n').count() + 1;

        println!("=> [Line {}]: {}", line_num, line_text.trim());
        println!("   [MIR] {:?}", block.instrs[instr_idx]);

        loop {
            print!("(vdb) ");
            io::stdout().flush().unwrap();
            let mut input = String::new();
            io::stdin().read_line(&mut input).unwrap();
            let input = input.trim();

            if input.is_empty() {
                continue;
            }

            let mut parts = input.split_whitespace();
            let cmd = parts.next().unwrap();

            match cmd {
                "n" | "next" | "s" | "step" => {
                    self.stepping = true;
                    break;
                }
                "c" | "continue" => {
                    self.stepping = false;
                    break;
                }
                "p" | "print" => {
                    if let Some(var_str) = parts.next() {
                        if var_str.starts_with("ssa_") {
                            if let Ok(id_num) = var_str[4..].parse::<u32>() {
                                let id = SsaValueId(id_num);
                                if let Some(val) = locals.get(&id) {
                                    println!("{} = {}", var_str, val.to_display());
                                } else {
                                    println!("Variable not found in current locals");
                                }
                            } else {
                                println!("Invalid SSA ID format (expected ssa_<num>)");
                            }
                        } else {
                            println!("Printing original variable names is not yet fully supported. Use MIR SSA IDs (e.g. ssa_0).");
                        }
                    } else {
                        println!("Usage: p <var>");
                    }
                }
                "l" | "locals" => {
                    for (id, val) in locals.iter() {
                        println!("ssa_{} = {}", id.0, val.to_display());
                    }
                }
                "q" | "quit" => {
                    std::process::exit(0);
                }
                "h" | "help" => {
                    println!("vdb commands:");
                    println!("  n, next, s, step  : Execute next instruction");
                    println!("  c, continue       : Continue execution");
                    println!("  p <var>, print    : Print variable value");
                    println!("  l, locals         : Print all local variables");
                    println!("  q, quit           : Exit debugger");
                }
                _ => {
                    println!("Unknown command: {}", cmd);
                }
            }
        }

        Ok(())
    }
}

fn cmd_debug(file: &Path) -> Result<(), String> {
    let compile_res = compile_project(file, HealingMode::Deny, None)?;
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
    
    let debugger = ReplDebugger {
        source: compile_res.entry_src,
        stepping: true,
    };
    
    let debugger_ref = std::rc::Rc::new(std::cell::RefCell::new(debugger));
    interp.debugger_hook = Some(debugger_ref);

    interp
        .run_module(&ssa_module)
        .map_err(|e| format!("runtime error: {}", e.message))
}

fn cmd_build(
    file: &Path,
    output: &Path,
    backend: &str,
    emit: Option<String>,
    mode: HealingMode,
) -> Result<(), String> {
    let compile_res = compile_project(file, mode, None)?;
    let mut symbol_table = compile_res.symbol_table;
    let mut mir_module = compile_res.mir_module;

    // Collect runtime paths
    let mut runtime_paths = Vec::new();
    let rt_dir = if let Ok(root) = std::env::var("VINGLISH_ROOT") {
        PathBuf::from(root).join("rt")
    } else {
        std::env::current_dir().unwrap_or_default().join("rt")
    };

    if let Ok(entries) = fs::read_dir(&rt_dir) {
        for entry in entries.flatten() {
            if entry.path().extension().is_some_and(|ext| ext == "c") {
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
        let c_src = emit_mir_c(&ssa_module, &symbol_table)
            .map_err(|e| format!("code generation error: {}", e))?;
        let c_file = output.with_extension("c");
        fs::write(&c_file, &c_src).map_err(|e| format!("cannot write C source: {}", e))?;
        eprintln!(
            "  \x1b[32m✓\x1b[0m  Generated C source: {}",
            c_file.display()
        );

        let cc = std::env::var("CC").unwrap_or_else(|_| "cc".into());
        let mut cmd = Command::new(&cc);
        cmd.arg("-O2")
            .arg("-Wno-int-conversion")
            .arg("-o")
            .arg(output)
            .arg(&c_file);

        for rt_path in &runtime_paths {
            cmd.arg(rt_path);
        }

        let rt_rust_toml = std::env::current_dir()
            .unwrap_or_default()
            .join("rt_rust")
            .join("Cargo.toml");
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

fn cmd_check(file: &Path, mode: HealingMode) -> bool {
    let compile_res = match compile_project(file, mode, None) {
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
            eprintln!(
                "MIR validation error after pre-SSA optimization: {}",
                e.message
            );
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
            eprintln!(
                "MIR validation error after post-SSA optimization: {}",
                e.message
            );
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
        return Err(format!("no benchmarks found in '{}'", directory.display()));
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
            cmd_build(&file, &output, "c", None, HealingMode::Deny)?;
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
            fs::write(&runner_js, runner_code)
                .map_err(|e| format!("failed to write runner: {}", e))?;
        }

        let mut elapsed = Duration::ZERO;
        for _ in 0..runs {
            let start = Instant::now();
            let status = if ext == "py" {
                Command::new("python3")
                    .arg(&file)
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()
            } else if ext == "elm" {
                let runner_js = temp_dir.join(format!("{}_runner.js", name));
                Command::new("node")
                    .arg(&runner_js)
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()
            } else {
                Command::new(&output)
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()
            }
            .map_err(|e| format!("cannot run '{}': {}", display_name, e))?;

            if !status.success() {
                return Err(format!(
                    "benchmark '{}' exited with {}",
                    display_name, status
                ));
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

fn get_healing_mode(heal: bool, deny_heal: bool) -> HealingMode {
    if deny_heal {
        HealingMode::Deny
    } else if heal {
        HealingMode::ApplyInMemory
    } else {
        HealingMode::SuggestOnly
    }
}

fn cmd_fix(file: &Path, allow_dirty: bool, yes: bool) -> Result<(), String> {
    if !allow_dirty {
        let output = std::process::Command::new("git")
            .args(["status", "--porcelain"])
            .output()
            .map_err(|e| format!("Failed to run git status: {}", e))?;
        if !output.stdout.is_empty() {
            return Err("Git working tree is dirty. Commit or stash changes, or use --allow-dirty.".into());
        }
    }

    let mut collected_fixes = Vec::new();
    let _ = compile_project(file, HealingMode::SuggestOnly, Some(&mut collected_fixes));

    if collected_fixes.is_empty() {
        println!("No auto-fixable errors found.");
        return Ok(());
    }

    println!("Found {} auto-fixable error(s):", collected_fixes.len());
    for (path, _, warning) in &collected_fixes {
        println!("  - {}:{}:{} : wrap in `{}`", path.display(), warning.span.start, warning.span.end, vinglish_fmt::format_expr(&warning.replacement));
    }

    if !yes {
        use std::io::Write;
        print!("Apply these fixes? [y/N] ");
        std::io::stdout().flush().unwrap();
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).unwrap();
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Aborted.");
            return Ok(());
        }
    }

    // Group by file
    let mut files_to_fix: std::collections::HashMap<PathBuf, (String, Vec<vinglish_types::healer::HealingWarning>)> = std::collections::HashMap::new();
    for (path, src, warning) in collected_fixes.clone() {
        files_to_fix.entry(path).or_insert_with(|| (src, Vec::new())).1.push(warning);
    }

    for (path, (mut src, mut warnings)) in files_to_fix {
        warnings.sort_by_key(|w| std::cmp::Reverse(w.span.start));
        for warning in warnings {
            let start = warning.span.start as usize;
            let end = warning.span.end as usize;
            let replacement_str = vinglish_fmt::format_expr(&warning.replacement);
            src = format!("{}{}{}", &src[..start], replacement_str, &src[end..]);
        }
        std::fs::write(&path, src).map_err(|e| format!("Failed to write {}: {}", path.display(), e))?;
    }

    println!("Fixes applied. Verifying...");
    
    // We expect it to succeed now. If it still fails, it's fine, the user can see errors.
    match compile_project(file, HealingMode::Deny, None) {
        Ok(_) => {
            println!("Healed {} type mismatch(es). Review with `git diff` before committing.", collected_fixes.len());
            Ok(())
        }
        Err(_) => {
            Err("Fixes applied, but compilation still fails. Please review the errors.".into())
        }
    }
}
