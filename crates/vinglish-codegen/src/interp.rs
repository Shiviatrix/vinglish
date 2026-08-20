use std::collections::HashMap;
use std::fmt;

use vinglish_hir::symbol::{FieldId, FunctionId, SsaValueId, SymbolTable};
use vinglish_mir::{BlockId, Instruction, MirFunction, MirModule, Operand, Terminator};
use vinglish_parser::ast::{BinOp, Literal, UnOp};

// ─────────────────────────────────────────────────────────────────────────────
// Value
// ─────────────────────────────────────────────────────────────────────────────

use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Int(i64),
    Float(f64),
    Bool(bool),
    Text(String),
    Unit,
    List(Rc<RefCell<Vec<Value>>>),
    Function(FunctionId),
    NativeFunction(NativeFn),
    Struct(Rc<RefCell<HashMap<FieldId, Value>>>),
    Return(Box<Value>),
    Reference(ReferenceLoc),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ReferenceLoc {
    Local(SsaValueId),
    ListElement(Rc<RefCell<Vec<Value>>>, usize),
    StructField(Rc<RefCell<HashMap<FieldId, Value>>>, FieldId),
}

pub type NativeFnPointer = fn(Vec<Value>) -> Result<Value, InterpError>;

#[derive(Clone)]
pub struct NativeFn {
    pub name: &'static str,
    pub f: NativeFnPointer,
}

impl fmt::Debug for NativeFn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<native:{}>", self.name)
    }
}

impl PartialEq for NativeFn {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl Value {
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::Int(i) => *i != 0,
            Value::Unit => false,
            _ => true,
        }
    }

    pub fn to_display(&self) -> String {
        match self {
            Value::Int(i) => i.to_string(),
            Value::Float(f) => {
                if f.fract() == 0.0 {
                    format!("{:.1}", f)
                } else {
                    f.to_string()
                }
            }
            Value::Bool(b) => b.to_string(),
            Value::Text(s) => s.clone(),
            Value::Unit => "()".to_string(),
            Value::List(addr) => {
                let inner: Vec<_> = addr.borrow().iter().map(|v| v.to_display()).collect();
                format!("[{}]", inner.join(", "))
            }
            Value::Struct(_) => "<struct>".to_string(),
            Value::Function(_) => "<function>".to_string(),
            Value::NativeFunction(nf) => format!("<native:{}>", nf.name),
            Value::Return(v) => v.to_display(),
            Value::Reference(loc) => match loc {
                ReferenceLoc::Local(id) => format!("&var_{}", id.0),
                ReferenceLoc::ListElement(addr, idx) => {
                    format!("&list_{:x}[{}]", Rc::as_ptr(addr) as usize, idx)
                }
                ReferenceLoc::StructField(addr, fid) => {
                    format!("&struct_{:x}.field_{}", Rc::as_ptr(addr) as usize, fid.0)
                }
            },
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Errors
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct InterpError {
    pub message: String,
}

impl InterpError {
    pub fn new(msg: impl Into<String>) -> Self {
        Self {
            message: msg.into(),
        }
    }
}

impl fmt::Display for InterpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "runtime error: {}", self.message)
    }
}

impl std::error::Error for InterpError {}

// ─────────────────────────────────────────────────────────────────────────────
// Interpreter
// ─────────────────────────────────────────────────────────────────────────────
pub trait DebuggerHook {
    fn on_instruction(
        &mut self,
        func: &MirFunction<SsaValueId>,
        block: &vinglish_mir::BasicBlock<SsaValueId>,
        instr_idx: usize,
        locals: &HashMap<SsaValueId, Value>,
    ) -> Result<(), InterpError>;
}

pub struct Interpreter<'a> {
    _symbol_table: &'a SymbolTable,
    functions: HashMap<FunctionId, &'a MirFunction<SsaValueId>>,
    native_functions: HashMap<FunctionId, NativeFn>,
    libraries: Vec<libloading::Library>,
    pub debugger_hook: Option<std::rc::Rc<std::cell::RefCell<dyn DebuggerHook + 'a>>>,
}

impl<'a> Interpreter<'a> {
    pub fn new(symbol_table: &'a SymbolTable) -> Self {
        let mut interp = Self {
            _symbol_table: symbol_table,
            functions: HashMap::new(),
            native_functions: HashMap::new(),
            libraries: Vec::new(),
            debugger_hook: None,
        };

        let builtins: Vec<(&'static str, NativeFnPointer)> = vec![
            ("print", |args| {
                if let Some(val) = args.first() {
                    print!("{}", val.to_display());
                }
                Ok(Value::Unit)
            }),
            ("println", |args| {
                if let Some(val) = args.first() {
                    println!("{}", val.to_display());
                } else {
                    println!();
                }
                Ok(Value::Unit)
            }),
            ("to_text", |args| {
                if let Some(val) = args.first() {
                    Ok(Value::Text(val.to_display()))
                } else {
                    Ok(Value::Text("".to_string()))
                }
            }),
            ("to_number", |args| {
                if let Some(Value::Text(s)) = args.first() {
                    if let Ok(i) = s.parse::<i64>() {
                        Ok(Value::Int(i))
                    } else {
                        Ok(Value::Int(0))
                    }
                } else {
                    Ok(Value::Int(0))
                }
            }),
            ("min", |args| {
                if let (Some(Value::Int(a)), Some(Value::Int(b))) = (args.first(), args.get(1)) {
                    Ok(Value::Int(*a.min(b)))
                } else {
                    Ok(Value::Int(0))
                }
            }),
            ("max", |args| {
                if let (Some(Value::Int(a)), Some(Value::Int(b))) = (args.first(), args.get(1)) {
                    Ok(Value::Int(*a.max(b)))
                } else {
                    Ok(Value::Int(0))
                }
            }),
            ("abs", |args| {
                if let Some(Value::Int(a)) = args.first() {
                    Ok(Value::Int(a.abs()))
                } else {
                    Ok(Value::Int(0))
                }
            }),
        ];

        for (name, f) in builtins {
            if let Some(id) = symbol_table.lookup(name) {
                interp
                    .native_functions
                    .insert(vinglish_hir::symbol::FunctionId(id), NativeFn { name, f });
            }
        }

        interp
    }

    pub fn load_dynamic_library(
        &mut self,
        path: &std::path::Path,
    ) -> Result<(), Box<dyn std::error::Error>> {
        unsafe {
            let lib = libloading::Library::new(path)?;
            self.libraries.push(lib);
        }
        Ok(())
    }

    pub fn run_module(
        &mut self,
        module: &'a MirModule<vinglish_hir::symbol::SsaValueId>,
    ) -> Result<(), InterpError> {
        for func in &module.functions {
            self.functions.insert(func.id, func);

            if func.is_foreign {
                let native_fn = match func.name.as_str() {
                    "puts" => Some(NativeFn {
                        name: "puts",
                        f: |args| {
                            if let Some(Value::Text(s)) = args.first() {
                                println!("{}", s);
                                Ok(Value::Int(0))
                            } else {
                                Err(InterpError::new("puts: expected string"))
                            }
                        },
                    }),
                    "string_concat" => Some(NativeFn {
                        name: "string_concat",
                        f: |args| {
                            if args.len() == 2
                                && let (Value::Text(a), Value::Text(b)) = (&args[0], &args[1])
                            {
                                return Ok(Value::Text(format!("{}{}", a, b)));
                            }
                            Err(InterpError::new("string_concat: expected two strings"))
                        },
                    }),
                    "string_len" => Some(NativeFn {
                        name: "string_len",
                        f: |args| {
                            if args.len() == 1
                                && let Value::Text(a) = &args[0]
                            {
                                return Ok(Value::Int(a.len() as i64));
                            }
                            Err(InterpError::new("string_len: expected string"))
                        },
                    }),
                    _ => None,
                };
                if let Some(nf) = native_fn {
                    self.native_functions.insert(func.id, nf);
                }
            }
        }

        let mut main_id = None;
        for func in &module.functions {
            if func.name == "main" {
                main_id = Some(func.id);
                break;
            }
        }

        if let Some(id) = main_id {
            self.call_function(id, vec![])?;
        }

        Ok(())
    }

    fn call_function(&self, id: FunctionId, args: Vec<Value>) -> Result<Value, InterpError> {
        if let Some(nf) = self.native_functions.get(&id).cloned() {
            return (nf.f)(args);
        }

        let func = self
            .functions
            .get(&id)
            .ok_or_else(|| InterpError::new("Function not found"))?;

        let mut locals = HashMap::new();
        // Bind arguments to parameters (the first N locals are parameters)
        for (i, arg) in args.into_iter().enumerate() {
            if i < func.locals.len() {
                locals.insert(func.locals[i], arg);
            }
        }

        if func.blocks.is_empty() {
            return Ok(Value::Unit);
        }

        let mut current_block_id = func.blocks[0].id;
        let mut previous_block_id = current_block_id; // For the first block, Phi nodes shouldn't exist, so this is safe.

        loop {
            let block = func
                .blocks
                .iter()
                .find(|b| b.id == current_block_id)
                .ok_or_else(|| {
                    InterpError::new(format!("Block {} not found", current_block_id.0))
                })?;

            for (instr_idx, instr) in block.instrs.iter().enumerate() {
                if let Some(hook) = &self.debugger_hook {
                    hook.borrow_mut()
                        .on_instruction(func, block, instr_idx, &locals)?;
                }
                self.exec_instr(instr, &mut locals, previous_block_id)?;
            }

            previous_block_id = current_block_id;

            match &block.terminator {
                Terminator::<SsaValueId>::Return(opt_op) => {
                    return match opt_op {
                        Some(op) => self.eval_operand(op, &locals),
                        None => Ok(Value::Unit),
                    };
                }
                Terminator::<SsaValueId>::Jump(target) => {
                    current_block_id = *target;
                }
                Terminator::<SsaValueId>::Branch(cond, true_b, false_b) => {
                    let cond_val = self.eval_operand(cond, &locals)?;
                    if cond_val.is_truthy() {
                        current_block_id = *true_b;
                    } else {
                        current_block_id = *false_b;
                    }
                }
            }
        }
    }

    fn exec_instr(
        &self,
        instr: &Instruction<SsaValueId>,
        locals: &mut HashMap<SsaValueId, Value>,
        previous_block: BlockId,
    ) -> Result<(), InterpError> {
        match instr {
            Instruction::<SsaValueId>::Assign(dest, op) => {
                let val = self.eval_operand(op, locals)?;
                locals.insert(*dest, val);
            }
            Instruction::<SsaValueId>::CallIntrinsic(dest, _name, _args) => {
                // Dummy for now
                locals.insert(*dest, Value::Unit);
            }
            Instruction::<SsaValueId>::LoadField(dest, obj_op, field_id) => {
                let mut obj = self.eval_operand(obj_op, locals)?;
                loop {
                    let new_val = match &obj {
                        Value::Reference(ReferenceLoc::Local(v)) => {
                            locals.get(v).cloned().unwrap_or(Value::Unit)
                        }
                        Value::Reference(ReferenceLoc::ListElement(a, i)) => a.borrow()[*i].clone(),
                        Value::Reference(ReferenceLoc::StructField(_, _)) => {
                            return Err(InterpError::new("Struct field deref unsupported"));
                        }
                        _ => break,
                    };
                    obj = new_val;
                }
                if let Value::Struct(struct_id) = obj {
                    let val = struct_id
                        .borrow()
                        .get(&field_id.field_id)
                        .cloned()
                        .unwrap_or(Value::Unit);
                    locals.insert(*dest, val);
                } else {
                    return Err(InterpError::new("Cannot load field from non-struct"));
                }
            }
            Instruction::<SsaValueId>::StoreField(obj_var, field_id, val_op) => {
                let val = self.eval_operand(val_op, locals)?;
                let mut obj_val = locals
                    .get(obj_var)
                    .cloned()
                    .ok_or_else(|| InterpError::new("Variable not found"))?;
                loop {
                    let new_val = match &obj_val {
                        Value::Reference(ReferenceLoc::Local(v)) => {
                            locals.get(v).cloned().unwrap_or(Value::Unit)
                        }
                        Value::Reference(ReferenceLoc::ListElement(a, i)) => a.borrow()[*i].clone(),
                        Value::Reference(ReferenceLoc::StructField(_, _)) => {
                            return Err(InterpError::new("Struct field deref unsupported"));
                        }
                        _ => break,
                    };
                    obj_val = new_val;
                }
                if let Value::Struct(struct_id) = obj_val {
                    struct_id.borrow_mut().insert(field_id.field_id, val);
                } else {
                    return Err(InterpError::new("Cannot store field to non-struct"));
                }
            }
            Instruction::<SsaValueId>::Call(dest, func_id, arg_ops) => {
                let mut args = Vec::new();
                for arg_op in arg_ops {
                    args.push(self.eval_operand(arg_op, locals)?);
                }
                let func_id = match func_id {
                    vinglish_mir::CallTarget::Direct(id) => *id,
                    vinglish_mir::CallTarget::Foreign { c_symbol } => {
                        let mut matched_id = None;
                        for (id, nf) in &self.native_functions {
                            if nf.name == c_symbol {
                                matched_id = Some(*id);
                                break;
                            }
                        }
                        if let Some(id) = matched_id {
                            id
                        } else {
                            // Try dynamically loaded libraries
                            let mut dynamic_fn: Option<NativeFnPointer> = None;
                            for lib in &self.libraries {
                                unsafe {
                                    // First try ving_<name> (Vinglish std library convention)
                                    let symbol_name = format!("ving_{}\0", c_symbol);
                                    if let Ok(sym) =
                                        lib.get::<NativeFnPointer>(symbol_name.as_bytes())
                                    {
                                        dynamic_fn = Some(*sym);
                                        break;
                                    }
                                    // Fallback: try raw symbol name (for external C/Rust/Zig libraries)
                                    let raw_name = format!("{}\0", c_symbol);
                                    if let Ok(sym) = lib.get::<NativeFnPointer>(raw_name.as_bytes())
                                    {
                                        dynamic_fn = Some(*sym);
                                        break;
                                    }
                                }
                            }
                            if let Some(f) = dynamic_fn {
                                let ret = (f)(args)?;
                                locals.insert(*dest, ret);
                                return Ok(());
                            }
                            return Err(InterpError::new(format!(
                                "cannot interpret foreign call `{c_symbol}`"
                            )));
                        }
                    }
                };
                let ret = self.call_function(func_id, args)?;
                locals.insert(*dest, ret);
            }
            Instruction::<SsaValueId>::HeapAllocate(dest, _ty)
            | Instruction::<SsaValueId>::StackAllocate(dest, _ty) => {
                locals.insert(*dest, Value::Struct(Rc::new(RefCell::new(HashMap::new()))));
            }
            Instruction::<SsaValueId>::BinaryOp(dest, op, left_op, right_op) => {
                let left = self.eval_operand(left_op, locals)?;
                let right = self.eval_operand(right_op, locals)?;
                let val = self.eval_binop(*op, left, right)?;
                locals.insert(*dest, val);
            }
            Instruction::<SsaValueId>::UnaryOp(dest, op, operand) => {
                let val = self.eval_operand(operand, locals)?;
                let res = match op {
                    UnOp::Neg => match val {
                        Value::Int(i) => Value::Int(-i),
                        Value::Float(f) => Value::Float(-f),
                        _ => return Err(InterpError::new("Cannot negate non-number")),
                    },
                    UnOp::Not => Value::Bool(!val.is_truthy()),
                    UnOp::Borrow(_) | UnOp::Deref => {
                        return Err(InterpError::new("borrow/deref UnOp should be lowered to Instruction::Borrow/Instruction::Deref"))
                    }
                };
                locals.insert(*dest, res);
            }
            Instruction::<SsaValueId>::Borrow(dest, op)
            | Instruction::<SsaValueId>::BorrowMut(dest, op) => {
                // For borrow, we create a reference to the variable given by op.
                // We only support borrowing variables for now.
                let reference = if let Operand::Var(var_id) = op {
                    Value::Reference(ReferenceLoc::Local(*var_id))
                } else {
                    return Err(InterpError::new(format!(
                        "Borrow of non-variable operand not yet implemented: {:?}",
                        op
                    )));
                };
                locals.insert(*dest, reference);
            }
            Instruction::<SsaValueId>::Deref(dest, op, _) => {
                let val = self.eval_operand(op, locals)?;
                if let Value::Reference(loc) = val {
                    let deref_val = match loc {
                        ReferenceLoc::Local(var) => {
                            locals.get(&var).cloned().unwrap_or(Value::Unit)
                        }
                        ReferenceLoc::ListElement(addr, idx) => {
                            addr.borrow().get(idx).cloned().unwrap_or(Value::Unit)
                        }
                        ReferenceLoc::StructField(addr, fid) => {
                            let borrow = addr.borrow();
                            borrow.get(&fid).cloned().unwrap_or(Value::Unit)
                        }
                    };
                    locals.insert(*dest, deref_val);
                } else {
                    return Err(InterpError::new(format!(
                        "Cannot deref non-reference value: {:?}",
                        val
                    )));
                }
            }
            Instruction::<SsaValueId>::StoreDeref(ptr_op, val_op) => {
                let ptr = self.eval_operand(ptr_op, locals)?;
                let val = self.eval_operand(val_op, locals)?;
                if let Value::Reference(loc) = ptr {
                    match loc {
                        ReferenceLoc::Local(var) => {
                            locals.insert(var, val);
                        }
                        ReferenceLoc::ListElement(addr, idx) => {
                            if idx < addr.borrow().len() {
                                addr.borrow_mut()[idx] = val;
                            } else {
                                return Err(InterpError::new(
                                    "List index out of bounds in StoreDeref",
                                ));
                            }
                        }
                        ReferenceLoc::StructField(addr, fid) => {
                            addr.borrow_mut().insert(fid, val);
                        }
                    }
                } else {
                    return Err(InterpError::new(format!(
                        "Cannot store deref non-reference value: {:?}",
                        ptr
                    )));
                }
            }
            Instruction::<SsaValueId>::Drop(var) => {
                // Remove the variable from the environment.
                // Rust's Rc drops it automatically if this is the last reference.
                locals.remove(var);
            }
            Instruction::<SsaValueId>::Phi(dest, args) => {
                let mut resolved = false;
                for (op, block_id) in args {
                    if *block_id == previous_block {
                        let val = self.eval_operand(op, locals)?;
                        locals.insert(*dest, val);
                        resolved = true;
                        break;
                    }
                }
                if !resolved {
                    return Err(InterpError::new(format!(
                        "Phi node has no argument for predecessor block {}",
                        previous_block.0
                    )));
                }
            }
            Instruction::<SsaValueId>::ListNew(dest, cap_op) => {
                let _cap = self.eval_operand(cap_op, locals)?;
                locals.insert(*dest, Value::List(Rc::new(RefCell::new(Vec::new()))));
            }
            Instruction::<SsaValueId>::ListGet(dest, list_op, idx_op) => {
                let list_val = self.eval_operand(list_op, locals)?;
                let idx_val = self.eval_operand(idx_op, locals)?;

                let idx = match idx_val {
                    Value::Int(i) => i as usize,
                    _ => return Err(InterpError::new("List index must be an integer")),
                };

                let mut actual_list = list_val;
                loop {
                    let new_val = match &actual_list {
                        Value::Reference(ReferenceLoc::Local(v)) => {
                            locals.get(v).cloned().unwrap_or(Value::Unit)
                        }
                        Value::Reference(ReferenceLoc::ListElement(a, i)) => a.borrow()[*i].clone(),
                        Value::Reference(ReferenceLoc::StructField(_, _)) => {
                            return Err(InterpError::new("Struct field deref unsupported"));
                        }
                        _ => break,
                    };
                    actual_list = new_val;
                }

                if let Value::List(addr) = actual_list {
                    if idx < addr.borrow().len() {
                        let got = std::mem::replace(&mut addr.borrow_mut()[idx], Value::Unit);
                        locals.insert(*dest, got);
                    } else {
                        return Err(InterpError::new(format!("Index {} out of bounds", idx)));
                    }
                } else {
                    return Err(InterpError::new("Cannot get from non-list"));
                }
            }
            Instruction::<SsaValueId>::ListBorrowGet(dest, list_op, idx_op)
            | Instruction::<SsaValueId>::ListBorrowMutGet(dest, list_op, idx_op) => {
                let list_val = self.eval_operand(list_op, locals)?;
                let idx_val = self.eval_operand(idx_op, locals)?;

                let idx = match idx_val {
                    Value::Int(i) => i as usize,
                    _ => return Err(InterpError::new("List index must be an integer")),
                };

                let mut actual_list = list_val;
                loop {
                    let new_val = match &actual_list {
                        Value::Reference(ReferenceLoc::Local(v)) => locals.get(v).cloned().unwrap(),
                        Value::Reference(ReferenceLoc::ListElement(a, i)) => a.borrow()[*i].clone(),
                        Value::Reference(ReferenceLoc::StructField(_, _)) => {
                            return Err(InterpError::new("Struct field not supported"));
                        }
                        _ => break,
                    };
                    actual_list = new_val;
                }

                if let Value::List(addr) = actual_list {
                    if idx < addr.borrow().len() {
                        locals.insert(
                            *dest,
                            Value::Reference(ReferenceLoc::ListElement(addr, idx)),
                        );
                    } else {
                        return Err(InterpError::new(format!("Index {} out of bounds", idx)));
                    }
                } else {
                    return Err(InterpError::new("Cannot get from non-list"));
                }
            }
            Instruction::<SsaValueId>::ListSet(list_op, idx_op, val_op) => {
                let list_val = self.eval_operand(list_op, locals)?;
                let idx_val = self.eval_operand(idx_op, locals)?;
                let idx = match idx_val {
                    Value::Int(i) => i as usize,
                    _ => return Err(InterpError::new("List index must be an integer")),
                };
                let val = self.eval_operand(val_op, locals)?;

                let mut actual_list = list_val;
                loop {
                    let new_val = match &actual_list {
                        Value::Reference(ReferenceLoc::Local(v)) => {
                            locals.get(v).cloned().unwrap_or(Value::Unit)
                        }
                        Value::Reference(ReferenceLoc::ListElement(a, i)) => a.borrow()[*i].clone(),
                        Value::Reference(ReferenceLoc::StructField(_, _)) => {
                            return Err(InterpError::new("Struct field deref unsupported"));
                        }
                        _ => break,
                    };
                    actual_list = new_val;
                }

                if let Value::List(addr) = actual_list {
                    if idx < addr.borrow().len() {
                        addr.borrow_mut()[idx] = val;
                    } else {
                        return Err(InterpError::new(format!("Index {} out of bounds", idx)));
                    }
                } else {
                    return Err(InterpError::new("Cannot set non-list"));
                }
            }
            Instruction::<SsaValueId>::ListLen(dest, list_op) => {
                let list_val = self.eval_operand(list_op, locals)?;
                let mut actual_list = list_val;
                loop {
                    let new_val = match &actual_list {
                        Value::Reference(ReferenceLoc::Local(v)) => {
                            locals.get(v).cloned().unwrap_or(Value::Unit)
                        }
                        Value::Reference(ReferenceLoc::ListElement(a, i)) => a.borrow()[*i].clone(),
                        Value::Reference(ReferenceLoc::StructField(_, _)) => {
                            return Err(InterpError::new("Struct field deref unsupported"));
                        }
                        _ => break,
                    };
                    actual_list = new_val;
                }

                if let Value::List(addr) = actual_list {
                    locals.insert(*dest, Value::Int(addr.borrow().len() as i64));
                } else {
                    return Err(InterpError::new("Cannot get len of non-list"));
                }
            }
            Instruction::<SsaValueId>::ListPush(list_op, val_op) => {
                let list_val = self.eval_operand(list_op, locals)?;
                let val = self.eval_operand(val_op, locals)?;

                let mut actual_list = list_val;
                loop {
                    let new_val = match &actual_list {
                        Value::Reference(ReferenceLoc::Local(v)) => {
                            locals.get(v).cloned().unwrap_or(Value::Unit)
                        }
                        Value::Reference(ReferenceLoc::ListElement(a, i)) => a.borrow()[*i].clone(),
                        Value::Reference(ReferenceLoc::StructField(_, _)) => {
                            return Err(InterpError::new("Struct field deref unsupported"));
                        }
                        _ => break,
                    };
                    actual_list = new_val;
                }

                if let Value::List(addr) = actual_list {
                    addr.borrow_mut().push(val);
                } else {
                    return Err(InterpError::new("Cannot push to non-list"));
                }
            }
            Instruction::<SsaValueId>::ListPop(dest, list_op) => {
                let list_val = self.eval_operand(list_op, locals)?;

                let mut actual_list = list_val;
                loop {
                    let new_val = match &actual_list {
                        Value::Reference(ReferenceLoc::Local(v)) => {
                            locals.get(v).cloned().unwrap_or(Value::Unit)
                        }
                        Value::Reference(ReferenceLoc::ListElement(a, i)) => a.borrow()[*i].clone(),
                        Value::Reference(ReferenceLoc::StructField(_, _)) => {
                            return Err(InterpError::new("Struct field deref unsupported"));
                        }
                        _ => break,
                    };
                    actual_list = new_val;
                }

                if let Value::List(addr) = actual_list {
                    let popped = addr.borrow_mut().pop().unwrap_or(Value::Unit);
                    locals.insert(*dest, popped);
                } else {
                    return Err(InterpError::new("Cannot pop from non-list"));
                }
            }
        }
        Ok(())
    }

    fn eval_operand(
        &self,
        op: &Operand<SsaValueId>,
        locals: &HashMap<SsaValueId, Value>,
    ) -> Result<Value, InterpError> {
        match op {
            Operand::<SsaValueId>::Constant(lit) => Ok(match lit {
                Literal::Int(i) => Value::Int(*i),
                Literal::Float(f) => Value::Float(*f),
                Literal::Bool(b) => Value::Bool(*b),
                Literal::Text(s) => Value::Text(s.clone()),
                Literal::Unit => Value::Unit,
            }),
            Operand::<SsaValueId>::Var(id) => locals
                .get(id)
                .cloned()
                .ok_or_else(|| InterpError::new(format!("Variable {} not found", id.0))),
        }
    }

    fn eval_binop(&self, op: BinOp, lv: Value, rv: Value) -> Result<Value, InterpError> {
        use Value::*;
        match (&lv, op, &rv) {
            (Int(a), BinOp::Add, Int(b)) => Ok(Int(a + b)),
            (Int(a), BinOp::Sub, Int(b)) => Ok(Int(a - b)),
            (Int(a), BinOp::Mul, Int(b)) => Ok(Int(a * b)),
            (Int(a), BinOp::Div, Int(b)) => {
                if *b == 0 {
                    Err(InterpError::new("division by zero"))
                } else {
                    Ok(Int(a / b))
                }
            }
            (Int(a), BinOp::Mod, Int(b)) => Ok(Int(a % b)),
            (Float(a), BinOp::Add, Float(b)) => Ok(Float(a + b)),
            (Float(a), BinOp::Sub, Float(b)) => Ok(Float(a - b)),
            (Float(a), BinOp::Mul, Float(b)) => Ok(Float(a * b)),
            (Float(a), BinOp::Div, Float(b)) => Ok(Float(a / b)),
            (Int(a), BinOp::Add, Float(b)) => Ok(Float(*a as f64 + b)),
            (Float(a), BinOp::Add, Int(b)) => Ok(Float(a + *b as f64)),
            (Int(a), BinOp::Mul, Float(b)) => Ok(Float(*a as f64 * b)),
            (Float(a), BinOp::Mul, Int(b)) => Ok(Float(a * *b as f64)),
            (Int(a), BinOp::Sub, Float(b)) => Ok(Float(*a as f64 - b)),
            (Float(a), BinOp::Sub, Int(b)) => Ok(Float(a - *b as f64)),
            (Text(a), BinOp::Add, Text(b)) => Ok(Text(format!("{}{}", a, b))),
            (Int(a), BinOp::Eq, Int(b)) => Ok(Bool(a == b)),
            (Int(a), BinOp::NotEq, Int(b)) => Ok(Bool(a != b)),
            (Int(a), BinOp::Lt, Int(b)) | (Int(a), BinOp::IsBelow, Int(b)) => Ok(Bool(a < b)),
            (Int(a), BinOp::Gt, Int(b))
            | (Int(a), BinOp::IsAbove, Int(b))
            | (Int(a), BinOp::Exceeds, Int(b)) => Ok(Bool(a > b)),
            (Int(a), BinOp::LtEq, Int(b)) => Ok(Bool(a <= b)),
            (Int(a), BinOp::GtEq, Int(b)) => Ok(Bool(a >= b)),
            (Float(a), BinOp::Eq, Float(b)) => Ok(Bool((a - b).abs() < 1e-10)),
            (Float(a), BinOp::Lt, Float(b)) | (Float(a), BinOp::IsBelow, Float(b)) => {
                Ok(Bool(a < b))
            }
            (Float(a), BinOp::Gt, Float(b))
            | (Float(a), BinOp::IsAbove, Float(b))
            | (Float(a), BinOp::Exceeds, Float(b)) => Ok(Bool(a > b)),
            (Bool(a), BinOp::Eq, Bool(b)) => Ok(Bool(a == b)),
            (Text(a), BinOp::Eq, Text(b)) => Ok(Bool(a == b)),
            (Text(a), BinOp::NotEq, Text(b)) => Ok(Bool(a != b)),
            (Bool(a), BinOp::And, Bool(b)) => Ok(Bool(*a && *b)),
            (Bool(a), BinOp::Or, Bool(b)) => Ok(Bool(*a || *b)),
            _ => Err(InterpError::new(format!(
                "operator {:?} not supported between `{}` and `{}`",
                op,
                lv.to_display(),
                rv.to_display()
            ))),
        }
    }
}
