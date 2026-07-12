use eng_hir::Module as HirModule;
use eng_hir::Item as HirItem;
use eng_hir::Expr as HirExpr;
use eng_hir::Stmt as HirStmt;
use crate::passes::{CompilerContext, CompilerPass};
use crate::TypeError;

pub struct HirValidatorPass;

impl Default for HirValidatorPass {
    fn default() -> Self {
        Self::new()
    }
}

impl HirValidatorPass {
    pub fn new() -> Self {
        Self
    }

    fn validate_item(&self, ctx: &mut CompilerContext, item: &HirItem) {
        match item {
            HirItem::Function(f) => {
                if ctx.symbol_table.get_func(f.id).is_none() {
                    ctx.type_errors.push(TypeError::new(
                        format!("Function `{}` has invalid ID {:?}", f.name, f.id),
                        f.span,
                    ));
                }
                for param in &f.params {
                    if ctx.symbol_table.get_var(param.id).is_none() {
                        ctx.type_errors.push(TypeError::new(
                            format!("Parameter `{}` has invalid ID {:?}", param.name, param.id),
                            param.span,
                        ));
                    }
                    if ctx.symbol_table.get(param.ty.0).is_none() {
                        ctx.type_errors.push(TypeError::new(
                            format!("Parameter `{}` has invalid TypeId {:?}", param.name, param.ty),
                            param.span,
                        ));
                    }
                }
                if ctx.symbol_table.get(f.ret_ty.0).is_none() {
                    ctx.type_errors.push(TypeError::new(
                        format!("Function `{}` has invalid return TypeId {:?}", f.name, f.ret_ty),
                        f.span,
                    ));
                }
                self.validate_expr(ctx, &f.body);
            }
            HirItem::Type(t) => {
                if ctx.symbol_table.get_type(t.id).is_none() {
                    ctx.type_errors.push(TypeError::new(
                        format!("Type `{}` has invalid ID {:?}", t.name, t.id),
                        t.span,
                    ));
                }
                for field in &t.fields {
                    if ctx.symbol_table.get(field.ty.0).is_none() {
                        ctx.type_errors.push(TypeError::new(
                            format!("Field `{}` has invalid TypeId {:?}", field.name, field.ty),
                            field.span,
                        ));
                    }
                }
            }
            HirItem::Statement(s) => {
                self.validate_stmt(ctx, s);
            }
        }
    }

    fn validate_stmt(&self, ctx: &mut CompilerContext, stmt: &HirStmt) {
        match stmt {
            HirStmt::Let { id, ty, init, span, .. } => {
                if ctx.symbol_table.get_var(*id).is_none() {
                    ctx.type_errors.push(TypeError::new(
                        format!("Let statement has invalid VariableId {:?}", id),
                        *span,
                    ));
                }
                if ctx.symbol_table.get(ty.0).is_none() {
                    ctx.type_errors.push(TypeError::new(
                        format!("Let statement has invalid TypeId {:?}", ty),
                        *span,
                    ));
                }
                self.validate_expr(ctx, init);
            }
            HirStmt::Assign { target, value, .. } => {
                self.validate_expr(ctx, target);
                self.validate_expr(ctx, value);
            }
            HirStmt::If { condition, then_block, otherwise, .. } => {
                self.validate_expr(ctx, condition);
                for s in &then_block.stmts {
                    self.validate_stmt(ctx, s);
                }
                if let Some(e) = &then_block.expr {
                    self.validate_expr(ctx, e);
                }
                if let Some(other) = otherwise {
                    for s in &other.stmts {
                        self.validate_stmt(ctx, s);
                    }
                    if let Some(e) = &other.expr {
                        self.validate_expr(ctx, e);
                    }
                }
            }
            HirStmt::RepeatWhile { condition, body, .. } => {
                self.validate_expr(ctx, condition);
                for s in &body.stmts {
                    self.validate_stmt(ctx, s);
                }
                if let Some(e) = &body.expr {
                    self.validate_expr(ctx, e);
                }
            }
            HirStmt::Return { value, .. } => {
                if let Some(v) = value {
                    self.validate_expr(ctx, v);
                }
            }
            HirStmt::Expr(e) => {
                self.validate_expr(ctx, e);
            }
        }
    }

    fn validate_expr(&self, ctx: &mut CompilerContext, expr: &HirExpr) {
        // Validate the `ty` field of every expression
        let ty_id = expr.ty();
        if ctx.symbol_table.get(ty_id.0).is_none() {
            ctx.type_errors.push(TypeError::new(
                format!("Expression has invalid TypeId {:?}", ty_id),
                expr.span(),
            ));
        }

        match expr {
            HirExpr::Lit { .. } => {}
            HirExpr::VarRef { id, span, .. } => {
                if ctx.symbol_table.get_var(*id).is_none()
                    && ctx.symbol_table.get_func(eng_hir::symbol::FunctionId(id.0)).is_none() {
                        ctx.type_errors.push(TypeError::new(
                            format!("VarRef has invalid ID {:?}", id),
                            *span,
                        ));
                    }
            }
            HirExpr::Call { callee, args, .. } => {
                self.validate_expr(ctx, callee);
                for arg in args {
                    self.validate_expr(ctx, arg);
                }
            }
            HirExpr::BinOp { left, right, .. } => {
                self.validate_expr(ctx, left);
                self.validate_expr(ctx, right);
            }
            HirExpr::UnOp { operand, .. } => {
                self.validate_expr(ctx, operand);
            }
            HirExpr::Block(b) => {
                for stmt in &b.stmts {
                    self.validate_stmt(ctx, stmt);
                }
                if let Some(e) = &b.expr {
                    self.validate_expr(ctx, e);
                }
            }
            HirExpr::StructInit { fields, .. } => {
                for fexpr in fields {
                    self.validate_expr(ctx, fexpr);
                }
            }
            HirExpr::FieldIndex { object, .. } => {
                self.validate_expr(ctx, object);
            }
            HirExpr::Index { object, index, .. } => {
                self.validate_expr(ctx, object);
                self.validate_expr(ctx, index);
            }
            HirExpr::List { elements, .. } => {
                for e in elements {
                    self.validate_expr(ctx, e);
                }
            }
        }
    }
}

impl CompilerPass for HirValidatorPass {
    fn run(&mut self, _ast: &eng_parser::ast::Module, _ctx: &mut CompilerContext) -> Option<HirModule> {
        // HirValidatorPass expects an already compiled HirModule, so it cannot run as a standard ast pass directly.
        // It should be invoked separately on the generated HIR.
        None
    }
}

impl HirValidatorPass {
    pub fn validate(&self, ctx: &mut CompilerContext, hir: &HirModule) {
        for item in &hir.items {
            self.validate_item(ctx, item);
        }
    }
}
