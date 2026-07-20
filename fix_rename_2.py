import re

with open('crates/vinglish-ssa/src/rename.rs', 'r') as f:
    content = f.read()

# Add span to the first one (line 130) if missing
content = content.replace('ty: vs.ty.clone(),\n                }', 'ty: vs.ty.clone(),\n                    span: vs.span,\n                }')
content = content.replace('ty,\n                }', 'ty,\n                    span: None,\n                }')

# Replace new_id.0 .0 with vs_name
block_orig = """            let mut ty = vinglish_hir::types::Type::Unit;
            if let Some(vinglish_hir::symbol::SymbolKind::Variable(vs)) =
                symbol_table.get(vinglish_hir::symbol::SymbolId(orig.0 .0))
            {
                ty = vs.ty.clone();
            }
            symbol_table.define_var_with_id(
                new_id.0,
                vinglish_hir::symbol::VariableSymbol {
                    id: new_id,
                    name: format!("{}_{}", new_id.0 .0, orig.0 .0), // give it some name
                    is_mut: false,
                    ty,
                    span: None,
                },
            );"""

block_new = """            let mut ty = vinglish_hir::types::Type::Unit;
            let mut vs_name = format!("var{}", orig.0 .0);
            let mut span = None;
            if let Some(vinglish_hir::symbol::SymbolKind::Variable(vs)) =
                symbol_table.get(vinglish_hir::symbol::SymbolId(orig.0 .0))
            {
                ty = vs.ty.clone();
                vs_name = vs.name.clone();
                span = vs.span;
            }
            symbol_table.define_var_with_id(
                new_id.0,
                vinglish_hir::symbol::VariableSymbol {
                    id: new_id,
                    name: format!("{}_{}", vs_name, new_id.0 .0),
                    is_mut: false,
                    ty,
                    span,
                },
            );"""

content = content.replace(block_orig, block_new)

with open('crates/vinglish-ssa/src/rename.rs', 'w') as f:
    f.write(content)
