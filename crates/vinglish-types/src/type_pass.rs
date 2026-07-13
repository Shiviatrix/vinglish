use std::collections::HashMap;

use vinglish_lexer::Span;
use vinglish_parser::ast::*;
use strsim::levenshtein;

use crate::passes::{CompilerContext, CompilerPass, ScopedId};
use vinglish_hir::symbol::{
    FieldId, FunctionId, FunctionSymbol, SymbolId, SymbolKind, SymbolTable, TypeId, VariableId,
    VariableSymbol,
};
use vinglish_hir::types::{Type, TypeVar};
use vinglish_hir::Block as HirBlock;
use vinglish_hir::Expr as HirExpr;
use vinglish_hir::FunctionDef as HirFunctionDef;
use vinglish_hir::Item as HirItem;
use vinglish_hir::Module as HirModule;
use vinglish_hir::Param as HirParam;
use vinglish_hir::Stmt as HirStmt;
use vinglish_hir::TypeDef as HirTypeDef;

// ─────────────────────────────────────────────────────────────────────────────
// Errors
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct TypeError {
    pub message: String,
    pub span: Span,
}

impl TypeError {
    pub fn new(msg: impl Into<String>, span: Span) -> Self {
        Self {
            message: msg.into(),
            span,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Union-Find for type variable unification
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Default)]
struct UnionFind {
    parent: HashMap<u32, Type>,
}

impl UnionFind {
    fn new() -> Self {
        Self {
            parent: HashMap::new(),
        }
    }

    fn resolve(&self, mut ty: Type) -> Type {
        loop {
            match &ty {
                Type::Var(v) => {
                    if let Some(bound) = self.parent.get(&v.0) {
                        let bound = bound.clone();
                        ty = bound;
                    } else {
                        break;
                    }
                }
                _ => break,
            }
        }
        ty
    }

    fn bind(&mut self, var: TypeVar, ty: Type) -> Result<(), TypeError> {
        let resolved = self.resolve(ty.clone());
        if let Type::Var(v2) = &resolved {
            if v2.0 == var.0 {
                return Ok(());
            }
        }
        if self.occurs(var.0, &resolved) {
            return Err(TypeError::new(
                format!("recursive type: cannot unify '{}' with '{}'", var, resolved),
                Span::dummy(),
            ));
        }
        self.parent.insert(var.0, resolved);
        Ok(())
    }

    fn unify(&mut self, a: Type, b: Type, span: Span) -> Result<(), TypeError> {
        let a = self.resolve(a);
        let b = self.resolve(b);
        match (a, b) {
            (Type::Var(va), Type::Var(vb)) if va.0 == vb.0 => Ok(()),
            (Type::Var(v), ty) | (ty, Type::Var(v)) => self.bind(v, ty),
            (Type::Int, Type::Int) => Ok(()),
            (Type::Float, Type::Float) => Ok(()),
            (Type::Bool, Type::Bool) => Ok(()),
            (Type::Text, Type::Text) => Ok(()),
            (Type::Unit, Type::Unit) => Ok(()),
            (Type::Int, Type::Float) | (Type::Float, Type::Int) => Ok(()),
            (Type::List(a), Type::List(b)) => self.unify(*a, *b, span),
            (Type::Dict(ka, va), Type::Dict(kb, vb)) => {
                self.unify(*ka, *kb, span)?;
                self.unify(*va, *vb, span)
            }
            (Type::Optional(a), Type::Optional(b)) => self.unify(*a, *b, span),
            (Type::Result(a1, a2), Type::Result(b1, b2)) => {
                self.unify(*a1, *b1, span)?;
                self.unify(*a2, *b2, span)
            }
            (Type::Function(aa, ar), Type::Function(ba, br)) => {
                if aa.len() != ba.len() {
                    return Err(TypeError::new(
                        format!(
                            "function arity mismatch: expected {} args, got {}",
                            aa.len(),
                            ba.len()
                        ),
                        span,
                    ));
                }
                for (x, y) in aa.into_iter().zip(ba) {
                    self.unify(x, y, span)?;
                }
                self.unify(*ar, *br, span)
            }
            (Type::Reference(a_inner, a_mut), Type::Reference(b_inner, b_mut)) => {
                if a_mut != b_mut {
                    return Err(TypeError::new(
                        "mutability mismatch in references".to_string(),
                        span,
                    ));
                }
                self.unify(*a_inner, *b_inner, span)
            }
            (Type::Pointer(a_inner), Type::Pointer(b_inner)) => {
                self.unify(*a_inner, *b_inner, span)
            }
            (Type::Named(na, arga), Type::Named(nb, argb)) => {
                if na != nb || arga.len() != argb.len() {
                    return Err(TypeError::new(
                        format!("type mismatch: expected `{}`, got `{}`", na, nb),
                        span,
                    ));
                }
                for (x, y) in arga.into_iter().zip(argb) {
                    self.unify(x, y, span)?;
                }
                Ok(())
            }
            (t1, t2) => Err(TypeError::new(
                format!("type mismatch: expected `{}`, got `{}`", t1, t2),
                span,
            )),
        }
    }

    fn occurs(&self, var: u32, ty: &Type) -> bool {
        match self.resolve(ty.clone()) {
            Type::Var(v) => v.0 == var,
            Type::List(t) => self.occurs(var, &t),
            Type::Dict(k, v) => self.occurs(var, &k) || self.occurs(var, &v),
            Type::Optional(t) => self.occurs(var, &t),
            Type::Result(ok, err) => self.occurs(var, &ok) || self.occurs(var, &err),
            Type::Function(args, ret) => {
                args.iter().any(|a| self.occurs(var, a)) || self.occurs(var, &ret)
            }
            Type::Named(_, args) => args.iter().any(|a| self.occurs(var, a)),
            Type::Reference(inner, _) => self.occurs(var, &inner),
            Type::Pointer(inner) => self.occurs(var, &inner),
            _ => false,
        }
    }

    fn apply(&self, ty: &Type) -> Type {
        let ty = self.resolve(ty.clone());
        match ty {
            Type::Var(v) => {
                if let Some(bound) = self.parent.get(&v.0) {
                    self.apply(bound)
                } else {
                    Type::Var(v)
                }
            }
            Type::List(t) => Type::List(Box::new(self.apply(&t))),
            Type::Dict(k, v) => Type::Dict(Box::new(self.apply(&k)), Box::new(self.apply(&v))),
            Type::Optional(t) => Type::Optional(Box::new(self.apply(&t))),
            Type::Result(ok, err) => {
                Type::Result(Box::new(self.apply(&ok)), Box::new(self.apply(&err)))
            }
            Type::Function(args, ret) => {
                let args = args.iter().map(|a| self.apply(a)).collect();
                Type::Function(args, Box::new(self.apply(&ret)))
            }
            Type::Named(n, args) => {
                Type::Named(n, args.into_iter().map(|a| self.apply(&a)).collect())
            }
            Type::Reference(inner, mutable) => {
                Type::Reference(Box::new(self.apply(&inner)), mutable)
            }
            Type::Pointer(inner) => Type::Pointer(Box::new(self.apply(&inner))),
            other => other.clone(),
        }
    }
}

pub struct TypeInferencePass {
    uf: UnionFind,
}

impl Default for TypeInferencePass {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeInferencePass {
    pub fn new() -> Self {
        Self {
            uf: UnionFind::new(),
        }
    }

    fn record(&mut self, ctx: &mut CompilerContext, span: Span, ty: Type) {
        let resolved = self.uf.apply(&ty);
        let id = ctx.symbol_table.intern_type(resolved);
        ctx.types.insert(span.start, id);
    }

    fn unify(&mut self, ctx: &mut CompilerContext, a: Type, b: Type, span: Span) {
        if let Err(e) = self.uf.unify(a, b, span) {
            ctx.type_errors.push(e);
        }
    }

    fn fresh(&self) -> Type {
        Type::Var(TypeVar::fresh())
    }

    fn substitute(&self, ty: &Type, subst: &std::collections::HashMap<TypeVar, Type>) -> Type {
        match ty {
            Type::Var(v) => {
                if let Some(t) = subst.get(v) {
                    t.clone()
                } else {
                    Type::Var(*v)
                }
            }
            Type::List(t) => Type::List(Box::new(self.substitute(t, subst))),
            Type::Dict(k, v) => Type::Dict(
                Box::new(self.substitute(k, subst)),
                Box::new(self.substitute(v, subst)),
            ),
            Type::Optional(t) => Type::Optional(Box::new(self.substitute(t, subst))),
            Type::Result(ok, err) => Type::Result(
                Box::new(self.substitute(ok, subst)),
                Box::new(self.substitute(err, subst)),
            ),
            Type::Function(args, ret) => {
                let args = args.iter().map(|a| self.substitute(a, subst)).collect();
                Type::Function(args, Box::new(self.substitute(ret, subst)))
            }
            Type::Named(n, args) => {
                let args = args.iter().map(|a| self.substitute(a, subst)).collect();
                Type::Named(n.clone(), args)
            }
            Type::Reference(t, m) => Type::Reference(Box::new(self.substitute(t, subst)), *m),
            Type::Pointer(t) => Type::Pointer(Box::new(self.substitute(t, subst))),
            other => other.clone(),
        }
    }

    fn resolve(&self, ty: Type) -> Type {
        self.uf.apply(&ty)
    }

    fn intern(&self, ctx: &mut CompilerContext, ty: Type) -> vinglish_hir::symbol::TypeId {
        let resolved = self.resolve(ty);
        ctx.symbol_table.intern_type(resolved)
    }
}

impl CompilerPass for TypeInferencePass {
    fn run(&mut self, ast: &Module, ctx: &mut CompilerContext) -> Option<HirModule> {
        let mut hir_items = Vec::new();

        // Populate scope with builtins
        // We will seed built-ins directly into SymbolTable later, for now we will assume they exist or handle them inside lookup.

        for item in &ast.items {
            if let Some(hi) = self.infer_item(ctx, item) {
                hir_items.push(hi);
            }
        }

        Some(HirModule { items: hir_items })
    }
}

fn type_expr_to_type(te: &TypeExpr, env: &std::collections::HashMap<String, TypeVar>) -> Type {
    match te {
        TypeExpr::Named(id) => match id.name.as_str() {
            "number" | "integer" | "int" => Type::Int,
            "decimal" | "float" => Type::Float,
            "text" | "string" => Type::Text,
            "boolean" | "bool" => Type::Bool,
            "unit" => Type::Unit,
            other => {
                if let Some(tv) = env.get(other) {
                    Type::Var(*tv)
                } else {
                    Type::Named(other.to_string(), vec![])
                }
            }
        },
        TypeExpr::List(t) => Type::List(Box::new(type_expr_to_type(t, env))),
        TypeExpr::Dict { key, val } => Type::Dict(
            Box::new(type_expr_to_type(key, env)),
            Box::new(type_expr_to_type(val, env)),
        ),
        TypeExpr::Optional(t) => Type::Optional(Box::new(type_expr_to_type(t, env))),
        TypeExpr::Result(t) => {
            Type::Result(Box::new(type_expr_to_type(t, env)), Box::new(Type::Text))
        }
        TypeExpr::Generic { base, args } => {
            if base.name == "address" && args.len() == 1 {
                Type::Pointer(Box::new(type_expr_to_type(&args[0], env)))
            } else {
                Type::Named(
                    base.name.clone(),
                    args.iter().map(|a| type_expr_to_type(a, env)).collect(),
                )
            }
        }
        TypeExpr::Reference { mutable, inner } => {
            Type::Reference(Box::new(type_expr_to_type(inner, env)), *mutable)
        }
    }
}

impl TypeInferencePass {
    fn infer_item(&mut self, ctx: &mut CompilerContext, item: &Item) -> Option<HirItem> {
        match item {
            Item::Function(f) => Some(HirItem::Function(self.infer_function(ctx, f))),
            Item::Statement(s) => Some(HirItem::Statement(self.infer_stmt(ctx, s).1)),
            Item::Type(t) => {
                let env = if let Some(SymbolKind::Type(ts)) = ctx
                    .symbol_table
                    .lookup(&t.name.name)
                    .and_then(|id| ctx.symbol_table.get(id))
                {
                    let mut e = std::collections::HashMap::new();
                    for (i, param) in ts.generic_params.iter().enumerate() {
                        if let Some(ident) = t.type_params.get(i) {
                            e.insert(ident.name.clone(), *param);
                        }
                    }
                    e
                } else {
                    std::collections::HashMap::new()
                };

                let mut hir_fields = Vec::new();
                for f in &t.fields {
                    hir_fields.push(HirParam {
                        id: VariableId(SymbolId(0)), // Fields don't currently have their own independent variable IDs in the scope in the same way, but let's give them anonymous ones or 0
                        name: f.name.name.clone(),
                        ty: self.intern(ctx, type_expr_to_type(&f.ty, &env)),
                        span: f.span,
                    });
                }

                let qualified_name = if ctx.current_module.is_empty() {
                    t.name.name.clone()
                } else {
                    format!("{}.{}", ctx.current_module, t.name.name)
                };
                let id = ctx
                    .symbol_table
                    .lookup(&qualified_name)
                    .map(TypeId)
                    .unwrap_or(TypeId(SymbolId(0)));

                // Update TypeSymbol with fields
                if let Some(ts) = ctx.symbol_table.get_type_mut(id) {
                    for f in &t.fields {
                        ts.add_field(
                            f.name.name.clone(),
                            type_expr_to_type(&f.ty, &env),
                            Visibility::Public,
                        );
                    }
                }

                Some(HirItem::Type(HirTypeDef {
                    visibility: t.visibility,
                    id,
                    name: t.name.name.clone(),
                    fields: hir_fields,
                    span: t.span,
                }))
            }
            Item::Enum(e) => {
                let env = if let Some(SymbolKind::Type(ts)) = ctx
                    .symbol_table
                    .lookup(&e.name.name)
                    .and_then(|id| ctx.symbol_table.get(id))
                {
                    let mut env_map = std::collections::HashMap::new();
                    for (i, param) in ts.generic_params.iter().enumerate() {
                        if let Some(ident) = e.type_params.get(i) {
                            env_map.insert(ident.name.clone(), *param);
                        }
                    }
                    env_map
                } else {
                    std::collections::HashMap::new()
                };

                let qualified_name = if ctx.current_module.is_empty() {
                    e.name.name.clone()
                } else {
                    format!("{}.{}", ctx.current_module, e.name.name)
                };
                let id = ctx
                    .symbol_table
                    .lookup(&qualified_name)
                    .map(TypeId)
                    .unwrap_or(TypeId(SymbolId(0)));

                let mut hir_variants = Vec::new();
                if let Some(ts) = ctx.symbol_table.get_type_mut(id) {
                    ts.add_field("tag".to_string(), Type::Int, Visibility::Public);
                }

                for (index, v) in e.variants.iter().enumerate() {
                    let ty = v.payload.as_ref().map(|ty_expr| {
                        type_expr_to_type(ty_expr, &env)
                    }).unwrap_or(Type::Unit);

                    if let Some(ts) = ctx.symbol_table.get_type_mut(id) {
                        ts.add_field(v.name.name.clone(), ty.clone(), Visibility::Public);
                    }

                    let payload = v.payload.as_ref().map(|_| self.intern(ctx, ty.clone()));

                    
                    hir_variants.push(vinglish_hir::Variant {
                        name: v.name.name.clone(),
                        payload,
                    });

                    // Set constructor function types
                    let variant_name = if ctx.current_module.is_empty() {
                        v.name.name.clone()
                    } else {
                        format!("{}.{}", ctx.current_module, v.name.name)
                    };
                    
                    if let Some(id) = ctx.symbol_table.lookup(&variant_name) {
                        if let Some(fs) = ctx.symbol_table.get_func_mut(vinglish_hir::symbol::FunctionId(id)) {
                            let enum_type = Type::Named(
                                qualified_name.clone(), 
                                fs.generic_params.iter().map(|v| Type::Var(*v)).collect()
                            );
                            
                            if let Some(ref ty_expr) = v.payload {
                                let arg_ty = type_expr_to_type(ty_expr, &env);
                                fs.ty = Type::Function(vec![arg_ty], Box::new(enum_type));
                            } else {
                                fs.ty = Type::Function(vec![], Box::new(enum_type));
                            }
                        }
                    }
                }

                Some(HirItem::Enum(vinglish_hir::EnumDef {
                    visibility: e.visibility,
                    id,
                    name: e.name.name.clone(),
                    variants: hir_variants,
                    span: e.span,
                }))
            }
            _ => None,
        }
    }

    fn infer_function(&mut self, ctx: &mut CompilerContext, f: &FunctionDef) -> HirFunctionDef {
        ctx.push_scope();

        let qualified_name = if ctx.current_module.is_empty() {
            f.name.name.clone()
        } else {
            format!("{}.{}", ctx.current_module, f.name.name)
        };

        let env = if let Some(SymbolKind::Function(fs)) = ctx
            .symbol_table
            .lookup(&qualified_name)
            .and_then(|id| ctx.symbol_table.get(id))
        {
            let mut e = std::collections::HashMap::new();
            for (i, param) in fs.generic_params.iter().enumerate() {
                if let Some(ident) = f.type_params.get(i) {
                    e.insert(ident.name.clone(), *param);
                }
            }
            e
        } else {
            std::collections::HashMap::new()
        };

        let mut hir_params = Vec::new();
        let mut param_types = Vec::new();

        if let Some(target) = &f.target_type {
            let self_ty = Type::Named(target.name.clone(), vec![]);
            let self_id = ctx.symbol_table.define_anon_var(VariableSymbol {
                id: VariableId(SymbolId(0)),
                name: "self".to_string(),
                is_mut: false,
                ty: self_ty.clone(),
            });
            if let Some(vs) = ctx.symbol_table.get_var_mut(self_id) {
                vs.id = self_id;
            }

            ctx.define("self".to_string(), ScopedId::Var(self_id));
            hir_params.push(HirParam {
                id: self_id,
                name: "self".to_string(),
                ty: self.intern(ctx, self_ty.clone()),
                span: f.span,
            });
            param_types.push(self_ty);
        }

        for param in &f.params {
            let ty = type_expr_to_type(&param.ty, &env);
            let param_id = ctx.symbol_table.define_anon_var(VariableSymbol {
                id: VariableId(SymbolId(0)),
                name: param.name.name.clone(),
                is_mut: false,
                ty: ty.clone(),
            });
            if let Some(vs) = ctx.symbol_table.get_var_mut(param_id) {
                vs.id = param_id;
            }

            ctx.define(param.name.name.clone(), ScopedId::Var(param_id));
            self.record(ctx, param.name.span, ty.clone());
            hir_params.push(HirParam {
                id: param_id,
                name: param.name.name.clone(),
                ty: self.intern(ctx, ty.clone()),
                span: param.span,
            });
            param_types.push(ty);
        }

        let expected_ret = f
            .ret_type
            .as_ref()
            .map(|t| type_expr_to_type(t, &env))
            .unwrap_or_else(|| self.fresh());

        let fn_ty = Type::Function(param_types.clone(), Box::new(expected_ret.clone()));

        // Update the function symbol's type in the symbol table so recursive calls work
        if f.target_type.is_none() {
            if let Some(sym_id) = ctx.symbol_table.lookup(&f.name.name) {
                if let Some(vinglish_hir::symbol::SymbolKind::Function(_)) = ctx.symbol_table.get(sym_id)
                {
                    let func_id = vinglish_hir::symbol::FunctionId(sym_id);
                    if let Some(fs) = ctx.symbol_table.get_func_mut(func_id) {
                        fs.ty = fn_ty.clone();
                    }
                }
            }
        }

        let prev_ret_ty = ctx.current_return_type.take();
        ctx.current_return_type = Some(expected_ret.clone());

        let (actual_ret, hir_body) = self.infer_block(ctx, &f.body);

        ctx.current_return_type = prev_ret_ty;

        // If the block returns a value implicitly, unify it.
        // If it's Unit, we assume explicit `return` statements provided the value (we don't have exhaustiveness checking yet).
        let actual_resolved = self.resolve(actual_ret.clone());
        let expected_resolved = self.resolve(expected_ret.clone());

        if !matches!(actual_resolved, Type::Unit) || matches!(expected_resolved, Type::Unit) {
            self.unify(ctx, expected_ret.clone(), actual_ret, f.span);
        }

        ctx.pop_scope();
        self.record(ctx, f.name.span, Type::Named(f.name.name.clone(), vec![]));

        let hir_name = if let Some(target) = &f.target_type {
            format!("{}_{}", target.name, f.name.name) // Lowered name format
        } else {
            f.name.name.clone()
        };

        let fn_ty = Type::Function(param_types, Box::new(expected_ret.clone()));

        // We look up the existing FunctionSymbol if it's not a method, otherwise we define a new one or add to struct
        let func_id = if let Some(target) = &f.target_type {
            // It's a method
            let method_id = ctx.symbol_table.define_func(
                hir_name.clone(),
                FunctionSymbol {
                    id: FunctionId(SymbolId(0)),
                    name: hir_name.clone(),
                    visibility: f.visibility,
                    ty: fn_ty.clone(),
                    generic_params: vec![],
                    is_variant_constructor: None,
                },
            );
            if let Some(fs) = ctx.symbol_table.get_func_mut(method_id) {
                fs.id = method_id;
            }

            // Add method to TypeSymbol
            let type_id_opt = ctx.lookup(&target.name);
            if let Some(type_id) = type_id_opt {
                if let Some(ts) = ctx.symbol_table.get_type_mut(type_id.as_type().unwrap()) {
                    ts.add_method(f.name.name.clone(), method_id);
                }
            }
            method_id
        } else {
            ctx.lookup(&f.name.name).unwrap().as_func().unwrap()
        };

        if let Some(fs) = ctx.symbol_table.get_func_mut(func_id) {
            fs.ty = fn_ty.clone();
        }

        HirFunctionDef {
            visibility: f.visibility,
            is_foreign: f.is_foreign,
            id: func_id,
            name: hir_name,
            params: hir_params,
            ret_ty: self.intern(ctx, expected_ret),
            body: HirExpr::Block(hir_body),
            span: f.span,
        }
    }

    fn infer_block(&mut self, ctx: &mut CompilerContext, block: &Block) -> (Type, HirBlock) {
        ctx.push_scope();
        let mut last = Type::Unit;
        let mut hir_stmts = Vec::new();

        for stmt in &block.stmts {
            let (ty, hir_s) = self.infer_stmt(ctx, stmt);
            last = ty;
            hir_stmts.push(hir_s);
        }

        ctx.pop_scope();

        let mut block_expr = None;
        if let Some(HirStmt::Expr(e)) = hir_stmts.last() {
            block_expr = Some(Box::new(e.clone()));
            hir_stmts.pop();
        }

        let hir_block = HirBlock {
            stmts: hir_stmts,
            expr: block_expr,
            ty: self.intern(ctx, last.clone()),
            span: block.span,
        };

        (last, hir_block)
    }

    fn infer_stmt(&mut self, ctx: &mut CompilerContext, stmt: &Stmt) -> (Type, HirStmt) {
        match stmt {
            Stmt::Let(let_stmt) => {
                let ty = if let Some(te) = &let_stmt.ty {
                    type_expr_to_type(te, &std::collections::HashMap::new())
                } else {
                    self.fresh()
                };

                let mut hir_init = HirExpr::Lit {
                    value: Literal::Unit,
                    ty: self.intern(ctx, Type::Unit),
                    span: let_stmt.span,
                };
                if let Some(expr) = &let_stmt.value {
                    let (expr_ty, h) = self.infer_expr(ctx, expr);
                    self.unify(ctx, ty.clone(), expr_ty, let_stmt.span);
                    hir_init = h;
                }

                let resolved = self.resolve(ty.clone());

                let id = ctx.symbol_table.define_anon_var(VariableSymbol {
                    id: VariableId(SymbolId(0)),
                    name: let_stmt.name.name.clone(),
                    is_mut: let_stmt.mutable,
                    ty: resolved.clone(),
                });
                if let Some(vs) = ctx.symbol_table.get_var_mut(id) {
                    vs.id = id;
                }

                ctx.define(let_stmt.name.name.clone(), ScopedId::Var(id));
                self.record(ctx, let_stmt.name.span, resolved.clone());

                (
                    Type::Unit,
                    HirStmt::Let {
                        id,
                        name: let_stmt.name.name.clone(),
                        is_mut: let_stmt.mutable,
                        ty: self.intern(ctx, resolved),
                        init: hir_init,
                        span: let_stmt.span,
                    },
                )
            }
            Stmt::Assign(a) => {
                let (target_ty, ht) = self.infer_expr(ctx, &a.target);
                let (value_ty, hv) = self.infer_expr(ctx, &a.value);
                self.unify(ctx, target_ty.clone(), value_ty, a.span);
                (
                    Type::Unit,
                    HirStmt::Assign {
                        target: ht,
                        op: a.op,
                        value: hv,
                        span: a.span,
                    },
                )
            }
            Stmt::Return(r) => {
                let mut hir_val = None;
                let val_ty = if let Some(expr) = &r.value {
                    let (ty, hv) = self.infer_expr(ctx, expr);
                    hir_val = Some(hv);
                    ty
                } else {
                    Type::Unit
                };

                if let Some(expected) = ctx.current_return_type.clone() {
                    self.unify(ctx, expected, val_ty, r.span);
                }

                (
                    Type::Unit,
                    HirStmt::Return {
                        value: hir_val,
                        span: r.span,
                    },
                )
            }
            Stmt::If(if_stmt) => {
                let (cond_ty, hc) = self.infer_expr(ctx, &if_stmt.condition);
                self.unify(ctx, cond_ty, Type::Bool, if_stmt.condition.span());
                let (then_ty, ht) = self.infer_block(ctx, &if_stmt.then_block);
                let mut ho = None;
                if let Some(else_block) = &if_stmt.otherwise {
                    let (else_ty, he) = self.infer_block(ctx, else_block);
                    self.unify(ctx, then_ty.clone(), else_ty, if_stmt.span);
                    ho = Some(he);
                }
                (
                    then_ty,
                    HirStmt::If {
                        condition: hc,
                        then_block: ht,
                        otherwise: ho,
                        span: if_stmt.span,
                    },
                )
            }
            Stmt::Repeat(RepeatStmt::While {
                condition,
                body,
                span,
            }) => {
                let (cond_ty, hc) = self.infer_expr(ctx, condition);
                self.unify(ctx, cond_ty, Type::Bool, condition.span());
                let (_, hb) = self.infer_block(ctx, body);
                (
                    Type::Unit,
                    HirStmt::RepeatWhile {
                        condition: hc,
                        body: hb,
                        span: *span,
                    },
                )
            }
            Stmt::Expr(e) => {
                let (ty, hir_expr) = self.infer_expr(ctx, e);
                (ty, HirStmt::Expr(hir_expr))
            }
            _ => (
                Type::Unit,
                HirStmt::Expr(HirExpr::Lit {
                    value: Literal::Unit,
                    ty: self.intern(ctx, Type::Unit),
                    span: Span::dummy(),
                }),
            ),
        }
    }

    fn infer_expr(&mut self, ctx: &mut CompilerContext, expr: &Expr) -> (Type, HirExpr) {
        match expr {
            Expr::Lit { value, span } => {
                let ty = match value {
                    Literal::Int(_) => Type::Int,
                    Literal::Float(_) => Type::Float,
                    Literal::Bool(_) => Type::Bool,
                    Literal::Text(_) => Type::Text,
                    Literal::Unit => Type::Unit,
                };
                (
                    ty.clone(),
                    HirExpr::Lit {
                        value: value.clone(),
                        ty: self.intern(ctx, ty),
                        span: *span,
                    },
                )
            }
            Expr::Ident(id) => {
                if let Some(symbol_id) = ctx.lookup(&id.name) {
                    let mut ty = self.fresh();
                    if let Some(vs) = ctx
                        .symbol_table
                        .get_var(symbol_id.as_var().unwrap_or(VariableId(SymbolId(0))))
                    {
                        ty = vs.ty.clone(); // In HM this should be instantiated if it's a let-scheme, but for now we clone
                    } else if let Some(fs) = ctx
                        .symbol_table
                        .get_func(symbol_id.as_func().unwrap_or(FunctionId(SymbolId(0))))
                    {
                        ty = fs.ty.clone();
                    }
                    // For let-polymorphism, we'd need a TypeScheme in the SymbolTable, but we'll stick to mono types for variables right now, and generalize builtins manually.

                    self.record(ctx, id.span, ty.clone());
                    (
                        ty.clone(),
                        HirExpr::VarRef {
                            id: VariableId(symbol_id.as_raw_id()),
                            ty: self.intern(ctx, ty.clone()),
                            span: id.span,
                        },
                    )
                } else {
                    let fresh = self.fresh();
                    ctx.type_errors.push(TypeError::new(
                        format!("unknown identifier `{}`", id.name),
                        id.span,
                    ));
                    (
                        fresh.clone(),
                        HirExpr::VarRef {
                            id: VariableId(SymbolId(0)),
                            ty: self.intern(ctx, fresh.clone()),
                            span: id.span,
                        },
                    )
                }
            }
            Expr::GenericInst { base, args, span } => {
                if let Some(symbol_id) = ctx.lookup(&base.name) {
                    let mut ty;
                    if let Some(fs) = ctx
                        .symbol_table
                        .get_func(symbol_id.as_func().unwrap_or(FunctionId(SymbolId(0))))
                    {
                        ty = fs.ty.clone();

                        // Substitute generic parameters!
                        let provided_tys: Vec<Type> = args
                            .iter()
                            .map(|a| type_expr_to_type(a, &std::collections::HashMap::new()))
                            .collect();
                        if provided_tys.len() == fs.generic_params.len() {
                            let mut subst = std::collections::HashMap::new();
                            for (i, param) in fs.generic_params.iter().enumerate() {
                                subst.insert(*param, provided_tys[i].clone());
                            }
                            ty = self.substitute(&ty, &subst);
                        } else {
                            ctx.type_errors.push(TypeError::new(
                                format!(
                                    "expected {} generic arguments, got {}",
                                    fs.generic_params.len(),
                                    provided_tys.len()
                                ),
                                *span,
                            ));
                        }
                    } else {
                        // Types or vars with generics not supported in expressions yet
                        ty = self.fresh();
                    }

                    self.record(ctx, *span, ty.clone());
                    (
                        ty.clone(),
                        HirExpr::VarRef {
                            id: VariableId(symbol_id.as_raw_id()),
                            ty: self.intern(ctx, ty.clone()),
                            span: *span,
                        },
                    )
                } else {
                    let fresh = self.fresh();
                    ctx.type_errors.push(TypeError::new(
                        format!("unknown identifier `{}`", base.name),
                        *span,
                    ));
                    (
                        fresh.clone(),
                        HirExpr::VarRef {
                            id: VariableId(SymbolId(0)),
                            ty: self.intern(ctx, fresh.clone()),
                            span: *span,
                        },
                    )
                }
            }
            Expr::Call { callee, args, span } => {
                // Intercept `Ok` and `Err` as built-in constructors
                if let Expr::Ident(id) = &**callee {
                    if id.name == "Ok" || id.name == "Err" {
                        let mut hir_args = Vec::new();
                        let mut arg_tys = Vec::new();
                        for a in args {
                            let (aty, ha) = self.infer_expr(ctx, a);
                            arg_tys.push(aty);
                            hir_args.push(ha);
                        }
                        if arg_tys.len() == 1 {
                            let result_ty = if id.name == "Ok" {
                                Type::Result(Box::new(arg_tys[0].clone()), Box::new(self.fresh()))
                            } else {
                                Type::Result(Box::new(self.fresh()), Box::new(arg_tys[0].clone()))
                            };
                            self.record(ctx, *span, result_ty.clone());
                            return (
                                result_ty.clone(),
                                HirExpr::MacroCall {
                                    name: id.name.clone(),
                                    args: hir_args,
                                    ty: self.intern(ctx, result_ty),
                                    span: *span,
                                }
                            );
                        } else {
                            ctx.type_errors.push(TypeError::new(format!("{} expects exactly 1 argument", id.name), *span));
                        }
                    }
                }

                let (callee_ty, mut hir_callee) = self.infer_expr(ctx, callee);
                let mut arg_tys = Vec::new();
                let mut hir_args = Vec::new();

                for a in args {
                    let (aty, ha) = self.infer_expr(ctx, a);
                    arg_tys.push(aty);
                    hir_args.push(ha);
                }

                let ret_ty = self.fresh();

                // Method call resolution: if callee is a Field that resolved to a method,
                // the parser AST gives us `Field { object, field }` as the callee.
                if let Expr::Field { object, field, .. } = &**callee {
                    let (obj_ty, hir_obj) = self.infer_expr(ctx, object);
                    let resolved_obj = self.uf.apply(&obj_ty);
                    if let Type::Named(name, _) = &resolved_obj {
                        let type_id_opt = ctx.symbol_table.lookup(name);
                        if let Some(type_id) = type_id_opt {
                            let mut is_field = false;
                            if let Some(SymbolKind::Type(ts)) = ctx.symbol_table.get(type_id) {
                                is_field = ts.fields.iter().any(|f| f.name == field.name);
                            }

                            if !is_field {
                                let method_name = format!("{}_{}", name, field.name);
                                if let Some(method_id) = ctx.lookup(&method_name) {
                                    // It's a method! Flatten it.
                                    hir_args.insert(0, hir_obj);
                                    let mut m_ty = self.fresh();
                                    if let Some(fs) = ctx.symbol_table.get_func(
                                        method_id.as_func().unwrap_or(FunctionId(SymbolId(0))),
                                    ) {
                                        m_ty = fs.ty.clone(); // Instantiation needed in true HM
                                    }

                                    hir_callee = HirExpr::VarRef {
                                        id: VariableId(
                                            method_id
                                                .as_func()
                                                .unwrap_or(FunctionId(SymbolId(0)))
                                                .0,
                                        ),
                                        ty: self.intern(ctx, m_ty.clone().clone()),
                                        span: field.span,
                                    };
                                    let mut method_arg_tys = arg_tys.clone();
                                    method_arg_tys.insert(0, obj_ty);
                                    self.unify(
                                        ctx,
                                        m_ty,
                                        Type::Function(method_arg_tys, Box::new(ret_ty.clone())),
                                        *span,
                                    );
                                    self.record(ctx, *span, ret_ty.clone());
                                    return (
                                        ret_ty.clone(),
                                        HirExpr::Call {
                                            callee: Box::new(hir_callee),
                                            args: hir_args,
                                            ty: self.intern(ctx, ret_ty.clone()),
                                            span: *span,
                                        },
                                    );
                                }
                            }
                        }
                    }
                }

                let expected_fn_ty = Type::Function(arg_tys, Box::new(ret_ty.clone()));
                self.unify(ctx, callee_ty, expected_fn_ty, *span);
                self.record(ctx, *span, ret_ty.clone());
                (
                    ret_ty.clone(),
                    HirExpr::Call {
                        callee: Box::new(hir_callee),
                        args: hir_args,
                        ty: self.intern(ctx, ret_ty.clone()),
                        span: *span,
                    },
                )
            }
            Expr::BinOp {
                left,
                op,
                right,
                span,
            } => {
                let (lt, hl) = self.infer_expr(ctx, left);
                let (rt, hr) = self.infer_expr(ctx, right);
                let result = match op {
                    BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod => {
                        self.unify(ctx, lt.clone(), rt, *span);
                        lt
                    }
                    BinOp::Eq
                    | BinOp::NotEq
                    | BinOp::Lt
                    | BinOp::Gt
                    | BinOp::LtEq
                    | BinOp::GtEq
                    | BinOp::IsBelow
                    | BinOp::IsAbove
                    | BinOp::Exceeds => {
                        self.unify(ctx, lt, rt, *span);
                        Type::Bool
                    }
                    BinOp::And | BinOp::Or => {
                        self.unify(ctx, lt, Type::Bool, *span);
                        self.unify(ctx, rt, Type::Bool, *span);
                        Type::Bool
                    }
                };
                self.record(ctx, *span, result.clone());
                (
                    result.clone(),
                    HirExpr::BinOp {
                        left: Box::new(hl),
                        op: *op,
                        right: Box::new(hr),
                        ty: self.intern(ctx, result.clone()),
                        span: *span,
                    },
                )
            }
            Expr::Field {
                object,
                field,
                span,
            } => {
                if let Some(path_str) = get_path_string(expr) {
                    if let Some(scoped_id) = ctx.lookup(&path_str) {
                        let ty = match scoped_id {
                            ScopedId::Func(id) => {
                                if let Some(fs) = ctx.symbol_table.get_func(id) {
                                    fs.ty.clone()
                                } else {
                                    self.fresh()
                                }
                            }
                            ScopedId::Type(id) => Type::Named(
                                ctx.symbol_table.get_type(id).unwrap().name.clone(),
                                vec![],
                            ),
                            ScopedId::Var(id) => {
                                if let Some(vs) = ctx.symbol_table.get_var(id) {
                                    vs.ty.clone()
                                } else {
                                    self.fresh()
                                }
                            }
                        };
                        self.record(ctx, *span, ty.clone());
                        let hir_expr = match scoped_id {
                            ScopedId::Func(id) => HirExpr::VarRef {
                                id: VariableId(id.0),
                                ty: self.intern(ctx, ty.clone()),
                                span: *span,
                            },
                            ScopedId::Type(id) => HirExpr::VarRef {
                                id: VariableId(id.0),
                                ty: self.intern(ctx, ty.clone()),
                                span: *span,
                            },
                            ScopedId::Var(id) => HirExpr::VarRef {
                                id,
                                ty: self.intern(ctx, ty.clone()),
                                span: *span,
                            },
                        };
                        return (ty, hir_expr);
                    }
                }

                let (obj_ty, hir_obj) = self.infer_expr(ctx, object);
                let result_ty = self.fresh();
                let mut index = 0;

                let mut resolved_obj = self.uf.apply(&obj_ty);
                while let Type::Reference(inner, _) = resolved_obj {
                    resolved_obj = *inner;
                }
                if let Type::Named(name, _) = &resolved_obj {
                    let type_id_opt = ctx
                        .lookup(name)
                        .and_then(|id| id.as_type())
                        .or_else(|| ctx.symbol_table.lookup(name).map(TypeId));

                    if let Some(type_id) = type_id_opt {
                        let mut field_found = None;
                        let mut best_match = None;
                        if let Some(SymbolKind::Type(symbol)) = ctx.symbol_table.get(type_id.0) {
                            if let Some(f_sym) = symbol.get_field(&field.name) {
                                field_found = Some((f_sym.ty.clone(), f_sym.id.0));
                            } else {
                                // Diagnostic: Suggest intent-aware typos
                                let mut min_dist = usize::MAX;
                                for f_sym in &symbol.fields {
                                    let dist = levenshtein(&f_sym.name, &field.name);
                                    if dist < 3 && dist < min_dist {
                                        min_dist = dist;
                                        best_match = Some(f_sym.name.clone());
                                    }
                                }
                            }
                        }

                        if let Some((fty, fidx)) = field_found {
                            self.unify(ctx, result_ty.clone(), fty, *span);
                            index = fidx;
                        } else {
                            let method_name = format!("{}_{}", name, field.name);
                            if ctx.lookup(&method_name).is_none() {
                                let msg = if let Some(suggestion) = best_match {
                                    format!(
                                        "struct `{}` has no field `{}`. Did you mean `{}`?",
                                        name, field.name, suggestion
                                    )
                                } else {
                                    format!("struct `{}` has no field `{}`", name, field.name)
                                };

                                ctx.type_errors.push(TypeError::new(msg, field.span));
                            }
                        }
                    }
                }

                self.record(ctx, *span, result_ty.clone());
                (
                    result_ty.clone(),
                    HirExpr::FieldIndex {
                        object: Box::new(hir_obj),
                        field_id: FieldId(index),
                        ty: self.intern(ctx, result_ty.clone()),
                        span: *span,
                    },
                )
            }
            Expr::StructLit { ty, fields, span } => {
                let (name, mut struct_ty) = match &**ty {
                    Expr::Ident(ident) => (ident.clone(), Type::Named(ident.name.clone(), vec![])),
                    Expr::GenericInst { base, args, .. } => {
                        let ty_args: Vec<Type> = args
                            .iter()
                            .map(|arg| type_expr_to_type(arg, &HashMap::new()))
                            .collect();
                        (base.clone(), Type::Named(base.name.clone(), ty_args))
                    }
                    _ => panic!("Invalid type expression in struct literal"),
                };

                let mut hir_fields = Vec::new();
                let type_id_opt = ctx.lookup(&name.name).and_then(|id| id.as_type());

                if let Some(type_id) = type_id_opt {
                    if let Some(SymbolKind::Type(symbol)) = ctx.symbol_table.get(type_id.0).cloned()
                    {
                        // Generics substitution map for fields
                        let mut subst = HashMap::new();
                        if let Type::Named(_, ty_args) = &struct_ty {
                            for (i, param) in symbol.generic_params.iter().enumerate() {
                                if let Some(arg_ty) = ty_args.get(i) {
                                    subst.insert(*param, arg_ty.clone());
                                } else {
                                    let f = self.fresh();
                                    subst.insert(*param, f);
                                }
                            }
                            if ty_args.is_empty() && !symbol.generic_params.is_empty() {
                                // If not provided, inject fresh inference variables
                                let mut fresh_args = Vec::new();
                                for param in &symbol.generic_params {
                                    let f = self.fresh();
                                    subst.insert(*param, f.clone());
                                    fresh_args.push(f);
                                }
                                struct_ty = Type::Named(name.name.clone(), fresh_args);
                            }
                        }

                        // Constructor Validation
                        let mut provided = HashMap::new();
                        for (fname, fexpr) in fields {
                            let (fty, hf) = self.infer_expr(ctx, fexpr);
                            if let Some(f_sym) = symbol.get_field(&fname.name) {
                                let mut expected_fty = f_sym.ty.clone();
                                if !subst.is_empty() {
                                    expected_fty = self.substitute(&expected_fty, &subst);
                                }
                                self.unify(ctx, fty, expected_fty, fexpr.span());
                                provided.insert(f_sym.id, hf);
                            } else {
                                // Check typos
                                let mut best_match = None;
                                let mut min_dist = usize::MAX;
                                for f_sym in &symbol.fields {
                                    let dist = levenshtein(&f_sym.name, &fname.name);
                                    if dist < 3 && dist < min_dist {
                                        min_dist = dist;
                                        best_match = Some(f_sym.name.clone());
                                    }
                                }
                                let msg = if let Some(suggestion) = best_match {
                                    format!(
                                        "struct `{}` has no field `{}`. Did you mean `{}`?",
                                        name.name, fname.name, suggestion
                                    )
                                } else {
                                    format!("struct `{}` has no field `{}`", name.name, fname.name)
                                };
                                ctx.type_errors.push(TypeError::new(msg, fname.span));
                            }
                        }

                        // Ensure all fields provided in correct order
                        for f_sym in &symbol.fields {
                            if let Some(hf) = provided.remove(&f_sym.id) {
                                hir_fields.push(hf);
                            } else {
                                ctx.type_errors.push(TypeError::new(
                                    format!(
                                        "missing field `{}` in constructor for `{}`",
                                        f_sym.name, name.name
                                    ),
                                    *span,
                                ));
                                hir_fields.push(HirExpr::Lit {
                                    value: Literal::Unit,
                                    ty: self.intern(ctx, Type::Unit),
                                    span: Span::dummy(),
                                }); // Recover
                            }
                        }
                    }
                } else {
                    ctx.type_errors.push(TypeError::new(
                        format!("unknown struct `{}`", name.name),
                        name.span,
                    ));
                }
                self.record(ctx, *span, struct_ty.clone());
                (
                    struct_ty.clone(),
                    HirExpr::StructInit {
                        id: type_id_opt.unwrap_or(TypeId(SymbolId(0))),
                        fields: hir_fields,
                        ty: self.intern(ctx, struct_ty.clone()),
                        span: *span,
                    },
                )
            }
            Expr::UnOp { op, operand, span } => {
                let (inner, hi) = self.infer_expr(ctx, operand);
                let result = match op {
                    UnOp::Neg => inner,
                    UnOp::Not => {
                        self.unify(ctx, inner, Type::Bool, *span);
                        Type::Bool
                    }
                    UnOp::Borrow(mutable) => Type::Reference(Box::new(inner), *mutable),
                    UnOp::Deref => {
                        let inner_ty = self.fresh();
                        if let Type::Reference(ref t, _) = self.uf.apply(&inner) {
                            *t.clone()
                        } else {
                            self.unify(
                                ctx,
                                inner,
                                Type::Reference(Box::new(inner_ty.clone()), false),
                                *span,
                            );
                            inner_ty
                        }
                    }
                };
                self.record(ctx, *span, result.clone());
                (
                    result.clone(),
                    HirExpr::UnOp {
                        op: *op,
                        operand: Box::new(hi),
                        ty: self.intern(ctx, result.clone()),
                        span: *span,
                    },
                )
            }
            Expr::Index {
                object,
                index,
                span,
            } => {
                let (obj_ty, ho) = self.infer_expr(ctx, object);
                let (idx_ty, hi) = self.infer_expr(ctx, index);
                self.unify(ctx, idx_ty, Type::Int, *span);
                let elem_ty = self.fresh();
                self.unify(ctx, obj_ty, Type::List(Box::new(elem_ty.clone())), *span);
                self.record(ctx, *span, elem_ty.clone());
                (
                    elem_ty.clone(),
                    HirExpr::Index {
                        object: Box::new(ho),
                        index: Box::new(hi),
                        ty: self.intern(ctx, elem_ty.clone()),
                        span: *span,
                    },
                )
            }
            Expr::List { elements, span } => {
                let elem_ty = self.fresh();
                let mut hir_elems = Vec::new();
                for e in elements {
                    let (et, he) = self.infer_expr(ctx, e);
                    self.unify(ctx, elem_ty.clone(), et, e.span());
                    hir_elems.push(he);
                }
                let list_ty = Type::List(Box::new(elem_ty));
                self.record(ctx, *span, list_ty.clone());
                (
                    list_ty.clone(),
                    HirExpr::List {
                        elements: hir_elems,
                        ty: self.intern(ctx, list_ty.clone()),
                        span: *span,
                    },
                )
            }
            Expr::Block(block) => {
                let (ty, hb) = self.infer_block(ctx, block);
                (ty.clone(), HirExpr::Block(hb))
            }
            Expr::MacroCall { name, args, span } => {
                let mut hir_args = Vec::new();
                for a in args {
                    let (_, ha) = self.infer_expr(ctx, a);
                    hir_args.push(ha);
                }
                
                let ret_ty = if name.name == "fmt" {
                    Type::Text
                } else {
                    ctx.type_errors.push(TypeError::new(format!("Unknown macro: {}!", name.name), *span));
                    Type::Unit
                };

                self.record(ctx, *span, ret_ty.clone());
                (
                    ret_ty.clone(),
                    HirExpr::MacroCall {
                        name: name.name.clone(),
                        args: hir_args,
                        ty: self.intern(ctx, ret_ty.clone()),
                        span: *span,
                    }
                )
            }
            Expr::PostfixTry { inner, span } => {
                let (inner_ty, hir_inner) = self.infer_expr(ctx, inner);
                let ok_ty = self.fresh();
                let err_ty = self.fresh();
                
                self.unify(ctx, inner_ty.clone(), Type::Result(Box::new(ok_ty.clone()), Box::new(err_ty.clone())), *span);

                if let Some(expected_ret) = ctx.current_return_type.clone() {
                    let expected_ok = self.fresh();
                    self.unify(ctx, expected_ret, Type::Result(Box::new(expected_ok), Box::new(err_ty.clone())), *span);
                } else {
                    ctx.type_errors.push(TypeError::new("Cannot use `?` outside of a function returning Result", *span));
                }

                self.record(ctx, *span, ok_ty.clone());
                (
                    ok_ty.clone(),
                    HirExpr::PostfixTry {
                        inner: Box::new(hir_inner),
                        ty: self.intern(ctx, ok_ty.clone()),
                        span: *span,
                    }
                )
            }
        }
    }
}

fn get_path_string(expr: &Expr) -> Option<String> {
    match expr {
        Expr::Ident(id) => Some(id.name.clone()),
        Expr::Field { object, field, .. } => {
            let obj_str = get_path_string(object)?;
            Some(format!("{}.{}", obj_str, field.name))
        }
        _ => None,
    }
}

pub fn infer_module(ast: &Module) -> (SymbolTable, Vec<TypeError>, HirModule) {
    let mut ctx = CompilerContext::new();

    let mut name_pass = crate::passes::NameResolutionPass;
    name_pass.run(ast, &mut ctx);

    let mut type_pass = TypeInferencePass::new();
    let hir = type_pass
        .run(ast, &mut ctx)
        .unwrap_or_else(|| HirModule { items: vec![] });

    let validator = crate::validator::HirValidatorPass::new();
    validator.validate(&mut ctx, &hir);

    (ctx.symbol_table, ctx.type_errors, hir)
}
