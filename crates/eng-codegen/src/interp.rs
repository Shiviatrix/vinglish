use std::collections::HashMap;
use std::fmt;

use eng_hir::symbol::{FieldId, FunctionId, SsaValueId, SymbolTable};
use eng_mir::{BlockId, Instruction, MirFunction, MirModule, Operand, Terminator};
use eng_parser::ast::{BinOp, Literal, UnOp};

// ─────────────────────────────────────────────────────────────────────────────
// Value
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Int(i64),
    Float(f64),
    Bool(bool),
    Text(String),
    Unit,
    List(Vec<Value>),
    Function(FunctionId),
    NativeFunction(NativeFn),
    Struct(u64),
    Return(Box<Value>),
    Reference(Box<Value>),
}

#[derive(Clone)]
pub struct NativeFn {
    pub name: &'static str,
    pub f: fn(Vec<Value>) -> Result<Value, InterpError>,
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
            Value::List(vs) => {
                let inner: Vec<_> = vs.iter().map(|v| v.to_display()).collect();
                format!("[{}]", inner.join(", "))
            }
            Value::Struct(_) => "<struct>".to_string(),
            Value::Function(_) => "<function>".to_string(),
            Value::NativeFunction(nf) => format!("<native:{}>", nf.name),
            Value::Return(v) => v.to_display(),
            Value::Reference(v) => format!("&{}", v.to_display()),
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
    fn new(msg: impl Into<String>) -> Self {
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

pub struct Interpreter<'a> {
    _symbol_table: &'a SymbolTable,
    functions: HashMap<FunctionId, &'a MirFunction<SsaValueId>>,
    native_functions: HashMap<FunctionId, NativeFn>,
}

use std::sync::Mutex;
use std::sync::OnceLock;

static NEXT_ADDR: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

fn get_heap() -> &'static Mutex<HashMap<u64, Vec<Value>>> {
    static HEAP: OnceLock<Mutex<HashMap<u64, Vec<Value>>>> = OnceLock::new();
    HEAP.get_or_init(|| Mutex::new(HashMap::new()))
}

fn get_struct_store() -> &'static Mutex<HashMap<u64, HashMap<FieldId, Value>>> {
    static STRUCT_STORE: OnceLock<Mutex<HashMap<u64, HashMap<FieldId, Value>>>> = OnceLock::new();
    STRUCT_STORE.get_or_init(|| Mutex::new(HashMap::new()))
}

impl<'a> Interpreter<'a> {
    pub fn new(symbol_table: &'a SymbolTable) -> Self {
        let interp = Self {
            _symbol_table: symbol_table,
            functions: HashMap::new(),
            native_functions: HashMap::new(),
        };
        interp
    }

    pub fn run_module(
        &mut self,
        module: &'a MirModule<eng_hir::symbol::SsaValueId>,
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
                    "eng_alloc" => Some(NativeFn {
                        name: "eng_alloc",
                        f: |args| {
                            if let Some(Value::Int(size)) = args.first() {
                                let addr =
                                    NEXT_ADDR.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                                let mut heap = get_heap().lock().unwrap();
                                heap.insert(addr, vec![Value::Int(0); *size as usize]);
                                Ok(Value::Int(addr as i64))
                            } else {
                                Err(InterpError::new("eng_alloc: expected size"))
                            }
                        },
                    }),
                    "eng_free" => Some(NativeFn {
                        name: "eng_free",
                        f: |args| {
                            if let Some(Value::Int(addr)) = args.first() {
                                let mut heap = get_heap().lock().unwrap();
                                heap.remove(&(*addr as u64));
                            }
                            Ok(Value::Unit)
                        },
                    }),
                    "eng_write" => Some(NativeFn {
                        name: "eng_write",
                        f: |args| {
                            if args.len() >= 3 {
                                let addr = match &args[0] {
                                    Value::Int(a) => *a as u64,
                                    Value::Reference(r) => match &**r {
                                        Value::Int(a) => *a as u64,
                                        _ => {
                                            return Err(InterpError::new(
                                                "eng_write: expected pointer",
                                            ))
                                        }
                                    },
                                    _ => {
                                        return Err(InterpError::new("eng_write: expected pointer"))
                                    }
                                };
                                let index = match &args[1] {
                                    Value::Int(i) => *i as usize,
                                    _ => return Err(InterpError::new("eng_write: expected index")),
                                };
                                let val = args[2].clone();
                                let mut heap = get_heap().lock().unwrap();
                                if let Some(vec) = heap.get_mut(&addr) {
                                    if index < vec.len() {
                                        vec[index] = val;
                                    } else {
                                        vec.resize(index + 1, Value::Int(0));
                                        vec[index] = val;
                                    }
                                }
                            }
                            Ok(Value::Unit)
                        },
                    }),
                    "eng_read" => Some(NativeFn {
                        name: "eng_read",
                        f: |args| {
                            if args.len() >= 2 {
                                let addr = match &args[0] {
                                    Value::Int(a) => *a as u64,
                                    Value::Reference(r) => match &**r {
                                        Value::Int(a) => *a as u64,
                                        _ => {
                                            return Err(InterpError::new(
                                                "eng_read: expected pointer",
                                            ))
                                        }
                                    },
                                    _ => {
                                        return Err(InterpError::new("eng_read: expected pointer"))
                                    }
                                };
                                let index = match &args[1] {
                                    Value::Int(i) => *i as usize,
                                    _ => return Err(InterpError::new("eng_read: expected index")),
                                };
                                let heap = get_heap().lock().unwrap();
                                if let Some(vec) = heap.get(&addr) {
                                    if index < vec.len() {
                                        return Ok(vec[index].clone());
                                    }
                                }
                            }
                            Ok(Value::Int(0))
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

            for instr in &block.instrs {
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
                while let Value::Reference(inner) = obj {
                    obj = *inner;
                }
                if let Value::Struct(struct_id) = obj {
                    let store = get_struct_store().lock().unwrap();
                    let fields = store
                        .get(&struct_id)
                        .ok_or_else(|| InterpError::new("Struct not found in store"))?;
                    let val = fields.get(field_id).cloned().unwrap_or(Value::Unit);
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
                while let Value::Reference(inner) = obj_val {
                    obj_val = *inner;
                }
                if let Value::Struct(struct_id) = obj_val {
                    let mut store = get_struct_store().lock().unwrap();
                    if let Some(fields) = store.get_mut(&struct_id) {
                        fields.insert(*field_id, val);
                    } else {
                        return Err(InterpError::new("Struct not found in store"));
                    }
                } else {
                    return Err(InterpError::new("Cannot store field to non-struct"));
                }
            }
            Instruction::<SsaValueId>::Call(dest, func_id, arg_ops) => {
                let mut args = Vec::new();
                for arg_op in arg_ops {
                    args.push(self.eval_operand(arg_op, locals)?);
                }
                let ret = self.call_function(*func_id, args)?;
                locals.insert(*dest, ret);
            }
            Instruction::<SsaValueId>::HeapAllocate(dest, _ty)
            | Instruction::<SsaValueId>::StackAllocate(dest, _ty) => {
                let addr = NEXT_ADDR.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                let mut store = get_struct_store().lock().unwrap();
                store.insert(addr, HashMap::new());
                locals.insert(*dest, Value::Struct(addr));
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
                        panic!("borrow/deref not supported in interpreter")
                    }
                };
                locals.insert(*dest, res);
            }
            Instruction::<SsaValueId>::Borrow(dest, op)
            | Instruction::<SsaValueId>::BorrowMut(dest, op) => {
                let val = self.eval_operand(op, locals)?;
                locals.insert(*dest, Value::Reference(Box::new(val)));
            }
            Instruction::<SsaValueId>::Deref(dest, op, _) => {
                let val = self.eval_operand(op, locals)?;
                if let Value::Reference(inner) = val {
                    locals.insert(*dest, *inner);
                } else {
                    return Err(InterpError::new(format!(
                        "Cannot deref non-reference value: {:?}",
                        val
                    )));
                }
            }
            Instruction::<SsaValueId>::Drop(var) => {
                // Interpreter doesn't enforce drop right now
                let _ = var;
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
