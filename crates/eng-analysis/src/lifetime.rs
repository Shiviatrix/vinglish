use std::collections::HashMap;
use std::fmt;
use eng_hir::symbol::SsaValueId;
use eng_mir::{BlockId, MirFunction, MirModule, Operand};
use crate::alias::{AliasGraph, AliasId};

#[derive(Debug, Clone)]
pub struct Lifetime {
    pub created_at: Option<(BlockId, usize)>, // (BlockId, Instruction Index)
    pub last_used_at: Option<(BlockId, usize)>, // For terminators, index is blocks.instructions.len()
}

pub struct LifetimeGraph {
    pub alias_lifetimes: HashMap<AliasId, Lifetime>,
}

impl Default for LifetimeGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl LifetimeGraph {
    pub fn new() -> Self {
        Self {
            alias_lifetimes: HashMap::new(),
        }
    }
}

impl fmt::Display for LifetimeGraph {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "--- LIFETIME GRAPH ---")?;
        let mut sorted_aliases: Vec<_> = self.alias_lifetimes.keys().copied().collect();
        sorted_aliases.sort_by_key(|a| a.0);
        
        for alias in sorted_aliases {
            let lifetime = &self.alias_lifetimes[&alias];
            let created_str = if let Some((b, i)) = lifetime.created_at {
                format!("{}:{}", b, i)
            } else {
                "unknown".to_string()
            };
            let last_used_str = if let Some((b, i)) = lifetime.last_used_at {
                format!("{}:{}", b, i)
            } else {
                "unknown".to_string()
            };
            
            writeln!(f, "{} created: {} last_used: {}", alias, created_str, last_used_str)?;
        }
        Ok(())
    }
}

pub struct LifetimeAnalysisPass;

impl Default for LifetimeAnalysisPass {
    fn default() -> Self {
        Self::new()
    }
}

impl LifetimeAnalysisPass {
    pub fn new() -> Self {
        Self
    }

    pub fn run(&self, module: &MirModule<SsaValueId>, alias_graph: &AliasGraph) -> LifetimeGraph {
        let mut graph = LifetimeGraph::new();

        for func in &module.functions {
            self.analyze_function(func, alias_graph, &mut graph);
        }

        graph
    }

    fn analyze_function(&self, func: &MirFunction<SsaValueId>, alias_graph: &AliasGraph, graph: &mut LifetimeGraph) {
        // Find creation points
        for block in &func.blocks {
            for (idx, instr) in block.instrs.iter().enumerate() {
                let dests = match instr {
                    eng_mir::Instruction::HeapAllocate(dest, _) |
                    eng_mir::Instruction::StackAllocate(dest, _) => Some(*dest),
                    // In a more complete analysis, we would track creations of all aliases,
                    // but memory allocations are the most critical.
                    _ => None,
                };

                if let Some(dest) = dests {
                    if let Some(alias) = alias_graph.get_alias(dest) {
                        let entry = graph.alias_lifetimes.entry(alias).or_insert(Lifetime { created_at: None, last_used_at: None });
                        if entry.created_at.is_none() {
                            entry.created_at = Some((block.id, idx));
                        }
                    }
                }
            }
        }

        // Find last uses
        // A simple approach is just tracking the lexically last use in the CFG traversal.
        // For accurate lifetimes with branches, we should do a backward liveness analysis.
        // For the scope of Stage 2.7, we approximate it by finding the last block/idx where it is referenced.
        for block in &func.blocks {
            for (idx, instr) in block.instrs.iter().enumerate() {
                let mut uses = Vec::new();
                match instr {
                    eng_mir::Instruction::Assign(_, op) |
                    eng_mir::Instruction::UnaryOp(_, _, op) |
                    eng_mir::Instruction::Borrow(_, op) |
                    eng_mir::Instruction::BorrowMut(_, op) |
                    eng_mir::Instruction::LoadField(_, op, _) => {
                        if let Operand::Var(v) = op { uses.push(*v); }
                    }
                    eng_mir::Instruction::StoreField(obj, _, val) => {
                        uses.push(*obj);
                        if let Operand::Var(v) = val { uses.push(*v); }
                    }
                    eng_mir::Instruction::BinaryOp(_, _, left, right) => {
                        if let Operand::Var(v) = left { uses.push(*v); }
                        if let Operand::Var(v) = right { uses.push(*v); }
                    }
                    eng_mir::Instruction::Call(_, _, args) => {
                        for arg in args {
                            if let Operand::Var(v) = arg { uses.push(*v); }
                        }
                    }
                    eng_mir::Instruction::Phi(_, args) => {
                        for (op, _) in args {
                            if let Operand::Var(v) = op { uses.push(*v); }
                        }
                    }
                    _ => {}
                }

                for u in uses {
                    if let Some(alias) = alias_graph.get_alias(u) {
                        let entry = graph.alias_lifetimes.entry(alias).or_insert(Lifetime { created_at: None, last_used_at: None });
                        // Update last used. This is a very rough approximation.
                        entry.last_used_at = Some((block.id, idx));
                    }
                }
            }

            match &block.terminator {
                eng_mir::Terminator::Return(Some(Operand::Var(v))) => {
                    if let Some(alias) = alias_graph.get_alias(*v) {
                        let entry = graph.alias_lifetimes.entry(alias).or_insert(Lifetime { created_at: None, last_used_at: None });
                        entry.last_used_at = Some((block.id, block.instrs.len()));
                    }
                }
                eng_mir::Terminator::Branch(Operand::Var(v), _, _) => {
                    if let Some(alias) = alias_graph.get_alias(*v) {
                        let entry = graph.alias_lifetimes.entry(alias).or_insert(Lifetime { created_at: None, last_used_at: None });
                        entry.last_used_at = Some((block.id, block.instrs.len()));
                    }
                }
                _ => {}
            }
        }
    }
}
