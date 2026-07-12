import re

with open("crates/eng-types/src/type_pass.rs", "r") as f:
    code = f.read()

# Fix definition of types
code = code.replace(
"""                let id = ctx.lookup(&t.name.name).unwrap_or(SymbolId(0));""",
"""                let id = ctx.lookup(&t.name.name).unwrap_or(passes::ScopedId::Type(TypeId(SymbolId(0)))).as_type().unwrap();"""
)
code = code.replace(
"""                if let Some(SymbolKind::Type(ts)) = ctx.symbol_table.get_var_mut(id) {""",
"""                if let Some(ts) = ctx.symbol_table.get_type_mut(id) {"""
)
code = code.replace(
"""                        ts.add_field(f.name.clone(), type_expr_to_type(&f.ty), Visibility::Public);""",
"""                        ts.add_field(f.name.clone(), type_expr_to_type(&f.ty), Visibility::Public);"""
)

# Function definitions
code = code.replace(
"""              let self_id = ctx.symbol_table.define_anon_var(SymbolKind::Variable(VariableSymbol {""",
"""              let self_id = ctx.symbol_table.define_anon_var(VariableSymbol {"""
)
code = code.replace(
"""                ty: self.intern(ctx, self_ty.clone()),""",
"""                ty: self_ty.clone(),"""
)
code = code.replace(
"""            if let Some(SymbolKind::Variable(vs)) = ctx.symbol_table.get_var_mut(self_id) { vs.id = self_id; }""",
"""            if let Some(vs) = ctx.symbol_table.get_var_mut(self_id) { vs.id = self_id; }"""
)

code = code.replace(
"""              let param_id = ctx.symbol_table.define_anon_var(SymbolKind::Variable(VariableSymbol {""",
"""              let param_id = ctx.symbol_table.define_anon_var(VariableSymbol {"""
)
code = code.replace(
"""            if let Some(SymbolKind::Variable(vs)) = ctx.symbol_table.get_var_mut(param_id) { vs.id = param_id; }""",
"""            if let Some(vs) = ctx.symbol_table.get_var_mut(param_id) { vs.id = param_id; }"""
)

# And inside HirParam it needs ty: TypeId
code = code.replace(
"""                ty: ty.clone(),
                span: param.span,
            });""",
"""                ty: self.intern(ctx, ty.clone()),
                span: param.span,
            });"""
)


# In infer_item (Type):
code = code.replace(
"""                    let id = ctx.symbol_table.define(
                        t.name.name.clone(),
                        SymbolKind::Type(TypeSymbol::new(SymbolId(0), t.name.name.clone(), t.visibility))
                    );""",
"""                    let id = ctx.symbol_table.define_type(
                        t.name.name.clone(),
                        TypeSymbol::new(TypeId(SymbolId(0)), t.name.name.clone(), t.visibility)
                    );"""
)
code = code.replace(
"""                    ctx.define(t.name.name.clone(), id);""",
"""                    ctx.define(t.name.name.clone(), passes::ScopedId::Type(id));"""
)

# Replace remaining `ty: type_expr_to_type(&f.ty)` inside HirParam loop
code = code.replace(
"""                        ty: type_expr_to_type(&f.ty),
                        span: f.span,
                    });""",
"""                        ty: self.intern(ctx, type_expr_to_type(&f.ty)),
                        span: f.span,
                    });"""
)

with open("crates/eng-types/src/type_pass.rs", "w") as f:
    f.write(code)
