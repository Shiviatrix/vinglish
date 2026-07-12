import re

with open("crates/eng-types/src/type_pass.rs", "r") as f:
    code = f.read()

# Fix Expr::Index
code = code.replace("Expr::Index { object, field_id: FieldId(index), span }", "Expr::Index { object, index, span }")

# Fix method lookups
code = code.replace("ctx.symbol_table.get(method_id)", "ctx.symbol_table.get(method_id.0)")
code = code.replace("ctx.symbol_table.get_func(method_id)", "ctx.symbol_table.get(method_id.0)")

# Fix ScopedId errors
code = code.replace("ctx.symbol_table.get(symbol_id)", "ctx.symbol_table.get(symbol_id.as_var().unwrap_or(VariableId(SymbolId(0))).0)")

code = code.replace(
"""                let mut hir_init = HirExpr::Lit { value: Literal::Unit, ty: Type::Unit, span: let_stmt.span };""",
"""                let mut hir_init = HirExpr::Lit { value: Literal::Unit, ty: self.intern(ctx, Type::Unit), span: let_stmt.span };"""
)

code = code.replace(
"""            _ => (Type::Unit, HirStmt::Expr(HirExpr::Lit { value: Literal::Unit, ty: Type::Unit, span: Span::dummy() })),""",
"""            _ => (Type::Unit, HirStmt::Expr(HirExpr::Lit { value: Literal::Unit, ty: self.intern(ctx, Type::Unit), span: Span::dummy() })),"""
)

code = code.replace(
"""        let symbol_id = if let Some(id) = ctx.lookup(&id.name) {
            id.as_var().unwrap_or(VariableId(SymbolId(0)))
        }""",
"""        let symbol_id = if let Some(id) = ctx.lookup(&id.name) {
            id.as_var().unwrap_or(VariableId(SymbolId(0)))
        }"""
)

# Replace remaining `ty: Type::Unit`
code = code.replace("ty: Type::Unit", "ty: self.intern(ctx, Type::Unit)")

# Fix ScopedId inside VarRef
code = code.replace("id: symbol_id,", "id: symbol_id.as_var().unwrap_or(VariableId(SymbolId(0))),")
code = code.replace("id: method_id,", "id: VariableId(method_id.0),") # because method_id is ScopedId but VarRef expects VariableId

code = code.replace("ctx.symbol_table.define_anon(", "ctx.symbol_table.define_anon_var(")

# Fix get_var_mut -> we need to pass VariableId
code = code.replace("ctx.symbol_table.get_var_mut(id)", "ctx.symbol_table.get_var_mut(VariableId(id))")
code = code.replace("ctx.symbol_table.get_var_mut(id.as_var().unwrap())", "ctx.symbol_table.get_var_mut(id.as_var().unwrap())")


with open("crates/eng-types/src/type_pass.rs", "w") as f:
    f.write(code)
