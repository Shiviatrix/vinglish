use eng_hir::symbol::SsaValueId;
use eng_mir::{Instruction, MirFunction, MirModule, Operand};
use std::collections::{HashMap, HashSet};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AliasId(pub usize);

impl fmt::Display for AliasId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "alias_{}", self.0)
    }
}

pub struct AliasGraph {
    pub value_to_alias: HashMap<SsaValueId, AliasId>,
    pub alias_to_values: HashMap<AliasId, HashSet<SsaValueId>>,
    pub allocations: HashMap<AliasId, SsaValueId>, // Map AliasId to the original allocation SsaValueId if available
    next_alias_id: usize,
}

impl Default for AliasGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl AliasGraph {
    pub fn new() -> Self {
        Self {
            value_to_alias: HashMap::new(),
            alias_to_values: HashMap::new(),
            allocations: HashMap::new(),
            next_alias_id: 1,
        }
    }

    pub fn new_alias(&mut self) -> AliasId {
        let id = AliasId(self.next_alias_id);
        self.next_alias_id += 1;
        self.alias_to_values.insert(id, HashSet::new());
        id
    }

    pub fn assign_alias(&mut self, value: SsaValueId, alias: AliasId) {
        self.value_to_alias.insert(value, alias);
        self.alias_to_values.get_mut(&alias).unwrap().insert(value);
    }

    pub fn get_alias(&self, value: SsaValueId) -> Option<AliasId> {
        self.value_to_alias.get(&value).copied()
    }
}

impl fmt::Display for AliasGraph {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "--- ALIAS GRAPH ---")?;
        let mut sorted_aliases: Vec<_> = self.alias_to_values.keys().collect();
        sorted_aliases.sort_by_key(|k| k.0);

        for alias in sorted_aliases {
            let values = &self.alias_to_values[alias];
            if values.is_empty() {
                continue;
            }
            let alloc_str = if let Some(alloc_val) = self.allocations.get(alias) {
                format!(" (Allocation: {})", alloc_val)
            } else {
                "".to_string()
            };

            let mut sorted_vals: Vec<_> = values.iter().collect();
            sorted_vals.sort_by_key(|v| v.0);
            let vals_str = sorted_vals
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join(", ");

            writeln!(f, "{}{}: {}", alias, alloc_str, vals_str)?;
        }
        Ok(())
    }
}

pub struct AliasAnalysisPass;

impl Default for AliasAnalysisPass {
    fn default() -> Self {
        Self::new()
    }
}

impl AliasAnalysisPass {
    pub fn new() -> Self {
        Self
    }

    pub fn run(&self, module: &MirModule<SsaValueId>) -> AliasGraph {
        let mut graph = AliasGraph::new();

        for func in &module.functions {
            self.analyze_function(func, &mut graph);
        }

        graph
    }

    fn analyze_function(&self, func: &MirFunction<SsaValueId>, graph: &mut AliasGraph) {
        for &param in &func.params {
            let id = graph.new_alias();
            graph.assign_alias(param, id);
        }

        let mut alias_replacements = HashMap::new();

        for block in &func.blocks {
            for instr in &block.instrs {
                match instr {
                    Instruction::HeapAllocate(dest, _) | Instruction::StackAllocate(dest, _) => {
                        let id = graph.new_alias();
                        graph.assign_alias(*dest, id);
                        graph.allocations.insert(id, *dest);
                    }
                    Instruction::Assign(dest, op)
                    | Instruction::Borrow(dest, op)
                    | Instruction::BorrowMut(dest, op) => {
                        if let Operand::Var(src) = op {
                            if let Some(mut alias) = graph.get_alias(*src) {
                                while let Some(&mapped) = alias_replacements.get(&alias) {
                                    alias = mapped;
                                }
                                graph.assign_alias(*dest, alias);
                            } else {
                                let id = graph.new_alias();
                                graph.assign_alias(*dest, id);
                            }
                        } else {
                            let id = graph.new_alias();
                            graph.assign_alias(*dest, id);
                        }
                    }
                    Instruction::LoadField(dest, obj, _) => {
                        if let Operand::Var(src) = obj {
                            if let Some(mut alias) = graph.get_alias(*src) {
                                while let Some(&mapped) = alias_replacements.get(&alias) {
                                    alias = mapped;
                                }
                                graph.assign_alias(*dest, alias);
                            } else {
                                let id = graph.new_alias();
                                graph.assign_alias(*dest, id);
                            }
                        } else {
                            let id = graph.new_alias();
                            graph.assign_alias(*dest, id);
                        }
                    }
                    Instruction::Call(dest, _, _)
                    | Instruction::BinaryOp(dest, _, _, _)
                    | Instruction::UnaryOp(dest, _, _) => {
                        let id = graph.new_alias();
                        graph.assign_alias(*dest, id);
                    }
                    Instruction::Phi(dest, args) => {
                        let id = graph.new_alias();
                        graph.assign_alias(*dest, id);

                        for (op, _) in args {
                            if let Operand::Var(src) = op {
                                if let Some(mut src_alias) = graph.get_alias(*src) {
                                    while let Some(&mapped) = alias_replacements.get(&src_alias) {
                                        src_alias = mapped;
                                    }
                                    if src_alias != id {
                                        alias_replacements.insert(src_alias, id);
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        let mut final_graph = AliasGraph::new();
        let value_to_alias = graph.value_to_alias.clone();
        for (value, mut alias) in value_to_alias {
            while let Some(&mapped) = alias_replacements.get(&alias) {
                alias = mapped;
            }
            final_graph
                .alias_to_values
                .entry(alias)
                .or_insert_with(HashSet::new);
            final_graph.assign_alias(value, alias);
        }

        let allocations = graph.allocations.clone();
        for (alias, alloc) in allocations {
            let mut resolved = alias;
            while let Some(&mapped) = alias_replacements.get(&resolved) {
                resolved = mapped;
            }
            final_graph.allocations.insert(resolved, alloc);
        }

        *graph = final_graph;
    }
}
