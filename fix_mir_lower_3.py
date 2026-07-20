import re

with open('crates/vinglish-types/src/mir_lower.rs', 'r') as f:
    content = f.read()

# Change new_temp signature
content = content.replace('fn new_temp(&mut self, ty: TypeId) -> VariableId {', 'fn new_temp(&mut self, ty: TypeId, span: vinglish_lexer::Span) -> VariableId {')
content = content.replace('span: None, };\n        // Define it', 'span: Some(span), };\n        // Define it')

# Now find all calls to new_temp
# 1. Inside lower_expr: replace `self.new_temp(X)` with `self.new_temp(X, expr.span)`
def replace_in_lower_expr(match):
    text = match.group(0)
    text = re.sub(r'self\.new_temp\(([^,]+)\)', r'self.new_temp(\1, expr.span)', text)
    return text

content = re.sub(r'fn lower_expr.*?^    }', replace_in_lower_expr, content, flags=re.MULTILINE|re.DOTALL)

# 2. Inside lower_stmt: replace `self.new_temp(X)` with `self.new_temp(X, span)`
def replace_in_lower_stmt(match):
    text = match.group(0)
    text = re.sub(r'self\.new_temp\(([^,]+)\)', r'self.new_temp(\1, *span)', text)
    return text

content = re.sub(r'fn lower_stmt.*?^    }', replace_in_lower_stmt, content, flags=re.MULTILINE|re.DOTALL)

with open('crates/vinglish-types/src/mir_lower.rs', 'w') as f:
    f.write(content)
