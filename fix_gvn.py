import re

with open('crates/vinglish-opt/src/gvn.rs', 'r') as f:
    content = f.read()

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
    'let mut value_table: HashMap<ValueExpr, SsaValueId> = HashMap::new();',
    helper + '\n            let mut value_table: HashMap<ValueExpr, SsaValueId> = HashMap::new();'
)

# And replace `replacements.insert(d, existing_val);`
# But wait, in `gvn.rs`, `d` is the destination, `existing_val` is the old value.
# We map `d` -> `existing_val`.
# If `d` is a user variable and `existing_val` is a temp variable, we don't want to replace `d` with `existing_val`!
# Or we just skip the replacement.
old_replacement = """                        if let Some(&existing_val) = value_table.get(&e) {
                            replacements.insert(d, existing_val);
                            stats.gvn_eliminated += 1;
                            keep = false;
                        } else {"""
new_replacement = """                        if let Some(&existing_val) = value_table.get(&e) {
                            if !is_temp(d) && is_temp(existing_val) {
                                // DO NOT REPLACE user var with temp var.
                                // Instead, update the table so future expressions map to the user var!
                                value_table.insert(e, d);
                            } else {
                                replacements.insert(d, existing_val);
                                stats.gvn_eliminated += 1;
                                keep = false;
                            }
                        } else {"""

content = content.replace(old_replacement, new_replacement)

with open('crates/vinglish-opt/src/gvn.rs', 'w') as f:
    f.write(content)
