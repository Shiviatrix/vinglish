import re

with open('crates/vinglish-ssa/src/rename.rs', 'r') as f:
    content = f.read()

content = content.replace('ty: vs.ty.clone(),', 'ty: vs.ty.clone(),\n                    span: vs.span,')
content = content.replace('ty,\n                }', 'ty,\n                    span: None,\n                }')

with open('crates/vinglish-ssa/src/rename.rs', 'w') as f:
    f.write(content)
