pub mod state;
pub mod graph;
pub mod analysis;
pub mod validator;
pub mod diagnostics;

pub use state::OwnershipState;
pub use graph::OwnershipGraph;
pub use analysis::OwnershipAnalysisPass;
pub use validator::OwnershipValidator;

pub fn analyze_ownership(mut module: eng_mir::MirModule<eng_hir::symbol::SsaValueId>, symbol_table: &eng_hir::symbol::SymbolTable) -> Result<eng_mir::MirModule<eng_hir::symbol::SsaValueId>, Vec<eng_diagnostics::Diagnostic>> {
    let pass = analysis::OwnershipAnalysisPass::new();
    let graph = pass.run(&mut module, symbol_table);

    let validator = validator::OwnershipValidator::new();
    validator.validate(symbol_table, &module, &graph)?;

    Ok(module)
}
