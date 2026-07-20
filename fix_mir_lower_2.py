import re

with open('crates/vinglish-types/src/mir_lower.rs', 'r') as f:
    content = f.read()

content = content.replace('fn new_temp(&mut self, ty: TypeId) -> VariableId {', 'fn new_temp(&mut self, ty: TypeId, span: vinglish_lexer::Span) -> VariableId {')
content = content.replace('span: None, };\n        // Define it', 'span: Some(span), };\n        // Define it')

# Now for all occurrences of new_temp(
# In lower_expr, `expr` is in scope, so `expr.span` works.
# Let's just blindly replace `new_temp(.*)` with `new_temp(\1, expr.span)` 
# Wait, some are `new_temp(target.ty())`, let's do regex
content = re.sub(r'new_temp\(([^,]+)\)', r'new_temp(\1, expr.span)', content)

# In `lower_stmt`, `expr` might not be in scope, it's `stmt`. Wait! In `lower_stmt`:
# There are no `new_temp` calls in `lower_stmt` directly!
# Let's check `grep` output:
# Line 319-337: `let temp = self.new_temp(target.ty());` -> These are inside `HirStmt::Assign`, so `target` is a `HirExpr`. We can use `target.span` or `span` (since `HirStmt::Assign { target, value, span }`).
# Let's just use `span` for these lines!

with open('crates/vinglish-types/src/mir_lower.rs', 'w') as f:
    f.write(content)

