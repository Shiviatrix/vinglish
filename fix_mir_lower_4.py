import re

with open('crates/vinglish-types/src/mir_lower.rs', 'r') as f:
    content = f.read()

# Change new_temp signature
content = content.replace('fn new_temp(&mut self, ty: TypeId) -> VariableId {', 'fn new_temp(&mut self, ty: TypeId, span: vinglish_lexer::Span) -> VariableId {')
content = content.replace('span: None, };\n        // Define it', 'span: Some(span), };\n        // Define it')

# For all occurrences of `let temp = self.new_temp(X);` in lower_expr, we replace with `expr.span`
content = content.replace('let temp = self.new_temp(*ty);', 'let temp = self.new_temp(*ty, expr.span);')
content = content.replace('let tag_temp = self.new_temp(int_ty_id);', 'let tag_temp = self.new_temp(int_ty_id, expr.span);')
content = content.replace('let is_ok = self.new_temp(bool_ty_id);', 'let is_ok = self.new_temp(bool_ty_id, expr.span);')
content = content.replace('let ok_val = self.new_temp(*ty);', 'let ok_val = self.new_temp(*ty, expr.span);')

# In lower_stmt, there are:
# let temp = self.new_temp(target.ty());
# these are inside a match arm where `span` is a variable (`HirStmt::Assign { ..., span }`).
content = content.replace('let temp = self.new_temp(target.ty());', 'let temp = self.new_temp(target.ty(), *span);')

with open('crates/vinglish-types/src/mir_lower.rs', 'w') as f:
    f.write(content)
