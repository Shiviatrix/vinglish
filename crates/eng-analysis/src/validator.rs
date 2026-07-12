use eng_diagnostics::Diagnostic;
use eng_lexer::Span;
use eng_hir::symbol::SsaValueId;
use eng_mir::{Instruction, MirModule};
use crate::alias::AliasGraph;
use crate::escape::EscapeAnalysis;
use crate::lifetime::LifetimeGraph;

pub struct AnalysisValidator;

impl Default for AnalysisValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl AnalysisValidator {
    pub fn new() -> Self {
        Self
    }

    pub fn validate(
        &self,
        module: &MirModule<SsaValueId>,
        alias_graph: &AliasGraph,
        escape_analysis: &EscapeAnalysis,
        _lifetime_graph: &LifetimeGraph,
    ) -> Result<(), Vec<Diagnostic>> {
        let mut errors = Vec::new();

        for func in &module.functions {
            for block in &func.blocks {
                for instr in &block.instrs {
                    if let Instruction::StackAllocate(dest, _) = instr {
                        if let Some(alias) = alias_graph.get_alias(*dest) {
                            if escape_analysis.is_escaped(alias) {
                                let diag = Diagnostic::error(
                                    "E_ANALYSIS",
                                    format!("Stack allocation {} escapes the function", dest),
                                    Span::default()
                                );
                                errors.push(diag);
                            }
                        }
                    }
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}
