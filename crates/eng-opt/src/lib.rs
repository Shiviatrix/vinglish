pub mod cfg_simplify;
pub mod constant_folding;
pub mod constant_prop;
pub mod copy_prop;
pub mod dce;
pub mod gvn;

use eng_mir::MirModule;
use std::fmt::Display;
use std::hash::Hash;

#[derive(Default, Debug, Clone)]
pub struct PassStats {
    pub removed_instructions: usize,
    pub merged_blocks: usize,
    pub folded_constants: usize,
    pub gvn_eliminated: usize,
}

impl PassStats {
    pub fn add(&mut self, other: &PassStats) {
        self.removed_instructions += other.removed_instructions;
        self.merged_blocks += other.merged_blocks;
        self.folded_constants += other.folded_constants;
        self.gvn_eliminated += other.gvn_eliminated;
    }
}

pub trait OptimizationPass<V: Clone + Copy + Display + Eq + Hash> {
    fn name(&self) -> &'static str;
    fn run(&mut self, module: &mut MirModule<V>) -> PassStats;
}

pub struct PassManager<V: Clone + Copy + Display + Eq + Hash> {
    passes: Vec<Box<dyn OptimizationPass<V>>>,
}

impl<V: Clone + Copy + Display + Eq + Hash> Default for PassManager<V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<V: Clone + Copy + Display + Eq + Hash> PassManager<V> {
    pub fn new() -> Self {
        Self { passes: Vec::new() }
    }

    pub fn add_pass(&mut self, pass: Box<dyn OptimizationPass<V>>) {
        self.passes.push(pass);
    }

    pub fn run_all(
        &mut self,
        module: &mut MirModule<V>,
        symbol_table: &eng_hir::symbol::SymbolTable,
    ) -> Result<PassStats, Vec<eng_mir::validator::MirValidationError>> {
        let mut total_stats = PassStats::default();
        let validator = eng_mir::validator::MirValidatorPass::new();

        for pass in &mut self.passes {
            let stats = pass.run(module);
            total_stats.add(&stats);

            validator.validate(symbol_table, module)?;
        }
        Ok(total_stats)
    }
}

pub fn pre_ssa_pipeline() -> PassManager<eng_hir::symbol::VariableId> {
    let mut pm = PassManager::new();
    pm.add_pass(Box::new(dce::DeadCodeEliminationPass));
    pm.add_pass(Box::new(cfg_simplify::CfgSimplifyPass));
    pm
}

pub fn post_ssa_pipeline() -> PassManager<eng_hir::symbol::SsaValueId> {
    let mut pm = PassManager::new();
    pm.add_pass(Box::new(constant_folding::ConstantFoldingPass));
    pm.add_pass(Box::new(constant_prop::ConstantPropagationPass));
    pm.add_pass(Box::new(copy_prop::CopyPropagationPass));
    pm.add_pass(Box::new(gvn::GlobalValueNumberingPass));
    pm.add_pass(Box::new(dce::DeadCodeEliminationPass));
    pm.add_pass(Box::new(cfg_simplify::CfgSimplifyPass));
    pm
}
