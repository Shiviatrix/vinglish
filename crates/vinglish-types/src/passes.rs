use crate::TypeError;
use vinglish_hir::symbol::{
    FunctionId, FunctionSymbol, SymbolId, SymbolKind, SymbolTable, TypeId, TypeSymbol, VariableId,
};
use vinglish_hir::types::Type;
use vinglish_hir::Module as HirModule;
use vinglish_parser::ast::Visibility;
use vinglish_parser::ast::{Item as AstItem, Module as AstModule};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy)]
pub enum ScopedId {
    Type(TypeId),
    Func(FunctionId),
    Var(VariableId),
}

impl ScopedId {
    pub fn as_var(&self) -> Option<VariableId> {
        if let ScopedId::Var(id) = self {
            Some(*id)
        } else {
            None
        }
    }
    pub fn as_func(&self) -> Option<FunctionId> {
        if let ScopedId::Func(id) = self {
            Some(*id)
        } else {
            None
        }
    }
    pub fn as_type(&self) -> Option<TypeId> {
        if let ScopedId::Type(id) = self {
            Some(*id)
        } else {
            None
        }
    }
    pub fn as_raw_id(&self) -> SymbolId {
        match self {
            ScopedId::Type(id) => id.0,
            ScopedId::Func(id) => id.0,
            ScopedId::Var(id) => id.0,
        }
    }
}

pub struct CompilerContext {
    pub symbol_table: SymbolTable,
    pub type_errors: Vec<TypeError>,
    pub types: HashMap<u32, TypeId>, // Map ast node id -> resolved TypeId
    pub scope_stack: Vec<HashMap<String, ScopedId>>,
    pub current_return_type: Option<Type>,
    pub current_module: String,
}

impl Default for CompilerContext {
    fn default() -> Self {
        Self::new()
    }
}

impl CompilerContext {
    pub fn new() -> Self {
        Self::with_symbol_table(SymbolTable::new())
    }

    pub fn with_symbol_table(mut symbol_table: SymbolTable) -> Self {
        let mut scope = HashMap::new();

        // Register builtins if not already present
        let builtins = vec!["print", "println"];
        for name in builtins {
            let id = if let Some(sym_id) = symbol_table.lookup(name) {
                FunctionId(sym_id)
            } else {
                let sym_id = symbol_table.define_func(
                    name.to_string(),
                    FunctionSymbol {
                        id: vinglish_hir::symbol::FunctionId(SymbolId(0)),
                        name: name.to_string(),
                        visibility: Visibility::Public,
                        // Builtins take any type (represented by a fresh type variable) and return Unit
                        ty: Type::Function(
                            vec![Type::Var(vinglish_hir::types::TypeVar(0))],
                            Box::new(Type::Unit),
                        ),
                        generic_params: vec![],
                        is_variant_constructor: None,
                    },
                );
                if let Some(fs) = symbol_table.get_func_mut(sym_id) {
                    fs.id = sym_id;
                }
                sym_id
            };
            scope.insert(name.to_string(), ScopedId::Func(id));
        }

        Self {
            symbol_table,
            type_errors: Vec::new(),
            types: HashMap::new(),
            scope_stack: vec![scope],
            current_return_type: None,
            current_module: String::new(),
        }
    }

    pub fn push_scope(&mut self) {
        self.scope_stack.push(HashMap::new());
    }

    pub fn pop_scope(&mut self) {
        self.scope_stack.pop();
    }

    pub fn define(&mut self, name: String, id: ScopedId) {
        if let Some(scope) = self.scope_stack.last_mut() {
            scope.insert(name, id);
        }
    }

    pub fn lookup(&self, name: &str) -> Option<ScopedId> {
        for scope in self.scope_stack.iter().rev() {
            if let Some(id) = scope.get(name) {
                return Some(*id);
            }
        }

        // Fallback to symbol table names if they exist
        if let Some(sym_id) = self.symbol_table.lookup(name) {
            match self.symbol_table.get(sym_id) {
                Some(SymbolKind::Variable(_)) => Some(ScopedId::Var(VariableId(sym_id))),
                Some(SymbolKind::Function(_)) => Some(ScopedId::Func(FunctionId(sym_id))),
                Some(SymbolKind::Type(_)) => Some(ScopedId::Type(TypeId(sym_id))),
                _ => None,
            }
        } else {
            None
        }
    }
}

pub trait CompilerPass {
    fn run(&mut self, ast: &AstModule, ctx: &mut CompilerContext) -> Option<HirModule>;
}

pub struct NameResolutionPass;

impl Default for NameResolutionPass {
    fn default() -> Self {
        Self::new()
    }
}

impl NameResolutionPass {
    pub fn new() -> Self {
        Self
    }
}

impl CompilerPass for NameResolutionPass {
    fn run(&mut self, ast: &AstModule, ctx: &mut CompilerContext) -> Option<HirModule> {
        // First pass: register top-level types and functions
        for item in &ast.items {
            match item {
                AstItem::Type(t) => {
                    let mut generic_params = Vec::new();
                    for _ in &t.type_params {
                        generic_params.push(vinglish_hir::types::TypeVar::fresh());
                    }

                    let qualified_name = if ctx.current_module.is_empty() {
                        t.name.name.clone()
                    } else {
                        format!("{}.{}", ctx.current_module, t.name.name)
                    };

                    let mut ts = TypeSymbol::new(
                        vinglish_hir::symbol::TypeId(SymbolId(0)),
                        qualified_name.clone(),
                        t.visibility,
                    );
                    ts.generic_params = generic_params;

                    let id = ctx.symbol_table.define_type(qualified_name.clone(), ts);
                    if let Some(ts) = ctx.symbol_table.get_type_mut(id) {
                        ts.id = id;
                    }

                    ctx.define(t.name.name.clone(), ScopedId::Type(id));
                    if !ctx.current_module.is_empty() {
                        ctx.define(qualified_name, ScopedId::Type(id));
                    }
                }
                AstItem::Enum(e) => {
                    let mut generic_params = Vec::new();
                    for _ in &e.type_params {
                        generic_params.push(vinglish_hir::types::TypeVar::fresh());
                    }

                    let qualified_name = if ctx.current_module.is_empty() {
                        e.name.name.clone()
                    } else {
                        format!("{}.{}", ctx.current_module, e.name.name)
                    };

                    // An Enum is also a type
                    let mut ts = TypeSymbol::new(
                        vinglish_hir::symbol::TypeId(SymbolId(0)),
                        qualified_name.clone(),
                        e.visibility,
                    );
                    ts.generic_params = generic_params.clone();
                    
                    let id = ctx.symbol_table.define_type(qualified_name.clone(), ts);
                    if let Some(ts) = ctx.symbol_table.get_type_mut(id) {
                        ts.id = id;
                    }
                    
                    ctx.define(e.name.name.clone(), ScopedId::Type(id));
                    if !ctx.current_module.is_empty() {
                        ctx.define(qualified_name.clone(), ScopedId::Type(id));
                    }
                    
                    // We also need to define constructor functions for each variant
                    // E.g., `Ok` is a generic function `T -> Result<T, E>`.
                    for (index, variant) in e.variants.iter().enumerate() {
                        let variant_name = if ctx.current_module.is_empty() {
                            variant.name.name.clone()
                        } else {
                            format!("{}.{}", ctx.current_module, variant.name.name)
                        };

                        let fn_id = ctx.symbol_table.define_func(
                            variant_name.clone(),
                            FunctionSymbol {
                                id: vinglish_hir::symbol::FunctionId(SymbolId(0)),
                                name: variant_name.clone(),
                                visibility: e.visibility,
                                ty: Type::Unit, // Will be inferred in type_pass
                                generic_params: generic_params.clone(),
                                is_variant_constructor: Some((id, index + 1)), // + 1 for tag
                            },
                        );
                        if let Some(fs) = ctx.symbol_table.get_func_mut(fn_id) {
                            fs.id = fn_id;
                        }
                        ctx.define(variant.name.name.clone(), ScopedId::Func(fn_id));
                        if !ctx.current_module.is_empty() {
                            ctx.define(variant_name, ScopedId::Func(fn_id));
                        }
                    }
                }
                AstItem::Function(f) => {
                    let mut generic_params = Vec::new();
                    for _ in &f.type_params {
                        generic_params.push(vinglish_hir::types::TypeVar::fresh());
                    }

                    let qualified_name = if ctx.current_module.is_empty() {
                        f.name.name.clone()
                    } else {
                        format!("{}.{}", ctx.current_module, f.name.name)
                    };

                    let linkage_name = if f.is_foreign {
                        f.name.name.clone()
                    } else {
                        qualified_name.clone()
                    };

                    let id = ctx.symbol_table.define_func(
                        qualified_name.clone(),
                        FunctionSymbol {
                            id: vinglish_hir::symbol::FunctionId(SymbolId(0)),
                            name: linkage_name,
                            visibility: f.visibility,
                            ty: Type::Unit, // Will be inferred later
                            generic_params,
                            is_variant_constructor: None,
                        },
                    );
                    if let Some(fs) = ctx.symbol_table.get_func_mut(id) {
                        fs.id = id;
                    }

                    ctx.define(f.name.name.clone(), ScopedId::Func(id));
                    if !ctx.current_module.is_empty() {
                        ctx.define(qualified_name, ScopedId::Func(id));
                    }
                }
                AstItem::Use(u) => {
                    let path_parts: Vec<String> = u.path.iter().map(|id| id.name.clone()).collect();
                    let path_str = path_parts.join(".");
                    let prefix = format!("{}.", path_str);

                    let mut imported = Vec::new();
                    for (fq_name, &sym_id) in ctx.symbol_table.names() {
                        if fq_name.starts_with(&prefix) {
                            let is_public = match ctx.symbol_table.get(sym_id) {
                                Some(SymbolKind::Function(fs)) => {
                                    fs.visibility == Visibility::Public
                                }
                                Some(SymbolKind::Type(ts)) => ts.visibility == Visibility::Public,
                                _ => false,
                            };
                            if is_public {
                                let short_name = fq_name.strip_prefix(&prefix).unwrap().to_string();
                                imported.push((short_name, fq_name.clone(), sym_id));
                            }
                        }
                    }

                    for (short_name, fq_name, sym_id) in imported {
                        let scoped_id = match ctx.symbol_table.get(sym_id) {
                            Some(SymbolKind::Function(_)) => ScopedId::Func(FunctionId(sym_id)),
                            Some(SymbolKind::Type(_)) => ScopedId::Type(TypeId(sym_id)),
                            _ => continue,
                        };
                        ctx.define(short_name, scoped_id);
                        ctx.define(fq_name, scoped_id);
                    }
                }
                _ => {}
            }
        }
        None // Name resolution doesn't produce HIR, it just populates SymbolTable
    }
}
