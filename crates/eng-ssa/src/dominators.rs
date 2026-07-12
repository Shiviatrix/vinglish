use eng_hir::symbol::VariableId;
use std::collections::{HashMap, HashSet};
use eng_mir::{MirFunction, BlockId, Terminator};

pub struct DominatorTree {
    pub idom: HashMap<BlockId, BlockId>,
    pub children: HashMap<BlockId, Vec<BlockId>>,
    pub dominance_frontiers: HashMap<BlockId, HashSet<BlockId>>,
}

impl DominatorTree {
    pub fn new(func: &MirFunction<VariableId>) -> Self {
        if func.blocks.is_empty() {
            return Self {
                idom: HashMap::new(),
                children: HashMap::new(),
                dominance_frontiers: HashMap::new(),
            };
        }

        let mut preds: HashMap<BlockId, Vec<BlockId>> = HashMap::new();
        let mut succs: HashMap<BlockId, Vec<BlockId>> = HashMap::new();

        for block in &func.blocks {
            preds.entry(block.id).or_default();
            succs.entry(block.id).or_default();
            match &block.terminator {
                Terminator::<VariableId>::Jump(target) => {
                    preds.entry(*target).or_default().push(block.id);
                    succs.entry(block.id).or_default().push(*target);
                }
                Terminator::<VariableId>::Branch(_, true_target, false_target) => {
                    preds.entry(*true_target).or_default().push(block.id);
                    succs.entry(block.id).or_default().push(*true_target);
                    
                    preds.entry(*false_target).or_default().push(block.id);
                    succs.entry(block.id).or_default().push(*false_target);
                }
                Terminator::<VariableId>::Return(_) => {}
            }
        }

        let entry_node = func.blocks[0].id;

        // Post-order traversal to get reverse post-order (RPO)
        let mut rpo = Vec::new();
        let mut visited = HashSet::new();
        Self::post_order(&succs, entry_node, &mut visited, &mut rpo);
        rpo.reverse();

        let mut rpo_index: HashMap<BlockId, usize> = HashMap::new();
        for (i, &node) in rpo.iter().enumerate() {
            rpo_index.insert(node, i);
        }

        let mut idom: HashMap<BlockId, BlockId> = HashMap::new();
        idom.insert(entry_node, entry_node);

        let mut changed = true;
        while changed {
            changed = false;
            for &node in rpo.iter().skip(1) { // skip entry node
                let node_preds = preds.get(&node).unwrap();
                let mut new_idom = None;

                for &p in node_preds {
                    if idom.contains_key(&p) {
                        new_idom = Some(if let Some(n) = new_idom {
                            Self::intersect(&idom, &rpo_index, p, n)
                        } else {
                            p
                        });
                    }
                }

                if let Some(new_idom_node) = new_idom {
                    if idom.get(&node) != Some(&new_idom_node) {
                        idom.insert(node, new_idom_node);
                        changed = true;
                    }
                }
            }
        }

        let mut children: HashMap<BlockId, Vec<BlockId>> = HashMap::new();
        for block in &func.blocks {
            children.insert(block.id, Vec::new());
        }
        for (&node, &dom) in &idom {
            if node != dom { // entry node dominates itself in our setup
                children.entry(dom).or_default().push(node);
            }
        }

        let mut dominance_frontiers: HashMap<BlockId, HashSet<BlockId>> = HashMap::new();
        for block in &func.blocks {
            dominance_frontiers.insert(block.id, HashSet::new());
        }

        for block in &func.blocks {
            if let Some(node_preds) = preds.get(&block.id) {
                if node_preds.len() >= 2 {
                    for &p in node_preds {
                        let mut runner = p;
                        while runner != *idom.get(&block.id).unwrap() {
                            dominance_frontiers.entry(runner).or_default().insert(block.id);
                            if let Some(&next) = idom.get(&runner) {
                                runner = next;
                            } else {
                                break;
                            }
                        }
                    }
                }
            }
        }

        Self {
            idom,
            children,
            dominance_frontiers,
        }
    }

    fn post_order(
        succs: &HashMap<BlockId, Vec<BlockId>>,
        node: BlockId,
        visited: &mut HashSet<BlockId>,
        post_order: &mut Vec<BlockId>,
    ) {
        if !visited.insert(node) {
            return;
        }
        if let Some(node_succs) = succs.get(&node) {
            for &succ in node_succs {
                Self::post_order(succs, succ, visited, post_order);
            }
        }
        post_order.push(node);
    }

    fn intersect(
        idom: &HashMap<BlockId, BlockId>,
        rpo_index: &HashMap<BlockId, usize>,
        mut b1: BlockId,
        mut b2: BlockId,
    ) -> BlockId {
        while b1 != b2 {
            while rpo_index.get(&b1).unwrap_or(&usize::MAX) > rpo_index.get(&b2).unwrap_or(&usize::MAX) {
                b1 = *idom.get(&b1).unwrap();
            }
            while rpo_index.get(&b2).unwrap_or(&usize::MAX) > rpo_index.get(&b1).unwrap_or(&usize::MAX) {
                b2 = *idom.get(&b2).unwrap();
            }
        }
        b1
    }
}
