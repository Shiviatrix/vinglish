use crate::alias::{AliasGraph, AliasId};
use vinglish_hir::symbol::SsaValueId;
use vinglish_mir::{Instruction, MirFunction, MirModule, Operand, Terminator};
use std::collections::HashSet;
use std::fmt;

pub struct EscapeAnalysis {
    pub escaped_aliases: HashSet<AliasId>,
}

impl Default for EscapeAnalysis {
    fn default() -> Self {
        Self::new()
    }
}

impl EscapeAnalysis {
    pub fn new() -> Self {
        Self {
            escaped_aliases: HashSet::new(),
        }
    }

    pub fn is_escaped(&self, alias: AliasId) -> bool {
        self.escaped_aliases.contains(&alias)
    }
}

impl fmt::Display for EscapeAnalysis {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "--- ESCAPE ANALYSIS ---")?;
        let mut sorted_escapes: Vec<_> = self.escaped_aliases.iter().copied().collect();
        sorted_escapes.sort_by_key(|a| a.0);
        for alias in sorted_escapes {
            writeln!(f, "{} escapes", alias)?;
        }
        Ok(())
    }
}

pub struct EscapeAnalysisPass;

impl Default for EscapeAnalysisPass {
    fn default() -> Self {
        Self::new()
    }
}

impl EscapeAnalysisPass {
    pub fn new() -> Self {
        Self
    }

    pub fn run(&self, module: &MirModule<SsaValueId>, alias_graph: &AliasGraph) -> EscapeAnalysis {
        let mut analysis = EscapeAnalysis::new();

        for func in &module.functions {
            self.analyze_function(func, alias_graph, &mut analysis);
        }

        // Iterate until fixed point (since escaping an object might escape its fields, and vice versa)
        // For our simplified model, if object A is stored into object B, and B escapes, A escapes.
        let mut changed = true;
        while changed {
            changed = false;
            for func in &module.functions {
                for block in &func.blocks {
                    for instr in &block.instrs {
                        if let Instruction::StoreField(obj, _, val) = instr {
                            if let Operand::Var(val_src) = val {
                                if let (Some(obj_alias), Some(val_alias)) =
                                    (alias_graph.get_alias(*obj), alias_graph.get_alias(*val_src))
                                {
                                    if analysis.is_escaped(obj_alias)
                                        && !analysis.is_escaped(val_alias)
                                    {
                                        analysis.escaped_aliases.insert(val_alias);
                                        changed = true;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        analysis
    }

    fn analyze_function(
        &self,
        func: &MirFunction<SsaValueId>,
        alias_graph: &AliasGraph,
        analysis: &mut EscapeAnalysis,
    ) {
        for block in &func.blocks {
            for instr in &block.instrs {
                if let Instruction::Call(_, _, args) = instr {
                    // Conservatively assume all arguments passed to a call escape
                    for arg in args {
                        if let Operand::Var(v) = arg {
                            if let Some(alias) = alias_graph.get_alias(*v) {
                                analysis.escaped_aliases.insert(alias);
                            }
                        }
                    }
                }
            }

            if let Terminator::Return(Some(Operand::Var(v))) = &block.terminator {
                // Returned values escape
                if let Some(alias) = alias_graph.get_alias(*v) {
                    analysis.escaped_aliases.insert(alias);
                }
            }
        }
    }
}
