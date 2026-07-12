with open("crates/eng-types/src/passes.rs", "r") as f:
    code = f.read()

code = code.replace("ctx.symbol_table.define(", "ctx.symbol_table.define_type(")
code = code.replace("id: SymbolId(0),", "id: eng_hir::symbol::FunctionId(SymbolId(0)),")
code = code.replace("if let Some(SymbolKind::Type(ts)) = ctx.symbol_table.get_mut(id)", "if let Some(ts) = ctx.symbol_table.get_type_mut(id.as_type().unwrap())")
code = code.replace("if let Some(SymbolKind::Function(fs)) = ctx.symbol_table.get_mut(id)", "if let Some(fs) = ctx.symbol_table.get_func_mut(id.as_func().unwrap())")
code = code.replace("ctx.symbol_table.define_type(", "ctx.symbol_table.define_type(") # Noop
code = code.replace("ctx.symbol_table.define_var(", "ctx.symbol_table.define_var(") # Noop

# Specific replace for define
code = code.replace(
"""                    let id = ctx.symbol_table.define_type(
                        t.name.name.clone(),
                        SymbolKind::Type(TypeSymbol::new(SymbolId(0), t.name.name.clone(), t.visibility))
                    );""",
"""                    let id = ctx.symbol_table.define_type(
                        t.name.name.clone(),
                        TypeSymbol::new(eng_hir::symbol::TypeId(SymbolId(0)), t.name.name.clone(), t.visibility)
                    );"""
)

with open("crates/eng-types/src/passes.rs", "w") as f:
    f.write(code)
