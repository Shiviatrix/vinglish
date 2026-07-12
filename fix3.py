import re

with open("crates/eng-types/src/type_pass.rs", "r") as f:
    code = f.read()

# Fix define/get function methods
code = code.replace(
"""            let method_id = ctx.symbol_table.define(hir_name.clone(), SymbolKind::Function(FunctionSymbol {""",
"""            let method_id = ctx.symbol_table.define_func(hir_name.clone(), FunctionSymbol {""")

code = code.replace(
"""                id: VariableId(SymbolId(0)),
                name: hir_name.clone(),
                visibility: f.visibility,
                ty: fn_ty.clone(),
            }));""",
"""                id: FunctionId(SymbolId(0)),
                name: hir_name.clone(),
                visibility: f.visibility,
                ty: fn_ty.clone(),
            });"""
)

code = code.replace(
"""            if let Some(SymbolKind::Function(fs)) = ctx.symbol_table.get_var_mut(method_id) { fs.id = method_id; }""",
"""            if let Some(fs) = ctx.symbol_table.get_func_mut(method_id) { fs.id = method_id; }"""
)

code = code.replace(
"""                if let Some(SymbolKind::Type(ts)) = ctx.symbol_table.get_var_mut(type_id) {
                    ts.add_method(f.name.name.clone(), fn_ty.clone(), f.visibility);
                }""",
"""                if let Some(ts) = ctx.symbol_table.get_type_mut(type_id.as_type().unwrap()) {
                    ts.add_method(f.name.name.clone(), method_id);
                }"""
)

code = code.replace(
"""            ctx.lookup(&f.name.name).unwrap_or(SymbolId(0))""",
"""            ctx.lookup(&f.name.name).unwrap().as_func().unwrap()"""
)

code = code.replace(
"""        if let Some(SymbolKind::Function(fs)) = ctx.symbol_table.get_var_mut(func_id) {""",
"""        if let Some(fs) = ctx.symbol_table.get_func_mut(func_id) {"""
)

code = code.replace(
"""            ret_ty: expected_ret,""",
"""            ret_ty: self.intern(ctx, expected_ret),"""
)

code = code.replace(
"""            ty: last.clone(),""",
"""            ty: self.intern(ctx, last.clone()),"""
)

# Fix define_anon_var
code = code.replace(
"""                let id = ctx.symbol_table.define_anon_var(SymbolKind::Variable(VariableSymbol {
                    id: VariableId(SymbolId(0)),
                    name: let_stmt.name.name.clone(),
                    is_mut: let_stmt.mutable,
                    ty: resolved.clone(),
                }));""",
"""                let id = ctx.symbol_table.define_anon_var(VariableSymbol {
                    id: VariableId(SymbolId(0)),
                    name: let_stmt.name.name.clone(),
                    is_mut: let_stmt.mutable,
                    ty: resolved.clone(),
                });"""
)

code = code.replace(
"""                if let Some(SymbolKind::Variable(vs)) = ctx.symbol_table.get_var_mut(id) { vs.id = id; }""",
"""                if let Some(vs) = ctx.symbol_table.get_var_mut(id) { vs.id = id; }"""
)

code = code.replace(
"""                ctx.define(let_stmt.name.name.clone(), id);""",
"""                ctx.define(let_stmt.name.name.clone(), passes::ScopedId::Var(id));"""
)

code = code.replace(
"""                    ty: resolved,""",
"""                    ty: self.intern(ctx, resolved),"""
)

code = code.replace(
"""                (ty.clone(), HirExpr::Lit { value: value.clone(), ty, span: *span })""",
"""                (ty.clone(), HirExpr::Lit { value: value.clone(), ty: self.intern(ctx, ty), span: *span })"""
)

code = code.replace(
"""                    if let Some(fs) = ctx.symbol_table.get(method_id.0) {""",
"""                    if let Some(fs) = ctx.symbol_table.get_func(method_id) {"""
)

code = code.replace(
"""                        m_ty = fs.ty.clone(); // Instantiation needed in true HM""",
"""                        m_ty = fs.ty.clone(); // Instantiation needed in true HM"""
)

code = code.replace(
"""                    hir_callee = HirExpr::VarRef { id: VariableId(method_id.0), ty: self.intern(ctx, m_ty.clone().clone()), span: field.span };""",
"""                    hir_callee = HirExpr::VarRef { id: VariableId(method_id.0.0), ty: self.intern(ctx, m_ty.clone().clone()), span: field.span };"""
)

with open("crates/eng-types/src/type_pass.rs", "w") as f:
    f.write(code)
