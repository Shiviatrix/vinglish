use std::collections::HashMap;
use eng_mir::{MirFunction, BlockId, Instruction, Operand, Terminator};
use eng_hir::symbol::{VariableId, SymbolId};
use crate::dominators::DominatorTree;

pub struct Renamer {
    pub next_id: u32,
    pub stacks: HashMap<VariableId, Vec<VariableId>>,
}

impl Renamer {
    pub fn new(start_id: u32) -> Self {
        Self {
            next_id: start_id,
            stacks: HashMap::new(),
        }
    }

    pub fn new_name(&mut self, orig: VariableId, locals: &mut Vec<VariableId>) -> VariableId {
        let new_id = VariableId(SymbolId(self.next_id));
        self.next_id += 1;
        self.stacks.entry(orig).or_default().push(new_id);
        locals.push(new_id);
        new_id
    }

    pub fn current_name(&self, orig: VariableId) -> VariableId {
        *self.stacks.get(&orig).and_then(|s| s.last()).unwrap_or(&orig)
    }
}

pub fn rename_variables(func: &mut MirFunction<VariableId>, dom_tree: &DominatorTree, symbol_table: &mut eng_hir::symbol::SymbolTable) {
    let mut max_id = 0;
    for &var in &func.locals {
        if var.0.0 > max_id {
            max_id = var.0.0;
        }
    }
    let mut renamer = Renamer::new(symbol_table.num_symbols() as u32);

    // Initialize params
    for &param in &func.params {
        renamer.stacks.entry(param).or_default().push(param);
    }

    // Build phi origins map
    let mut phi_origins: HashMap<(BlockId, usize), VariableId> = HashMap::new();
    for block in &func.blocks {
        for (i, instr) in block.instrs.iter().enumerate() {
            if let Instruction::<VariableId>::Phi(orig_var, _) = instr {
                phi_origins.insert((block.id, i), *orig_var);
            } else {
                break; // Phis are only at the start
            }
        }
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

    if func.blocks.is_empty() {
        return;
    }
    let entry = func.blocks[0].id;
    rename_block(entry, &mut renamer, func, dom_tree, &succs, &phi_origins, symbol_table);
}

fn rename_block(
    block_id: BlockId,
    renamer: &mut Renamer,
    func: &mut MirFunction<VariableId>,
    dom_tree: &DominatorTree,
    succs: &HashMap<BlockId, Vec<BlockId>>,
    phi_origins: &HashMap<(BlockId, usize), VariableId>,
    symbol_table: &mut eng_hir::symbol::SymbolTable,
) {
    let mut pushed_counts: HashMap<VariableId, usize> = HashMap::new();

    // Split func.blocks borrow
    let block_idx = func.blocks.iter().position(|b| b.id == block_id).unwrap();
    
    // 1. Rename Phi node defs
    for (i, instr) in func.blocks[block_idx].instrs.iter_mut().enumerate() {
        if let Instruction::<VariableId>::Phi(dest, _) = instr {
            let orig = phi_origins.get(&(block_id, i)).copied().unwrap();
            let new_dest = renamer.new_name(orig, &mut func.locals);
            
            // Propagate type to new SSA variable
            let mut ty = eng_hir::types::Type::Unit;
            if let Some(eng_hir::symbol::SymbolKind::Variable(vs)) = symbol_table.get(eng_hir::symbol::SymbolId(orig.0.0)) {
                ty = vs.ty.clone();
            }
            symbol_table.define_var_with_id(
                new_dest.0,
                eng_hir::symbol::VariableSymbol {
                    id: new_dest,
                    name: format!("{}_{}", new_dest.0.0, orig.0.0), // give it some name
                    is_mut: false,
                    ty,
                }
            );
            
            *pushed_counts.entry(orig).or_default() += 1;
            *dest = new_dest;
        } else {
            break;
        }
    }

    // 2. Rename normal instructions
    for instr in &mut func.blocks[block_idx].instrs {
        if let Instruction::<VariableId>::Phi(_, _) = instr {
            continue;
        }

        // Uses
        match instr {
            Instruction::<VariableId>::Assign(_, op) => rename_op(op, renamer),
            Instruction::<VariableId>::LoadField(_, obj, _) => rename_op(obj, renamer),
            Instruction::<VariableId>::StoreField(obj, _, val) => {
                *obj = renamer.current_name(*obj);
                rename_op(val, renamer);
            }
            Instruction::<VariableId>::Call(_, _, args) => {
                for arg in args {
                    rename_op(arg, renamer);
                }
            }
            Instruction::<VariableId>::BinaryOp(_, _, left, right) => {
                rename_op(left, renamer);
                rename_op(right, renamer);
            }
            Instruction::<VariableId>::UnaryOp(_, _, op) |
            Instruction::<VariableId>::Borrow(_, op) |
            Instruction::<VariableId>::BorrowMut(_, op) |
            Instruction::<VariableId>::Deref(_, op, _) => rename_op(op, renamer),
            Instruction::<VariableId>::Drop(var) => *var = renamer.current_name(*var),
            Instruction::<VariableId>::HeapAllocate(_, _) | 
            Instruction::<VariableId>::StackAllocate(_, _) | 
            Instruction::<VariableId>::Phi(_, _) => {}
        }

        // Defs
        if let Instruction::<VariableId>::Assign(dest, _) | Instruction::<VariableId>::BinaryOp(dest, _, _, _) 
            | Instruction::<VariableId>::UnaryOp(dest, _, _) | Instruction::<VariableId>::Call(dest, _, _) 
            | Instruction::<VariableId>::HeapAllocate(dest, _) | Instruction::<VariableId>::StackAllocate(dest, _)
            | Instruction::<VariableId>::Borrow(dest, _)
            | Instruction::<VariableId>::BorrowMut(dest, _)
            | Instruction::<VariableId>::Deref(dest, _, _) = instr {
            let orig = *dest;
            let new_id = renamer.new_name(orig, &mut func.locals);
            
            // Propagate type to new SSA variable
            let mut ty = eng_hir::types::Type::Unit;
            if let Some(eng_hir::symbol::SymbolKind::Variable(vs)) = symbol_table.get(eng_hir::symbol::SymbolId(orig.0.0)) {
                ty = vs.ty.clone();
            }
            symbol_table.define_var_with_id(
                new_id.0,
                eng_hir::symbol::VariableSymbol {
                    id: new_id,
                    name: format!("{}_{}", new_id.0.0, orig.0.0), // give it some name
                    is_mut: false,
                    ty,
                }
            );
            
            *pushed_counts.entry(orig).or_default() += 1;
            *dest = new_id;
        }
    }

    // Terminator<VariableId> uses
    match &mut func.blocks[block_idx].terminator {
        Terminator::<VariableId>::Return(Some(op)) => rename_op(op, renamer),
        Terminator::<VariableId>::Branch(cond, _, _) => rename_op(cond, renamer),
        _ => {}
    }

    // 3. Fill in Phi arguments in successors
    if let Some(block_succs) = succs.get(&block_id) {
        for &succ in block_succs {
            let succ_idx = func.blocks.iter().position(|b| b.id == succ).unwrap();
            let mut i = 0;
            // Note: we can't iterate safely while holding mutable reference to whole block if we don't have to,
            // but we can borrow just `succ_block.instrs`.
            // We use a separate loop or indexed access.
            let len = func.blocks[succ_idx].instrs.len();
            while i < len {
                if let Instruction::<VariableId>::Phi(_, ref mut args) = func.blocks[succ_idx].instrs[i] {
                    let orig_var = phi_origins.get(&(succ, i)).copied().unwrap();
                    let current = renamer.current_name(orig_var);
                    args.push((Operand::<VariableId>::Var(current), block_id));
                    i += 1;
                } else {
                    break;
                }
            }
        }
    }

    // 4. Recursive calls to dominated blocks
    if let Some(children) = dom_tree.children.get(&block_id) {
        for &child in children {
            rename_block(child, renamer, func, dom_tree, succs, phi_origins, symbol_table);
        }
    }

    // 5. Pop off stacks
    for (orig, count) in pushed_counts {
        let stack = renamer.stacks.get_mut(&orig).unwrap();
        for _ in 0..count {
            stack.pop();
        }
    }
}

fn rename_op(op: &mut Operand<VariableId>, renamer: &Renamer) {
    if let Operand::<VariableId>::Var(id) = op {
        *id = renamer.current_name(*id);
    }
}
