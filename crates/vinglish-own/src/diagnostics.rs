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
) -> Diagnostic {
    let name = get_var_name(symbol_table, var);
    let to_name = get_var_name(symbol_table, moved_to);

    Diagnostic::error(
        "E001",
        format!("Use of moved value `{}`", name),
        Span::default(),
    )
    .with_note(format!("`{}` was moved here.", name))
    .with_note(format!("Ownership transferred to `{}`.", to_name))
    .with_note("This value can no longer be used.")
    // Instead of help, we can use a Suggestion if there's span, or just note
    .with_note(format!("help: borrow `{}`", name))
    .with_note(format!("help: clone `{}`", name))
}

pub fn double_mutable_borrow(symbol_table: &SymbolTable, var: SsaValueId) -> Diagnostic {
    let name = get_var_name(symbol_table, var);
    Diagnostic::error(
        "E002",
        format!("Cannot borrow `{}` as mutable more than once", name),
        Span::default(),
    )
    .with_note(format!("`{}` was already borrowed mutably.", name))
}

pub fn borrow_after_move(symbol_table: &SymbolTable, var: SsaValueId) -> Diagnostic {
    let name = get_var_name(symbol_table, var);
    Diagnostic::error(
        "E003",
        format!("Cannot borrow `{}` because it was moved", name),
        Span::default(),
    )
    .with_note(format!("`{}` was moved and is no longer valid.", name))
}
