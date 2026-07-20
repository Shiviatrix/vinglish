import re

with open('crates/vinglish-opt/src/copy_prop.rs', 'r') as f:
    content = f.read()

# We need to change how `copy_vars.insert(*dest, *src)` works.
# If dest is user variable and src is temp variable, we don't insert.
# But wait, how do we know if it's a temporary?
# We have `symbol_table: &SymbolTable` as a parameter now.
# Let's add `is_temp(v)` helper.
helper = """
            let is_temp = |v: V| -> bool {
                if let Some(vinglish_hir::symbol::SymbolKind::Variable(vs)) =
                    symbol_table.get(v.symbol_id())
                {
                    vs.name.starts_with("_tmp")
                } else {
                    false
                }
            };
"""

content = content.replace(
    'let mut copy_vars = HashMap::new();',
    helper + '\n            let mut copy_vars = HashMap::new();'
)

# Replace the condition `if assign_counts.get(dest) == Some(&1) && assign_counts.get(src).copied().unwrap_or(0) <= 1`
condition = """if assign_counts.get(dest) == Some(&1)
                            && assign_counts.get(src).copied().unwrap_or(0) <= 1
                        {
                            // Avoid replacing a user variable with a temporary
                            if !is_temp(*dest) && is_temp(*src) {
                                // DO NOT REPLACE
                            } else {
                                copy_vars.insert(*dest, *src);
                            }
                        }"""
old_condition = """if assign_counts.get(dest) == Some(&1)
                            && assign_counts.get(src).copied().unwrap_or(0) <= 1
                        {
                            copy_vars.insert(*dest, *src);
                        }"""
content = content.replace(old_condition, condition)

with open('crates/vinglish-opt/src/copy_prop.rs', 'w') as f:
    f.write(content)
