import re

with open('crates/vinglish-types/src/mir_lower.rs', 'r') as f:
    content = f.read()

# Change `fn new_temp(&mut self, ty: TypeId) -> VariableId {`
content = content.replace('fn new_temp(&mut self, ty: TypeId) -> VariableId {', 'fn new_temp(&mut self, ty: TypeId, span: vinglish_lexer::Span) -> VariableId {')

# Replace `span: None, };` with `span: Some(span), };` in new_temp
content = content.replace('span: None, };\n        // Define it', 'span: Some(span), };\n        // Define it')

# Now find all calls to `new_temp(ty)` and change to `new_temp(ty, expr.span)`
# Actually, wait. It's often called with `self.new_temp(ty)`. We need to pass the span of the expression being lowered.
# In `lower_expr`, we have `let span = expr.span;` at the top? No, `expr` doesn't have `span` maybe? 
# Wait, HirExpr has span! Let's see how new_temp is called.
