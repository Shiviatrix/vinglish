use vinglish_diagnostics::Diagnostic;
use vinglish_hir::symbol::{SsaValueId, SymbolKind, SymbolTable};
use vinglish_lexer::Span;

fn get_var_name(symbol_table: &SymbolTable, var: SsaValueId) -> String {
    if let Some(SymbolKind::Variable(v)) = symbol_table.get(vinglish_hir::symbol::SymbolId(var.0)) {
        v.name.clone()
    } else {
        format!("var_{}", var.0)
    }
}

pub fn use_after_move(
    symbol_table: &SymbolTable,
    var: SsaValueId,
    moved_to: SsaValueId,
    use_span: Span,
    move_span: Span,
) -> Diagnostic {
    let name = get_var_name(symbol_table, var);
    let to_name = get_var_name(symbol_table, moved_to);
    
    // Attempt to strip SSA suffixes for better user messages
    // Remove trailing _NUMBER
    let mut clean_name = name.clone();
    if let Some(pos) = name.rfind('_') {
        if name[pos + 1..].chars().all(|c| c.is_ascii_digit()) {
            clean_name = name[..pos].to_string();
        }
    }
    let display_name = if clean_name.starts_with("_tmp") || clean_name.starts_with("tmp") {
        "temporary value".to_string()
    } else {
        format!("`{}`", clean_name)
    };

    let mut diag = Diagnostic::error(
        "E001",
        format!("Use of moved value {}", display_name),
        use_span,
    )
    .with_note(format!("Value was moved in a previous statement."));

    if to_name.starts_with("_tmp") {
        diag = diag.with_note("Ownership was transferred to another value or function call.");
    } else {
        diag = diag.with_note(format!("Ownership transferred to `{}`.", to_name));
    }

    diag.with_note("This value can no longer be used.")
    .with_note(format!("help: consider borrowing `&{}` instead of moving it", clean_name))
}

pub fn double_mutable_borrow(symbol_table: &SymbolTable, var: SsaValueId, span: Span) -> Diagnostic {
    let name = get_var_name(symbol_table, var);
    Diagnostic::error(
        "E002",
        format!("Cannot borrow `{}` as mutable more than once", name),
        span,
    )
    .with_note(format!("`{}` was already borrowed mutably.", name))
}

pub fn borrow_after_move(symbol_table: &SymbolTable, var: SsaValueId, span: Span) -> Diagnostic {
    let name = get_var_name(symbol_table, var);
    Diagnostic::error(
        "E003",
        format!("Cannot borrow `{}` because it was moved", name),
        span,
    )
    .with_note(format!("`{}` was moved and is no longer valid.", name))
}
