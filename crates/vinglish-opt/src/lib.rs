/// TODO: Describe implementation.
pub mod cfg_simplify;
/// TODO: Describe implementation.
pub mod constant_folding;
/// TODO: Describe implementation.
pub mod constant_prop;
/// TODO: Describe implementation.
pub mod copy_prop;
/// TODO: Describe implementation.
pub mod dce;
/// TODO: Describe implementation.
pub mod gvn;

use std::fmt::Display;
use std::hash::Hash;
use vinglish_mir::MirModule;

/// TODO: Describe implementation.
#[derive(Default, Debug, Clone)]
pub struct PassStats {
    pub removed_instructions: usize,
    pub merged_blocks: usize,
    pub folded_constants: usize,
    pub gvn_eliminated: usize,
}

impl PassStats {
    /// TODO: Describe implementation.
    pub fn add(&mut self, other: &PassStats) {
        self.removed_instructions += other.removed_instructions;
        self.merged_blocks += other.merged_blocks;
        self.folded_constants += other.folded_constants;
        self.gvn_eliminated += other.gvn_eliminated;
    }
}

/// TODO: Describe implementation.
pub trait OptimizationPass<
    V: Clone + Copy + Display + Eq + Hash + vinglish_hir::symbol::HasSymbolId,
>
{
    fn name(&self) -> &'static str;
    fn run(
        &mut self,
        module: &mut MirModule<V>,
        symbol_table: &vinglish_hir::symbol::SymbolTable,
    ) -> PassStats;
}

/// TODO: Describe implementation.
pub struct PassManager<V: Clone + Copy + Display + Eq + Hash + vinglish_hir::symbol::HasSymbolId> {
    passes: Vec<Box<dyn OptimizationPass<V>>>,
}

impl<V: Clone + Copy + Display + Eq + Hash + vinglish_hir::symbol::HasSymbolId> Default
    for PassManager<V>
{
    fn default() -> Self {
        Self::new()
    }
}

impl<V: Clone + Copy + Display + Eq + Hash + vinglish_hir::symbol::HasSymbolId> PassManager<V> {
    /// TODO: Describe implementation.
    pub fn new() -> Self {
        Self { passes: Vec::new() }
    }

    /// TODO: Describe implementation.
    pub fn add_pass(&mut self, pass: Box<dyn OptimizationPass<V>>) {
        self.passes.push(pass);
    }

    /// TODO: Describe implementation.
    pub fn run_all(
        &mut self,
        module: &mut MirModule<V>,
        symbol_table: &vinglish_hir::symbol::SymbolTable,
    ) -> Result<PassStats, Vec<vinglish_mir::validator::MirValidationError>> {
        let mut total_stats = PassStats::default();
        let validator = vinglish_mir::validator::MirValidatorPass::new();

        for pass in &mut self.passes {
            let stats = pass.run(module, symbol_table);
            total_stats.add(&stats);

            validator.validate(symbol_table, module)?;
        }
        Ok(total_stats)
    }
}

/// TODO: Describe implementation.
pub fn pre_ssa_pipeline() -> PassManager<vinglish_hir::symbol::VariableId> {
    let mut pm = PassManager::new();
    pm.add_pass(Box::new(dce::DeadCodeEliminationPass));
    pm.add_pass(Box::new(cfg_simplify::CfgSimplifyPass));
    pm
}

/// TODO: Describe implementation.
pub fn post_ssa_pipeline() -> PassManager<vinglish_hir::symbol::SsaValueId> {
    let mut pm = PassManager::new();
    pm.add_pass(Box::new(constant_folding::ConstantFoldingPass));
    pm.add_pass(Box::new(constant_prop::ConstantPropagationPass));
    pm.add_pass(Box::new(copy_prop::CopyPropagationPass));
    pm.add_pass(Box::new(gvn::GlobalValueNumberingPass));
    pm.add_pass(Box::new(dce::DeadCodeEliminationPass));
    pm
}
