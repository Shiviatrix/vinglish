pub mod analysis;
pub mod diagnostics;
pub mod graph;
pub mod state;
pub mod validator;

pub use analysis::OwnershipAnalysisPass;
pub use graph::OwnershipGraph;
pub use state::OwnershipState;
pub use validator::OwnershipValidator;

pub fn analyze_ownership(
    mut module: vinglish_mir::MirModule<vinglish_hir::symbol::SsaValueId>,
    symbol_table: &vinglish_hir::symbol::SymbolTable,
) -> Result<vinglish_mir::MirModule<vinglish_hir::symbol::SsaValueId>, Vec<vinglish_diagnostics::Diagnostic>> {
    let pass = analysis::OwnershipAnalysisPass::new();
    let graph = pass.run(&mut module, symbol_table);

    let validator = validator::OwnershipValidator::new();
    validator.validate(symbol_table, &module, &graph)?;

    Ok(module)
}
