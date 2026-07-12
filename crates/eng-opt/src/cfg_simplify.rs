use crate::{OptimizationPass, PassStats};
use eng_mir::{BlockId, MirModule, Terminator};
use std::collections::{HashMap, HashSet};
use std::fmt::Display;
use std::hash::Hash;

pub struct CfgSimplifyPass;

impl<V: Clone + Copy + Display + Eq + Hash> OptimizationPass<V> for CfgSimplifyPass {
    fn name(&self) -> &'static str {
        "CFG Simplification"
    }

    fn run(&mut self, module: &mut MirModule<V>) -> PassStats {
        let mut stats = PassStats::default();

        for func in &mut module.functions {
            let mut changed = true;
            while changed {
                changed = false;

                // 1. Collapse jump chains (Empty block A -> Jump B)
                let mut jump_map = HashMap::new();
                for block in &func.blocks {
                    if block.instrs.is_empty() {
                        if let Terminator::<V>::Jump(target) = block.terminator {
                            if target != block.id {
                                // avoid infinite loop on self-jump
                                jump_map.insert(block.id, target);
                            }
                        }
                    }
                }

                // Resolve jump chains (e.g., A -> B -> C becomes A -> C)
                let mut resolved_jump_map = HashMap::new();
                let keys: Vec<_> = jump_map.keys().copied().collect();
                for src in keys {
                    let mut current = jump_map[&src];
                    let mut visited = HashSet::new();
                    visited.insert(src);
                    while let Some(&next) = jump_map.get(&current) {
                        if !visited.insert(next) {
                            break;
                        } // loop detected
                        current = next;
                    }
                    resolved_jump_map.insert(src, current);
                }

                if !resolved_jump_map.is_empty() {
                    for block in &mut func.blocks {
                        match &mut block.terminator {
                            Terminator::<V>::Jump(target) => {
                                if let Some(&new_target) = resolved_jump_map.get(target) {
                                    *target = new_target;
                                    changed = true;
                                }
                            }
                            Terminator::<V>::Branch(_, true_target, false_target) => {
                                if let Some(&new_target) = resolved_jump_map.get(true_target) {
                                    *true_target = new_target;
                                    changed = true;
                                }
                                if let Some(&new_target) = resolved_jump_map.get(false_target) {
                                    *false_target = new_target;
                                    changed = true;
                                }
                            }
                            _ => {}
                        }
                    }

                    // Phi nodes are not present during cfg_simplify (runs pre-SSA)
                }

                // 2. Merge trivial blocks
                // Compute predecessors
                let mut preds: HashMap<BlockId, Vec<BlockId>> = HashMap::new();
                for block in &func.blocks {
                    let successors = match &block.terminator {
                        Terminator::<V>::Jump(target) => vec![target],
                        Terminator::<V>::Branch(_, true_target, false_target) => {
                            vec![true_target, false_target]
                        }
                        Terminator::<V>::Return(_) => vec![],
                    };
                    for succ in successors {
                        preds.entry(*succ).or_default().push(block.id);
                    }
                }

                // Find a mergeable pair (A -> B where B has only predecessor A, and A ends with Jump(B))
                let mut merge_pair = None;
                for block in &func.blocks {
                    if let Terminator::<V>::Jump(target) = block.terminator {
                        if target != block.id && preds.get(&target).map_or(0, |p| p.len()) == 1 {
                            // Target's only predecessor is `block`
                            // We can't merge if `target` is the entry block (bb0), but entry block has no predecessors if it's bb0!
                            // Actually bb0 has 0 predecessors, so it won't be matched since len() == 1.
                            merge_pair = Some((block.id, target));
                            break;
                        }
                    }
                }

                if let Some((src_id, dst_id)) = merge_pair {
                    // Extract dst block
                    let dst_idx = func.blocks.iter().position(|b| b.id == dst_id).unwrap();
                    let dst_block = func.blocks.remove(dst_idx);

                    // Append to src block
                    let src_idx = func.blocks.iter().position(|b| b.id == src_id).unwrap();
                    let src_block = &mut func.blocks[src_idx];

                    src_block.instrs.extend(dst_block.instrs);
                    src_block.terminator = dst_block.terminator;

                    stats.merged_blocks += 1;
                    changed = true;
                }
            }
        }

        stats
    }
}
