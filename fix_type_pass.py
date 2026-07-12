import re

with open("crates/eng-types/src/type_pass.rs", "r") as f:
    code = f.read()

# Fix record
code = code.replace(
"""    fn record(&mut self, ctx: &mut CompilerContext, span: Span, ty: Type) {
        let resolved = self.uf.apply(&ty);
        ctx.types.insert(span.start, resolved);
    }""",
"""    fn record(&mut self, ctx: &mut CompilerContext, span: Span, ty: Type) {
        let resolved = self.uf.apply(&ty);
        let id = ctx.symbol_table.intern_type(resolved);
        ctx.types.insert(span.start, id);
    }""")

# Add helper intern method
code = code.replace(
"""    fn resolve(&self, ty: Type) -> Type {
        self.uf.apply(&ty)
    }""",
"""    fn resolve(&self, ty: Type) -> Type {
        self.uf.apply(&ty)
    }

    fn intern(&self, ctx: &mut CompilerContext, ty: Type) -> eng_hir::symbol::TypeId {
        let resolved = self.resolve(ty);
        ctx.symbol_table.intern_type(resolved)
    }""")

# Fix symbol imports
code = code.replace(
"""use eng_hir::symbol::{TypeSymbol, SymbolId, SymbolTable, SymbolKind, VariableSymbol, FunctionSymbol};""",
"""use eng_hir::symbol::{TypeSymbol, SymbolId, SymbolTable, SymbolKind, VariableSymbol, FunctionSymbol, TypeId, VariableId, FunctionId, FieldId};"""
)

# Fix SymbolId(0) to VariableId/TypeId
code = code.replace("id: SymbolId(0)", "id: VariableId(SymbolId(0))")
code = code.replace("id: type_id_opt.unwrap_or(SymbolId(0))", "id: TypeId(type_id_opt.unwrap_or(SymbolId(0)))")

# Fix method lookups
code = code.replace(
"""                    if let Some(SymbolKind::Function(fs)) = ctx.symbol_table.get(method_id) {""",
"""                    if let Some(fs) = ctx.symbol_table.get_func(method_id) {""")

code = code.replace(
"""                    let method_id = ts.methods.get(&method_name).unwrap().clone();""",
"""                    let method_id = *ts.methods.get(&method_name).unwrap();"""
)

code = code.replace(
"""                    let m_ty = if let Some(SymbolKind::Function(fs)) = ctx.symbol_table.get(method_id) {
                        fs.ty.clone()
                    }""",
"""                    let m_ty = if let Some(fs) = ctx.symbol_table.get_func(method_id) {
                        fs.ty.clone()
                    }"""
)

code = re.sub(r"HirExpr::\w+\s*\{[^\}]*ty:\s*([a-zA-Z0-9_\.\(\)\*]+),\s*span:", lambda m: m.group(0).replace(f"ty: {m.group(1)}", f"ty: self.intern(ctx, {m.group(1)}.clone())"), code)

code = code.replace("index,", "field_id: FieldId(index),")
code = code.replace("index: f_sym.index", "field_id: f_sym.id")
code = code.replace("f_sym.index", "f_sym.id.0")

code = code.replace(
"""        let symbol_id = if let Some(id) = ctx.lookup(&id.name) {
            id
        } else {
            ctx.type_errors.push(TypeError::new(
                format!("undefined variable: {}", id.name),
                id.span,
            ));
            SymbolId(0)
        };""",
"""        let symbol_id = if let Some(id) = ctx.lookup(&id.name) {
            id.as_var().unwrap_or(VariableId(SymbolId(0)))
        } else {
            ctx.type_errors.push(TypeError::new(
                format!("undefined variable: {}", id.name),
                id.span,
            ));
            VariableId(SymbolId(0))
        };"""
)

code = code.replace(
"""        let symbol_id = if let Some(id) = ctx.lookup(&callee_id.name) {
            id
        }""",
"""        let symbol_id = if let Some(id) = ctx.lookup(&callee_id.name) {
            id.as_func().unwrap_or(FunctionId(SymbolId(0)))
        }"""
)

with open("crates/eng-types/src/type_pass.rs", "w") as f:
    f.write(code)
