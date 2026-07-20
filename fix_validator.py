import re

with open('crates/vinglish-own/src/validator.rs', 'r') as f:
    content = f.read()

content = content.replace('use std::collections::HashSet;', 'use std::collections::HashMap;')
content = content.replace('let mut moved = HashSet::new();', 'let mut moved = HashMap::new();')

get_span_code = """
        let get_span = |id: SsaValueId| -> vinglish_lexer::Span {
            if let Some(vinglish_hir::symbol::SymbolKind::Variable(vs)) =
                symbol_table.get(vinglish_hir::symbol::SymbolId(id.0))
            {
                vs.span.unwrap_or_default()
            } else {
                vinglish_lexer::Span::default()
            }
        };
"""
content = content.replace('        let is_move = |var_id: SsaValueId| -> bool {', get_span_code + '\n        let is_move = |var_id: SsaValueId| -> bool {')

check_op_orig = """                    let mut check_op = |op: &Operand<SsaValueId>,
                                        is_val: bool,
                                        dest: SsaValueId| {
                        if let Operand::<SsaValueId>::Var(src) = op {
                            if moved.contains(src) {
                                errors.push(diagnostics::use_after_move(symbol_table, *src, dest));
                            } else if is_val && is_move(*src) {
                                moved.insert(*src);
                            }
                        }
                    };"""

check_op_new = """                    let mut check_op = |op: &Operand<SsaValueId>,
                                        is_val: bool,
                                        dest: SsaValueId| {
                        if let Operand::<SsaValueId>::Var(src) = op {
                            if let Some(move_span) = moved.get(src) {
                                let use_span = get_span(dest);
                                errors.push(diagnostics::use_after_move(symbol_table, *src, dest, use_span, *move_span));
                            } else if is_val && is_move(*src) {
                                moved.insert(*src, get_span(dest));
                            }
                        }
                    };"""

content = content.replace(check_op_orig, check_op_new)

# borrow_after_move
content = content.replace('if moved.contains(src) {', 'if let Some(move_span) = moved.get(src) {')
content = content.replace('errors.push(diagnostics::borrow_after_move(symbol_table, *src));', 'errors.push(diagnostics::borrow_after_move(symbol_table, *src, get_span(*_dest)));')

with open('crates/vinglish-own/src/validator.rs', 'w') as f:
    f.write(content)

