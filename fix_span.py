import re

files_to_fix = [
    'crates/vinglish-types/src/type_pass.rs',
    'crates/vinglish-types/src/mir_lower.rs',
    'crates/vinglish-ssa/src/rename.rs'
]

for file in files_to_fix:
    with open(file, 'r') as f:
        content = f.read()
    
    # We want to add `span: None,` to `VariableSymbol { ... }` where it's missing.
    # The safest way is to replace `ty: vs.ty.clone(),\n                }` with `ty: vs.ty.clone(),\n                    span: vs.span,\n                }` in rename.rs
    
    # Let's just do simple replacements.
    if 'rename.rs' in file:
        content = content.replace('ty: vs.ty.clone(),\n                }', 'ty: vs.ty.clone(),\n                    span: vs.span,\n                }')
        # But maybe `ty: ty,` or similar? Let's check `rename.rs` exactly.
        
    content = re.sub(r'(VariableSymbol\s*\{[^}]+ty:[^},]+,)\s*\}', r'\1 span: None, }', content)
    
    with open(file, 'w') as f:
        f.write(content)
