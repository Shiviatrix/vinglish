use crate::alias::AliasGraph;
use crate::escape::EscapeAnalysis;
use crate::lifetime::LifetimeGraph;
use vinglish_diagnostics::Diagnostic;
use vinglish_hir::symbol::SsaValueId;
use vinglish_lexer::Span;
use vinglish_mir::{Instruction, MirModule};

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
                                    Span::default(),
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
