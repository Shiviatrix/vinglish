use crate::alias::AliasGraph;
use crate::escape::EscapeAnalysis;
use vinglish_hir::symbol::SsaValueId;
use vinglish_mir::{Instruction, MirModule};

pub struct StackPromotionPass;

impl Default for StackPromotionPass {
    fn default() -> Self {
        Self::new()
    }
}

impl StackPromotionPass {
    pub fn new() -> Self {
        Self
    }

    pub fn run(
        &self,
        module: &mut MirModule<SsaValueId>,
        alias_graph: &AliasGraph,
        escape_analysis: &EscapeAnalysis,
    ) {
        for func in &mut module.functions {
            for block in &mut func.blocks {
                for instr in &mut block.instrs {
                    if let Instruction::HeapAllocate(dest, ty) = instr {
                        if let Some(alias) = alias_graph.get_alias(*dest) {
                            if !escape_analysis.is_escaped(alias) {
                                // Safe to promote to stack allocation
                                *instr = Instruction::StackAllocate(*dest, *ty);
                            }
                        }
                    }
                }
            }
        }
    }
}
