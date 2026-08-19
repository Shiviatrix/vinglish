//! MIR-only C backend. AST nodes cannot enter this API.

use std::collections::{BTreeMap, HashSet};
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;
use std::fmt::Write;
use thiserror::Error;
use vinglish_hir::symbol::{FunctionId, SsaValueId, SymbolTable, VariableId};
use vinglish_hir::types::Type;
use vinglish_mir::{Instruction, MirFunction, MirModule, Operand, Terminator};
use vinglish_parser::ast::{BinOp, Literal, UnOp};

#[derive(Debug, Error)]
pub enum MirCEmitError {
    #[error("formatting C output failed")]
    Fmt(#[from] std::fmt::Error),
}

/// The C backend only needs the stable numeric SSA identity; it deliberately
/// does not need an AST name or type annotation.
pub trait CValueId: Copy + std::fmt::Display + Eq {
    fn raw(self) -> u32;
}
impl CValueId for VariableId {
    fn raw(self) -> u32 {
        self.0.0
    }
}
impl CValueId for SsaValueId {
    fn raw(self) -> u32 {
        self.0
    }
}

use base64::Engine;
use flate2::Compression;
use flate2::write::ZlibEncoder;
use sha2::{Digest, Sha256};
use std::io::Write as IoWrite;

/// Emit C exclusively from optimized SSA MIR. Tags are C comments, removed by
/// standard preprocessing and therefore have zero runtime/object-code cost.
pub fn emit_mir_c<V: CValueId + serde::Serialize>(
    module: &MirModule<V>,
    symbols: &SymbolTable,
) -> Result<String, MirCEmitError> {
    let pool = StringPool::collect(module);
    let mut out = String::from(
        "/* Generated from Vinglish SSA MIR. */\n#include <stdint.h>\n#include <stddef.h>\n#include <inttypes.h>\n#include <stdio.h>\n#include <stdlib.h>\n#include <stdbool.h>\nstatic int64_t ving_print_text(const char *value) { return fputs(value, stdout); }\nstatic int64_t ving_print_double(double value) { return printf(\"%g\", value); }\nstatic int64_t ving_print_bool(bool value) { return fputs(value ? \"true\" : \"false\", stdout); }\nstatic int64_t ving_print_i64(int64_t value) { return printf(\"%\" PRId64, value); }\nstatic int64_t ving_println_text(const char *value) { return printf(\"%s\\n\", value); }\nstatic int64_t ving_println_double(double value) { return printf(\"%g\\n\", value); }\nstatic int64_t ving_println_bool(bool value) { return fputs(value ? \"true\\n\" : \"false\\n\", stdout); }\nstatic int64_t ving_println_i64(int64_t value) { return printf(\"%\" PRId64 \"\\n\", value); }\n#define print(x) _Generic((x), const char*: ving_print_text, char*: ving_print_text, double: ving_print_double, bool: ving_print_bool, default: ving_print_i64)(x)\n#define println(x) _Generic((x), const char*: ving_println_text, char*: ving_println_text, double: ving_println_double, bool: ving_println_bool, default: ving_println_i64)(x)\n#define abs llabs\nstatic inline const char* _to_text_double(double x) {\n    char* buf = malloc(32);\n    snprintf(buf, 32, \"%g\", x);\n    return buf;\n}\nstatic inline const char* _to_text_long(long x) {\n    char* buf = malloc(32);\n    snprintf(buf, 32, \"%ld\", x);\n    return buf;\n}\n#define to_text(x) _Generic((x), \\\n    double: _to_text_double(x), \\\n    long: _to_text_long(x), \\\n    int: _to_text_long(x) \\\n)\nextern const char* ving_str_concat(const char*, const char*);\nextern int64_t rt_list_new(int64_t);\nextern int64_t rt_list_get(int64_t, int64_t);\nextern int64_t rt_list_borrow_get(int64_t, int64_t);\nextern void rt_list_set(int64_t, int64_t, int64_t);\nextern int64_t rt_list_len(int64_t);\nextern void rt_list_push(int64_t, int64_t);\nextern int64_t rt_list_pop(int64_t);\nextern void rt_list_free(int64_t);\nextern uintptr_t ving_map_free(uintptr_t);\n\n",
    );
    for inc in &module.foreign_includes {
        writeln!(out, "#include \"{}\"", inc)?;
    }
    out.push('\n');
    for (text, index) in &pool.entries {
        writeln!(
            out,
            "static const char *const string_literal_{index} = \"{}\";",
            escape_c_string(text)
        )?;
    }
    if !pool.entries.is_empty() {
        out.push('\n');
    }
    for function in &module.functions {
        if !function.is_foreign {
            if function.name == "main" {
                out.push_str("int main(void);\n");
            } else {
                write!(
                    out,
                    "static {} fn_{}(",
                    function_c_return_type(function, symbols),
                    function.id.0.0
                )?;
                for (index, param) in function.params.iter().enumerate() {
                    if index != 0 {
                        out.push_str(", ");
                    }
                    out.push_str(c_value_type(*param, symbols));
                }
                out.push_str(");\n");
            }
        }
    }
    for function in &module.functions {
        if function.is_foreign
            && let Some(fn_sym) = symbols.get_func(function.id)
        {
            let symbol = c_ident(&fn_sym.name);
            if symbol != "print"
                && symbol != "println"
                && symbol != "abs"
                && let vinglish_hir::types::Type::Function(params, ret) = &fn_sym.ty
            {
                let mut arg_types = Vec::new();
                for p in params {
                    arg_types.push(to_c_type(p));
                }
                let args_decl = if arg_types.is_empty() {
                    "void".to_string()
                } else {
                    arg_types.join(", ")
                };
                writeln!(out, "extern {} {}({});", to_c_type(ret), symbol, args_decl)?;
            }
        }
    }
    out.push('\n');
    emit_drop_functions(&mut out, module, symbols).unwrap();
    out.push('\n');

    for function in &module.functions {
        if !function.is_foreign {
            emit_function(&mut out, function, symbols, module, &pool)?;
        }
    }

    // Epic 2: Binary Delta Encoding (Payload compression with tamper detection)
    let mut hasher = Sha256::new();
    hasher.update(out.as_bytes());
    let c_hash = hasher
        .finalize()
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<String>();

    let module_bytes = bincode::serialize(&module).unwrap();
    let payload: (String, Vec<u8>) = (c_hash, module_bytes);
    let serialized = bincode::serialize(&payload).unwrap();
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&serialized).unwrap();
    let compressed = encoder.finish().unwrap();
    let base64_payload = base64::engine::general_purpose::STANDARD.encode(compressed);

    writeln!(out, "/* VINGLISH_MIR_PAYLOAD: {} */", base64_payload)?;
    Ok(out)
}

fn emit_function<V: CValueId>(
    out: &mut String,
    function: &MirFunction<V>,
    symbols: &SymbolTable,
    module: &MirModule<V>,
    pool: &StringPool,
) -> Result<(), MirCEmitError> {
    if function.name == "main" {
        out.push_str("int main(");
    } else {
        let ret_c_type = function_c_return_type(function, symbols);
        write!(out, "static {} fn_{}(", ret_c_type, function.id.0.0)?;
    }
    for (index, param) in function.params.iter().enumerate() {
        if index != 0 {
            out.push_str(", ");
        }
        write!(out, "{} v_{}", c_value_type(*param, symbols), param.raw())?;
    }
    out.push_str(") {\n");
    let stack_allocations = stack_allocations(function);
    for (value, size) in &stack_allocations {
        writeln!(
            out,
            "    union {{ max_align_t align; unsigned char bytes[{}]; }} stack_storage_{};",
            size, value
        )?;
    }
    for local in &function.locals {
        if !function.params.contains(local) {
            writeln!(
                out,
                "    {} v_{} = {};",
                c_value_type(*local, symbols),
                local.raw(),
                c_zero(*local, symbols)
            )?;
        }
    }
    let mut phi_assignments: std::collections::HashMap<vinglish_mir::BlockId, Vec<String>> =
        std::collections::HashMap::new();
    for block in &function.blocks {
        for instruction in &block.instrs {
            if let vinglish_mir::Instruction::Phi(d, values) = instruction {
                for (v, pred) in values {
                    let assign = format!("v_{} = {}", d.raw(), operand(v, pool));
                    phi_assignments.entry(*pred).or_default().push(assign);
                }
            }
        }
    }

    for block in &function.blocks {
        writeln!(out, "bb_{}_{}:", function.id.0.0, block.id.0)?;
        for instruction in &block.instrs {
            if let vinglish_mir::Instruction::Phi(_, _) = instruction {
                continue;
            }
            writeln!(
                out,
                "    {};",
                instruction_to_c(instruction, symbols, module, pool, &stack_allocations)
            )?;
        }
        if let Some(assignments) = phi_assignments.get(&block.id) {
            for assign in assignments {
                writeln!(out, "    {};", assign)?;
            }
        }
        emit_terminator(out, function, &block.terminator, pool)?;
    }
    out.push_str("}\n\n");
    Ok(())
}

fn instruction_to_c<V: CValueId>(
    i: &Instruction<V>,
    symbols: &SymbolTable,
    module: &MirModule<V>,
    pool: &StringPool,
    stack_allocations: &BTreeMap<u32, u32>,
) -> String {
    match i {
        Instruction::Assign(d, v) => format!("v_{} = {}", d.raw(), operand(v, pool)),
        Instruction::BinaryOp(d, op, l, r) => {
            let is_string = match l {
                vinglish_mir::Operand::Constant(vinglish_parser::ast::Literal::Text(_)) => true,
                vinglish_mir::Operand::Var(v) => c_value_type(*v, symbols) == "const char *",
                _ => false,
            };
            if is_string && *op == vinglish_parser::ast::BinOp::Add {
                format!(
                    "v_{} = ving_str_concat({}, {})",
                    d.raw(),
                    operand(l, pool),
                    operand(r, pool)
                )
            } else {
                format!(
                    "v_{} = {} {} {}",
                    d.raw(),
                    operand(l, pool),
                    binop(*op),
                    operand(r, pool)
                )
            }
        }
        Instruction::UnaryOp(d, op, v) => {
            format!("v_{} = {}{}", d.raw(), unop(*op), operand(v, pool))
        }
        Instruction::Call(d, f, a) => format!(
            "v_{} = {}({})",
            d.raw(),
            match f {
                vinglish_mir::CallTarget::Direct(id) => call_name(*id, symbols, module),
                vinglish_mir::CallTarget::Foreign { c_symbol } => c_ident(c_symbol),
            },
            a.iter()
                .map(|value| operand(value, pool))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Instruction::CallIntrinsic(d, name, a) => format!(
            "v_{} = {}({})",
            d.raw(),
            c_ident(name),
            a.iter()
                .map(|value| operand(value, pool))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Instruction::Phi(_, _) => unreachable!("Phi handled in block loop"),
        Instruction::LoadField(d, object, access) => format!(
            "v_{} = *({} *)((unsigned char *)(uintptr_t){} + {})",
            d.raw(),
            c_value_type(*d, symbols),
            operand(object, pool),
            access.byte_offset
        ),
        Instruction::StoreField(object, access, value) => {
            let ty = match value {
                vinglish_mir::Operand::Var(v) => c_value_type(*v, symbols),
                vinglish_mir::Operand::Constant(vinglish_parser::ast::Literal::Float(_)) => {
                    "double"
                }
                vinglish_mir::Operand::Constant(vinglish_parser::ast::Literal::Bool(_)) => "bool",
                vinglish_mir::Operand::Constant(vinglish_parser::ast::Literal::Text(_)) => {
                    "const char *"
                }
                _ => "int64_t",
            };
            format!(
                "*({} *)((unsigned char *)(uintptr_t)v_{} + {}) = {}",
                ty,
                object.raw(),
                access.byte_offset,
                operand(value, pool)
            )
        }
        Instruction::HeapAllocate(d, layout) => {
            format!("v_{} = (uintptr_t)calloc(1, {})", d.raw(), layout.size)
        }
        Instruction::StackAllocate(d, _) => {
            format!("v_{} = (uintptr_t)stack_storage_{}.bytes", d.raw(), d.raw())
        }
        Instruction::Borrow(d, v) | Instruction::BorrowMut(d, v) => {
            if let vinglish_mir::Operand::Var(var) = v {
                let ty = symbols
                    .get_var(VariableId(vinglish_hir::symbol::SymbolId(var.raw())))
                    .map(|s| &s.ty);
                match ty {
                    Some(Type::Named(_, _))
                    | Some(Type::List(_))
                    | Some(Type::Dict(_, _))
                    | Some(Type::Pointer(_))
                    | Some(Type::Reference(_, _)) => {
                        format!("v_{} = v_{}", d.raw(), var.raw())
                    }
                    _ => format!("v_{} = (uintptr_t)&v_{}", d.raw(), var.raw()),
                }
            } else {
                unreachable!("Cannot borrow constant");
            }
        }
        Instruction::Deref(d, v, _) => {
            format!("v_{} = *(int64_t*)(uintptr_t){}", d.raw(), operand(v, pool))
        }
        Instruction::StoreDeref(ptr, val) => {
            format!(
                "*(int64_t*)(uintptr_t){} = {}",
                operand(ptr, pool),
                operand(val, pool)
            )
        }
        Instruction::ListNew(d, cap) => {
            format!("v_{} = rt_list_new({})", d.raw(), operand(cap, pool))
        }
        Instruction::ListGet(d, list, idx) => {
            format!(
                "v_{} = rt_list_get({}, {})",
                d.raw(),
                operand(list, pool),
                operand(idx, pool)
            )
        }
        Instruction::ListBorrowGet(d, list, idx) | Instruction::ListBorrowMutGet(d, list, idx) => {
            format!(
                "v_{} = rt_list_borrow_get({}, {})",
                d.raw(),
                operand(list, pool),
                operand(idx, pool)
            )
        }
        Instruction::ListSet(list, idx, val) => format!(
            "rt_list_set({}, {}, {})",
            operand(list, pool),
            operand(idx, pool),
            operand(val, pool)
        ),
        Instruction::ListLen(d, list) => {
            format!("v_{} = rt_list_len({})", d.raw(), operand(list, pool))
        }
        Instruction::ListPush(list, val) => format!(
            "rt_list_push({}, {})",
            operand(list, pool),
            operand(val, pool)
        ),
        Instruction::ListPop(d, list) => {
            format!("v_{} = rt_list_pop({})", d.raw(), operand(list, pool))
        }
        Instruction::Drop(var) => {
            if stack_allocations.contains_key(&var.raw()) {
                return format!("/* stack storage v_{} expires automatically */", var.raw());
            }
            if let Some(symbol) =
                symbols.get_var(VariableId(vinglish_hir::symbol::SymbolId(var.raw())))
            {
                match &symbol.ty {
                    Type::Reference(_, _)
                    | Type::Pointer(_)
                    | Type::Int
                    | Type::Float
                    | Type::Bool
                    | Type::Text
                    | Type::Unit => {
                        format!("/* skip free v_{} */", var.raw())
                    }
                    Type::List(_) | Type::Dict(_, _) | Type::HashMap(_, _) => {
                        format!("drop_type_{}((uintptr_t)v_{})", type_hash(&symbol.ty), var.raw())
                    }
                    _ => {
                        if !symbol.ty.is_copy() && !matches!(symbol.ty, Type::Text) {
                            format!("drop_type_{}((uintptr_t)v_{})", type_hash(&symbol.ty), var.raw())
                        } else {
                            format!("/* skip free v_{} (is_copy) */", var.raw())
                        }
                    }
                }
            } else {
                format!("free((void *)(uintptr_t)v_{})", var.raw())
            }
        }
    }
}

/// Stack allocations have function scope in generated C. Keeping their storage
/// automatic preserves the MIR distinction from heap allocations and makes a
/// subsequent `Drop` a no-op rather than an invalid `free`.
fn stack_allocations<V: CValueId>(function: &MirFunction<V>) -> BTreeMap<u32, u32> {
    let mut allocations = BTreeMap::new();
    for block in &function.blocks {
        for instruction in &block.instrs {
            if let Instruction::StackAllocate(value, layout) = instruction {
                allocations
                    .entry(value.raw())
                    .and_modify(|size: &mut u32| *size = (*size).max(layout.size.max(1)))
                    .or_insert(layout.size.max(1));
            }
        }
    }
    allocations
}
fn emit_terminator<V: CValueId>(
    out: &mut String,
    f: &MirFunction<V>,
    t: &Terminator<V>,
    pool: &StringPool,
) -> Result<(), MirCEmitError> {
    match t {
        Terminator::Return(Some(v)) => {
            if f.name == "main" {
                writeln!(out, "    return 0;")?;
            } else {
                writeln!(out, "    return {};", operand(v, pool))?;
            }
        }
        Terminator::Return(None) => writeln!(out, "    return 0;")?,
        Terminator::Jump(target) => writeln!(out, "    goto bb_{}_{};", f.id.0.0, target.0)?,
        Terminator::Branch(c, yes, no) => writeln!(
            out,
            "    if ({}) goto bb_{}_{}; else goto bb_{}_{};",
            operand(c, pool),
            f.id.0.0,
            yes.0,
            f.id.0.0,
            no.0
        )?,
    };
    Ok(())
}
fn operand<V: CValueId>(v: &Operand<V>, pool: &StringPool) -> String {
    match v {
        Operand::Var(id) => format!("v_{}", id.raw()),
        Operand::Constant(value) => literal(value, pool),
    }
}
fn literal(v: &Literal, pool: &StringPool) -> String {
    match v {
        Literal::Int(v) => v.to_string(),
        Literal::Float(v) => format!("{v}"),
        Literal::Bool(v) => {
            if *v {
                "(bool)true".into()
            } else {
                "(bool)false".into()
            }
        }
        Literal::Text(text) => format!("string_literal_{}", pool.entries[text]),
        Literal::Unit => "0".into(),
    }
}
fn binop(op: BinOp) -> &'static str {
    match op {
        BinOp::Add => "+",
        BinOp::Sub => "-",
        BinOp::Mul => "*",
        BinOp::Div => "/",
        BinOp::Mod => "%",
        BinOp::Eq => "==",
        BinOp::NotEq => "!=",
        BinOp::Lt | BinOp::IsBelow => "<",
        BinOp::Gt | BinOp::IsAbove | BinOp::Exceeds => ">",
        BinOp::LtEq => "<=",
        BinOp::GtEq => ">=",
        BinOp::And => "&&",
        BinOp::Or => "||",
        BinOp::BitXor => "^",
        BinOp::BitAnd => "&",
        BinOp::BitOr => "|",
        BinOp::Shl => "<<",
        BinOp::Shr => ">>",
    }
}
fn unop(op: UnOp) -> &'static str {
    match op {
        UnOp::Neg => "-",
        UnOp::Not => "!",
        UnOp::Deref | UnOp::Borrow(_) => "",
    }
}
fn call_name<V: CValueId>(id: FunctionId, symbols: &SymbolTable, module: &MirModule<V>) -> String {
    if module.functions.iter().any(|f| f.id == id) {
        format!("fn_{}", id.0.0)
    } else {
        symbols
            .get_func(id)
            .map(|f| c_ident(&f.name))
            .unwrap_or_else(|| format!("fn_{}", id.0.0))
    }
}

fn function_c_return_type<V: CValueId>(
    function: &MirFunction<V>,
    symbols: &SymbolTable,
) -> &'static str {
    symbols
        .get_func(function.id)
        .and_then(|symbol| match &symbol.ty {
            Type::Function(_, ret) => Some(to_c_type(ret)),
            _ => None,
        })
        .unwrap_or("int64_t")
}

fn type_hash(ty: &Type) -> u64 {
    let mut hasher = DefaultHasher::new();
    ty.hash(&mut hasher);
    hasher.finish()
}

fn emit_drop_functions<V: CValueId>(
    out: &mut String,
    module: &MirModule<V>,
    symbols: &SymbolTable,
) -> std::fmt::Result {
    use std::fmt::Write;
    let mut dropped_types = HashSet::new();

    // Collect all dropped types
    for function in &module.functions {
        for block in &function.blocks {
            for instruction in &block.instrs {
                if let Instruction::Drop(var) = instruction {
                    if let Some(symbol) = symbols.get_var(VariableId(vinglish_hir::symbol::SymbolId(var.raw()))) {
                        dropped_types.insert(symbol.ty.clone());
                    }
                }
            }
        }
    }

    // Expand to include inner types (e.g. List<List<T>> needs drop for List<T>)
    let mut queue: Vec<Type> = dropped_types.into_iter().collect();
    let mut processed = HashSet::new();
    let mut to_emit = Vec::new();

    while let Some(ty) = queue.pop() {
        if processed.contains(&ty) {
            continue;
        }
        processed.insert(ty.clone());
        to_emit.push(ty.clone());

        match &ty {
            Type::List(inner) => queue.push(*inner.clone()),
            Type::Dict(k, v) | Type::HashMap(k, v) => {
                queue.push(*k.clone());
                queue.push(*v.clone());
            }
            Type::Named(_, _) | Type::Tuple(_) | Type::Set(_) | Type::Array(_, _) | Type::Optional(_) | Type::Result(_, _) | Type::Box(_) => {
                // Not fully implemented recursive drop for all these yet
            }
            _ => {}
        }
    }

    // Forward declare drop functions
    for ty in &to_emit {
        let hash = type_hash(ty);
        writeln!(out, "static void drop_type_{}(uintptr_t ptr);", hash)?;
    }
    
    // Implement drop functions
    for ty in to_emit {
        let hash = type_hash(&ty);
        writeln!(out, "static void drop_type_{}(uintptr_t ptr) {{", hash)?;
        writeln!(out, "    if (!ptr) return;")?;
        match &ty {
            Type::List(inner) => {
                writeln!(out, "    int64_t len = rt_list_len(ptr);")?;
                writeln!(out, "    for (int64_t i = 0; i < len; i++) {{")?;
                writeln!(out, "        uintptr_t elem = rt_list_get(ptr, i);")?;
                if inner.is_copy() || matches!(**inner, Type::Text) {
                    // primitive, no drop needed
                } else {
                    let inner_hash = type_hash(inner);
                    writeln!(out, "        drop_type_{}(elem);", inner_hash)?;
                }
                writeln!(out, "    }}")?;
                writeln!(out, "    rt_list_free(ptr);")?;
            }
            Type::Dict(_, _) | Type::HashMap(_, _) => {
                // Not implemented iterating over map in C yet, just free it
                writeln!(out, "    ving_map_free(ptr);")?;
            }
            _ => {
                if !ty.is_copy() && !matches!(ty, Type::Text) {
                    writeln!(out, "    free((void *)ptr);")?;
                }
            }
        }
        writeln!(out, "}}")?;
    }

    Ok(())
}

fn c_ident(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

fn to_c_type(ty: &Type) -> &'static str {
    match ty {
        Type::Int | Type::Unit => "int64_t",
        Type::Bool => "bool",
        Type::Float => "double",
        Type::Text => "const char *",
        Type::Reference(_, _)
        | Type::Pointer(_)
        | Type::List(_)
        | Type::Dict(_, _)
        | Type::HashMap(_, _)
        | Type::Optional(_)
        | Type::Result(_, _)
        | Type::Box(_)
        | Type::Named(_, _)
        | Type::Function(_, _)
        | Type::Var(_) => "uintptr_t",
        Type::Never => "void", // ! type - zero sized
        Type::Array(inner, _len) => {
            // For simplicity, treat arrays as pointers to their element type
            to_c_type(inner)
        }
        Type::Set(_) => "uintptr_t",   // Treat as pointer to hash set
        Type::Tuple(_) => "uintptr_t", // Treat as pointer to struct
    }
}
fn c_value_type<V: CValueId>(value: V, symbols: &SymbolTable) -> &'static str {
    symbols
        .get_var(VariableId(vinglish_hir::symbol::SymbolId(value.raw())))
        .map(|symbol| to_c_type(&symbol.ty))
        .unwrap_or("int64_t")
}
fn c_zero<V: CValueId>(value: V, symbols: &SymbolTable) -> &'static str {
    if c_value_type(value, symbols) == "const char *" {
        "NULL"
    } else if c_value_type(value, symbols) == "double" {
        "0.0"
    } else {
        "0"
    }
}

struct StringPool {
    entries: BTreeMap<String, usize>,
}
impl StringPool {
    fn collect<V: CValueId>(module: &MirModule<V>) -> Self {
        let mut entries = BTreeMap::new();
        for literal in module
            .functions
            .iter()
            .flat_map(|function| function.blocks.iter())
            .flat_map(|block| block.instrs.iter())
            .flat_map(instruction_literals)
        {
            if let Literal::Text(text) = literal {
                let next = entries.len();
                entries.entry(text.clone()).or_insert(next);
            }
        }
        Self { entries }
    }
}
fn instruction_literals<V: CValueId>(instruction: &Instruction<V>) -> Vec<&Literal> {
    let operands: Vec<&Operand<V>> = match instruction {
        Instruction::Assign(_, value)
        | Instruction::UnaryOp(_, _, value)
        | Instruction::Borrow(_, value)
        | Instruction::BorrowMut(_, value)
        | Instruction::Deref(_, value, _) => vec![value],
        Instruction::BinaryOp(_, _, left, right) => vec![left, right],
        Instruction::Call(_, _, values) | Instruction::CallIntrinsic(_, _, values) => {
            values.iter().collect()
        }
        Instruction::LoadField(_, object, _) => vec![object],
        Instruction::StoreField(_, _, value) => vec![value],
        Instruction::Phi(_, values) => values.iter().map(|(value, _)| value).collect(),
        Instruction::ListNew(_, cap) => vec![cap],
        Instruction::ListGet(_, list, idx)
        | Instruction::ListBorrowGet(_, list, idx)
        | Instruction::ListBorrowMutGet(_, list, idx) => vec![list, idx],
        Instruction::ListSet(list, idx, val) => vec![list, idx, val],
        Instruction::ListLen(_, list) => vec![list],
        Instruction::ListPush(list, val) => vec![list, val],
        Instruction::ListPop(_, list) => vec![list],
        _ => vec![],
    };
    operands
        .into_iter()
        .filter_map(|operand| match operand {
            Operand::Constant(literal) => Some(literal),
            Operand::Var(_) => None,
        })
        .collect()
}
fn escape_c_string(text: &str) -> String {
    text.chars()
        .flat_map(|character| match character {
            '\\' => "\\\\".chars().collect::<Vec<_>>(),
            '"' => "\\\"".chars().collect(),
            '\n' => "\\n".chars().collect(),
            '\r' => "\\r".chars().collect(),
            '\t' => "\\t".chars().collect(),
            character if character.is_control() => {
                format!("\\x{:02x}", character as u32).chars().collect()
            }
            character => vec![character],
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use vinglish_hir::{
        symbol::{FunctionId, SymbolId, TypeId, VariableSymbol},
        types::Type,
    };
    use vinglish_mir::{AllocationLayout, BasicBlock, BlockId};
    #[test]
    fn metadata_preserves_instruction_ids() {
        let value = VariableId(SymbolId(1));
        let module = MirModule {
            functions: vec![MirFunction {
                id: FunctionId(SymbolId(9)),
                is_foreign: false,
                name: "f".into(),
                params: vec![],
                locals: vec![value],
                blocks: vec![BasicBlock {
                    spans: vec![],
                    id: BlockId(0),
                    instrs: vec![Instruction::Assign(
                        value,
                        Operand::Constant(Literal::Int(7)),
                    )],
                    terminator: Terminator::Return(Some(Operand::Var(value))),
                }],
            }],
            foreign_includes: vec![],
        };
        let c = emit_mir_c(&module, &SymbolTable::new()).unwrap();
        let bytes = vinglish_decompile::extract_mir_payload(&c).unwrap();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn stack_allocations_use_automatic_storage_and_are_not_freed() {
        let value = VariableId(SymbolId(1));
        let module = MirModule {
            functions: vec![MirFunction {
                id: FunctionId(SymbolId(9)),
                is_foreign: false,
                name: "main".into(),
                params: vec![],
                locals: vec![value],
                blocks: vec![BasicBlock {
                    spans: vec![],
                    id: BlockId(0),
                    instrs: vec![
                        Instruction::StackAllocate(
                            value,
                            AllocationLayout {
                                layout: TypeId(SymbolId(2)),
                                size: 8,
                                align: 8,
                            },
                        ),
                        Instruction::Drop(value),
                    ],
                    terminator: Terminator::Return(None),
                }],
            }],
            foreign_includes: vec![],
        };

        let c = emit_mir_c(&module, &SymbolTable::new()).unwrap();
        assert!(
            c.contains("union { max_align_t align; unsigned char bytes[8]; } stack_storage_1;")
        );
        assert!(c.contains("v_1 = (uintptr_t)stack_storage_1.bytes;"));
        assert!(c.contains("stack storage v_1 expires automatically"));
        assert!(!c.contains("free((void *)(uintptr_t)v_1)"));
    }

    #[test]
    fn collection_drops_use_runtime_destructors() {
        let value = VariableId(SymbolId(1));
        let module = MirModule {
            functions: vec![MirFunction {
                id: FunctionId(SymbolId(9)),
                is_foreign: false,
                name: "main".into(),
                params: vec![],
                locals: vec![value],
                blocks: vec![BasicBlock {
                    spans: vec![],
                    id: BlockId(0),
                    instrs: vec![Instruction::Drop(value)],
                    terminator: Terminator::Return(None),
                }],
            }],
            foreign_includes: vec![],
        };
        let mut symbols = SymbolTable::new();
        symbols.define_var_with_id(
            SymbolId(1),
            VariableSymbol {
                id: value,
                name: "items".into(),
                is_mut: false,
                ty: Type::List(Box::new(Type::Int)),
                span: None,
            },
        );

        let c = emit_mir_c(&module, &symbols).unwrap();
        assert!(c.contains("rt_list_free((int64_t)v_1)"));
        assert!(!c.contains("free((void *)(uintptr_t)v_1)"));
    }
}
