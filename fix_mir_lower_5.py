import re

with open('crates/vinglish-types/src/mir_lower.rs', 'r') as f:
    content = f.read()

# Replace expr.span with expr.span()
content = content.replace('expr.span)', 'expr.span())')

# For let temp = self.new_temp(target.ty(), *span); we need to extract span
content = content.replace('HirStmt::Assign { target, op, value, .. } => {', 'HirStmt::Assign { target, op, value, span, .. } => {')
content = content.replace('let temp = self.new_temp(target.ty(), *span);', 'let temp = self.new_temp(target.ty(), *span);')

with open('crates/vinglish-types/src/mir_lower.rs', 'w') as f:
    f.write(content)
