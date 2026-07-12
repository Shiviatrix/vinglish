use eng_hir::symbol::{SymbolId, SymbolKind, SymbolTable, TypeId, VariableId, VariableSymbol};
use eng_hir::{Expr as HirExpr, Item as HirItem, Module as HirModule, Stmt as HirStmt};
use eng_mir::{BasicBlock, BlockId, Instruction, MirFunction, MirModule, Operand, Terminator};

pub struct MirLowerer<'a> {
    symbol_table: &'a mut SymbolTable,
    next_block_id: usize,
    current_block: Option<BlockId>,
    blocks: Vec<BasicBlock<VariableId>>,
    current_instrs: Vec<Instruction<VariableId>>,
    locals: Vec<VariableId>,
}

impl<'a> MirLowerer<'a> {
    pub fn new(symbol_table: &'a mut SymbolTable) -> Self {
        Self {
            symbol_table,
            next_block_id: 0,
            current_block: None,
            blocks: Vec::new(),
            current_instrs: Vec::new(),
            locals: Vec::new(),
        }
    }

    fn new_temp(&mut self, ty: TypeId) -> VariableId {
        let name = format!("_tmp{}", self.locals.len());
        let symbol = VariableSymbol {
            id: VariableId(SymbolId(0)),
            name: name.clone(),
            is_mut: true,
            ty: self
                .symbol_table
                .get_interned_type(ty)
                .cloned()
                .unwrap_or(eng_hir::types::Type::Unit),
        };
        // Define it in the symbol table to get a valid VariableId
        let id = self.symbol_table.define_var(name, symbol);
        self.locals.push(id);
        id
    }

    fn new_block(&mut self) -> BlockId {
        let id = BlockId(self.next_block_id);
        self.next_block_id += 1;
        id
    }

    fn switch_to_block(&mut self, id: BlockId) {
        if let Some(_curr) = self.current_block {
            // If the old block doesn't have a terminator yet, this is an error or we just assume it's unfinished.
            // In a real compiler, we might panic if we switch without terminating.
        }
        self.current_block = Some(id);
    }

    fn end_block(&mut self, terminator: Terminator<VariableId>) {
        if let Some(id) = self.current_block {
            self.blocks.push(BasicBlock {
                id,
                instrs: std::mem::take(&mut self.current_instrs),
                terminator,
            });
            self.current_block = None;
        }
    }

    fn push_instr(&mut self, instr: Instruction<VariableId>) {
        self.current_instrs.push(instr);
    }

    pub fn lower_module(&mut self, hir: &HirModule) -> MirModule<eng_hir::symbol::VariableId> {
        let mut functions = Vec::new();
        for item in &hir.items {
            if let HirItem::Function(f) = item {
                functions.push(self.lower_function(f));
            }
        }
        MirModule { functions }
    }

    fn lower_function(&mut self, f: &eng_hir::FunctionDef) -> MirFunction<VariableId> {
        self.next_block_id = 0;
        self.blocks.clear();
        self.current_instrs.clear();
        self.locals.clear();

        for param in &f.params {
            self.locals.push(param.id);
        }

        let entry = self.new_block();
        self.switch_to_block(entry);

        let ret_val = self.lower_expr(&f.body);

        if self.current_block.is_some() {
            self.end_block(Terminator::Return(Some(ret_val)));
        }

        MirFunction {
            id: f.id,
            is_foreign: f.is_foreign,
            name: f.name.clone(),
            params: f.params.iter().map(|p| p.id).collect(),
            blocks: std::mem::take(&mut self.blocks),
            locals: std::mem::take(&mut self.locals),
        }
    }

    fn lower_expr(&mut self, expr: &HirExpr) -> Operand<VariableId> {
        match expr {
            HirExpr::Lit { value, .. } => Operand::Constant(value.clone()),
            HirExpr::VarRef { id, .. } => Operand::Var(*id),
            HirExpr::Call {
                callee, args, ty, ..
            } => {
                let mut lowered_args = Vec::new();
                for arg in args {
                    lowered_args.push(self.lower_expr(arg));
                }

                let func_id = if let HirExpr::VarRef { id, .. } = &**callee {
                    // Function calls resolve to VarRef(FunctionId) actually because of ScopedId
                    // We need to fetch FunctionId.
                    eng_hir::symbol::FunctionId(id.0)
                } else {
                    // Indirect calls not supported in our simple MIR yet, fallback to dummy
                    eng_hir::symbol::FunctionId(SymbolId(0))
                };

                let temp = self.new_temp(*ty);
                self.push_instr(Instruction::Call(temp, func_id, lowered_args));
                Operand::Var(temp)
            }
            HirExpr::BinOp {
                left,
                op,
                right,
                ty,
                ..
            } => {
                let l = self.lower_expr(left);

                // Handle short-circuiting for And/Or
                if *op == eng_parser::ast::BinOp::And {
                    let rhs_block = self.new_block();
                    let merge_block = self.new_block();
                    let temp = self.new_temp(*ty);

                    self.push_instr(Instruction::Assign(temp, l.clone()));
                    self.end_block(Terminator::Branch(l, rhs_block, merge_block));

                    self.switch_to_block(rhs_block);
                    let r = self.lower_expr(right);
                    self.push_instr(Instruction::Assign(temp, r));
                    self.end_block(Terminator::Jump(merge_block));

                    self.switch_to_block(merge_block);
                    return Operand::Var(temp);
                } else if *op == eng_parser::ast::BinOp::Or {
                    let rhs_block = self.new_block();
                    let merge_block = self.new_block();
                    let temp = self.new_temp(*ty);

                    self.push_instr(Instruction::Assign(temp, l.clone()));
                    self.end_block(Terminator::Branch(l, merge_block, rhs_block));

                    self.switch_to_block(rhs_block);
                    let r = self.lower_expr(right);
                    self.push_instr(Instruction::Assign(temp, r));
                    self.end_block(Terminator::Jump(merge_block));

                    self.switch_to_block(merge_block);
                    return Operand::Var(temp);
                }

                let r = self.lower_expr(right);
                let temp = self.new_temp(*ty);
                self.push_instr(Instruction::BinaryOp(temp, *op, l, r));
                Operand::Var(temp)
            }
            HirExpr::UnOp {
                op, operand, ty, ..
            } => {
                let op_val = self.lower_expr(operand);
                let temp = self.new_temp(*ty);
                match op {
                    eng_parser::ast::UnOp::Borrow(false) => {
                        self.push_instr(Instruction::Borrow(temp, op_val));
                    }
                    eng_parser::ast::UnOp::Borrow(true) => {
                        self.push_instr(Instruction::BorrowMut(temp, op_val));
                    }
                    eng_parser::ast::UnOp::Deref => {
                        self.push_instr(Instruction::Deref(temp, op_val, *ty));
                    }
                    _ => {
                        self.push_instr(Instruction::UnaryOp(temp, *op, op_val));
                    }
                }
                Operand::Var(temp)
            }
            HirExpr::Block(block) => {
                for stmt in &block.stmts {
                    self.lower_stmt(stmt);
                }
                if let Some(e) = &block.expr {
                    self.lower_expr(e)
                } else {
                    Operand::Constant(eng_parser::ast::Literal::Unit)
                }
            }
            HirExpr::StructInit { id, fields, ty, .. } => {
                let temp = self.new_temp(*ty);
                self.push_instr(Instruction::HeapAllocate(temp, *id));
                if let Some(SymbolKind::Type(ts)) = self.symbol_table.get(id.0).cloned() {
                    for (i, fexpr) in fields.iter().enumerate() {
                        if let Some(field_sym) = ts.fields.get(i) {
                            let val = self.lower_expr(fexpr);
                            self.push_instr(Instruction::StoreField(temp, field_sym.id, val));
                        }
                    }
                }
                Operand::Var(temp)
            }
            HirExpr::FieldIndex {
                object,
                field_id,
                ty,
                ..
            } => {
                let obj_op = self.lower_expr(object);
                let temp = self.new_temp(*ty);
                self.push_instr(Instruction::LoadField(temp, obj_op, *field_id));
                Operand::Var(temp)
            }
            _ => Operand::Constant(eng_parser::ast::Literal::Unit),
        }
    }

    fn lower_stmt(&mut self, stmt: &HirStmt) {
        match stmt {
            HirStmt::Let { id, init, .. } => {
                self.locals.push(*id);
                let init_val = self.lower_expr(init);
                self.push_instr(Instruction::Assign(*id, init_val));
            }
            HirStmt::Assign { target, value, .. } => {
                let val = self.lower_expr(value);
                if let HirExpr::VarRef { id, .. } = target {
                    self.push_instr(Instruction::Assign(*id, val));
                } else if let HirExpr::FieldIndex {
                    object, field_id, ..
                } = target
                {
                    let obj_op = self.lower_expr(object);
                    self.push_instr(Instruction::StoreField(
                        if let Operand::Var(v) = obj_op {
                            v
                        } else {
                            VariableId(SymbolId(0))
                        },
                        *field_id,
                        val,
                    ));
                }
            }
            HirStmt::Expr(e) => {
                self.lower_expr(e);
            }
            HirStmt::Return { value, .. } => {
                let ret_val = value.as_ref().map(|v| self.lower_expr(v));
                self.end_block(Terminator::Return(ret_val));
                let new_block = self.new_block();
                self.switch_to_block(new_block);
            }
            HirStmt::If {
                condition,
                then_block,
                otherwise,
                ..
            } => {
                let cond_op = self.lower_expr(condition);
                let then_b = self.new_block();
                let else_b = self.new_block();
                let merge_b = self.new_block();

                if otherwise.is_some() {
                    self.end_block(Terminator::Branch(cond_op, then_b, else_b));
                } else {
                    self.end_block(Terminator::Branch(cond_op, then_b, merge_b));
                }

                self.switch_to_block(then_b);
                for stmt in &then_block.stmts {
                    self.lower_stmt(stmt);
                }
                if let Some(e) = &then_block.expr {
                    self.lower_expr(e);
                }
                if self.current_block.is_some() {
                    self.end_block(Terminator::Jump(merge_b));
                }

                if let Some(other) = otherwise {
                    self.switch_to_block(else_b);
                    for stmt in &other.stmts {
                        self.lower_stmt(stmt);
                    }
                    if let Some(e) = &other.expr {
                        self.lower_expr(e);
                    }
                    if self.current_block.is_some() {
                        self.end_block(Terminator::Jump(merge_b));
                    }
                }

                self.switch_to_block(merge_b);
            }
            HirStmt::RepeatWhile {
                condition, body, ..
            } => {
                let cond_b = self.new_block();
                let body_b = self.new_block();
                let merge_b = self.new_block();

                self.end_block(Terminator::Jump(cond_b));
                self.switch_to_block(cond_b);

                let cond_op = self.lower_expr(condition);
                self.end_block(Terminator::Branch(cond_op, body_b, merge_b));

                self.switch_to_block(body_b);
                for stmt in &body.stmts {
                    self.lower_stmt(stmt);
                }
                if let Some(e) = &body.expr {
                    self.lower_expr(e);
                }
                if self.current_block.is_some() {
                    self.end_block(Terminator::Jump(cond_b));
                }

                self.switch_to_block(merge_b);
            }
        }
    }
}
