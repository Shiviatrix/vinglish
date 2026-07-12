import re

with open("crates/eng-types/src/type_pass.rs", "r") as f:
    code = f.read()

code = code.replace("ctx.define(\"self\".to_string(), self_id);", "ctx.define(\"self\".to_string(), passes::ScopedId::Var(self_id));")
code = code.replace("ty: self_ty.clone(),", "ty: self.intern(ctx, self_ty.clone()),")

code = code.replace("if let Some(SymbolKind::Variable(vs)) = ctx.symbol_table.get_mut(param_id)", "if let Some(vs) = ctx.symbol_table.get_var_mut(param_id)")
code = code.replace("ctx.define(param.name.name.clone(), param_id);", "ctx.define(param.name.name.clone(), passes::ScopedId::Var(param_id));")

code = code.replace("if let Some(SymbolKind::Function(fs)) = ctx.symbol_table.get_mut(method_id)", "if let Some(fs) = ctx.symbol_table.get_func_mut(method_id)")
code = code.replace("if let Some(SymbolKind::Type(ts)) = ctx.symbol_table.get_mut(type_id)", "if let Some(ts) = ctx.symbol_table.get_type_mut(type_id.as_type().unwrap())")
code = code.replace("ts.add_method(f.name.name.clone(), fn_ty.clone(), f.visibility);", "ts.add_method(f.name.name.clone(), method_id);")

code = code.replace("if let Some(SymbolKind::Function(fs)) = ctx.symbol_table.get_mut(func_id)", "if let Some(fs) = ctx.symbol_table.get_func_mut(func_id)")
code = code.replace("if let Some(SymbolKind::Variable(vs)) = ctx.symbol_table.get_mut(id)", "if let Some(vs) = ctx.symbol_table.get_var_mut(id)")

code = code.replace("HirExpr::VarRef { id: symbol_id.as_var().unwrap_or(VariableId(SymbolId(0))), ty, span: id.span }", "HirExpr::VarRef { id: symbol_id.as_var().unwrap_or(VariableId(SymbolId(0))), ty: self.intern(ctx, ty.clone()), span: id.span }")

code = code.replace("ctx.symbol_table.get_func(method_id)", "ctx.symbol_table.get_func(method_id.as_func().unwrap_or(FunctionId(SymbolId(0))))")
code = code.replace("VariableId(method_id.0.0)", "VariableId(method_id.as_func().unwrap_or(FunctionId(SymbolId(0))).0)")

code = code.replace("if let Some(SymbolKind::Variable(vs)) = ctx.symbol_table.get(symbol_id.as_var().unwrap_or(VariableId(SymbolId(0))).0) {", "if let Some(vs) = ctx.symbol_table.get_var(symbol_id.as_var().unwrap_or(VariableId(SymbolId(0)))) {")
code = code.replace("} else if let Some(SymbolKind::Function(fs)) = ctx.symbol_table.get(symbol_id.as_var().unwrap_or(VariableId(SymbolId(0))).0) {", "} else if let Some(fs) = ctx.symbol_table.get_func(symbol_id.as_func().unwrap_or(FunctionId(SymbolId(0)))) {")

with open("crates/eng-types/src/type_pass.rs", "w") as f:
    f.write(code)
