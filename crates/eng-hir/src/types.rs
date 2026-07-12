use std::sync::atomic::{AtomicU32, Ordering};

/// A fresh type variable ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TypeVar(pub u32);

static NEXT_VAR: AtomicU32 = AtomicU32::new(0);

impl TypeVar {
    pub fn fresh() -> Self {
        Self(NEXT_VAR.fetch_add(1, Ordering::Relaxed))
    }
}

impl std::fmt::Display for TypeVar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Display as `'a`, `'b`, ... for familiar feel
        let ch = (b'a' + (self.0 % 26) as u8) as char;
        if self.0 < 26 {
            write!(f, "'{}", ch)
        } else {
            write!(f, "'{}{}", ch, self.0 / 26)
        }
    }
}

/// The Englist type algebra.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    // ── Primitive types ───────────────────────────────────────────────────────
    Int,
    Float,
    Bool,
    Text,
    Unit,
    Reference(Box<Type>, bool), // bool is true if mutable
    Pointer(Box<Type>), // Raw pointer address<T>

    // ── Composite types ───────────────────────────────────────────────────────
    List(Box<Type>),
    Dict(Box<Type>, Box<Type>),
    Optional(Box<Type>),
    Result(Box<Type>, Box<Type>),  // Ok(T), Err(E)

    // ── Function type ─────────────────────────────────────────────────────────
    Function(Vec<Type>, Box<Type>),

    // ── Named / user-defined types ────────────────────────────────────────────
    Named(String, Vec<Type>),

    // ── Inference variable (unification) ─────────────────────────────────────
    Var(TypeVar),
}

impl Type {

    /// Returns true if this type has copy semantics (no ownership transfer on assignment/call).
    pub fn is_copy(&self) -> bool {
        match self {
            Type::Int | Type::Float | Type::Bool | Type::Unit | Type::Pointer(_) => true,
            _ => false, // Lists, dicts, strings, structs are moved
        }
    }

    /// Returns true if this type contains no type variables (is monomorphic).
    pub fn is_concrete(&self) -> bool {
        match self {
            Type::Var(_) => false,
            Type::List(t) => t.is_concrete(),
            Type::Dict(k, v) => k.is_concrete() && v.is_concrete(),
            Type::Optional(t) | Type::Result(t, _) => t.is_concrete(),
            Type::Function(args, ret) => args.iter().all(Type::is_concrete) && ret.is_concrete(),
            Type::Named(_, args) => args.iter().all(Type::is_concrete),
            _ => true,
        }
    }

    /// Collect all type variables referenced in this type.
    pub fn free_vars(&self) -> Vec<TypeVar> {
        let mut vars = Vec::new();
        self.collect_vars(&mut vars);
        vars.dedup_by_key(|v| v.0);
        vars
    }

    fn collect_vars(&self, acc: &mut Vec<TypeVar>) {
        match self {
            Type::Var(v) => acc.push(*v),
            Type::List(t) => t.collect_vars(acc),
            Type::Dict(k, v) => { k.collect_vars(acc); v.collect_vars(acc); }
            Type::Optional(t) => t.collect_vars(acc),
            Type::Result(ok, err) => { ok.collect_vars(acc); err.collect_vars(acc); }
            Type::Function(args, ret) => {
                for a in args { a.collect_vars(acc); }
                ret.collect_vars(acc);
            }
            Type::Named(_, args) => {
                for a in args { a.collect_vars(acc); }
            }
            _ => {}
        }
    }
}

impl std::fmt::Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Int     => write!(f, "number"),
            Type::Float   => write!(f, "decimal"),
            Type::Bool    => write!(f, "boolean"),
            Type::Text    => write!(f, "text"),
            Type::Unit    => write!(f, "unit"),
            Type::Reference(inner, mutable) => {
                if *mutable {
                    write!(f, "borrow mutable {inner}")
                } else {
                    write!(f, "borrow {inner}")
                }
            }
            Type::Pointer(inner) => write!(f, "address<{inner}>"),
            Type::List(t) => write!(f, "List of {t}"),
            Type::Dict(k, v) => write!(f, "Dictionary from {k} to {v}"),
            Type::Optional(t)     => write!(f, "{t}?"),
            Type::Result(ok, err) => write!(f, "Result<{ok}, {err}>"),
            Type::Function(args, ret) => {
                write!(f, "(")?;
                for (i, a) in args.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{a}")?;
                }
                write!(f, ") -> {ret}")
            }
            Type::Named(n, args) => {
                if args.is_empty() {
                    write!(f, "{n}")
                } else {
                    write!(f, "{n}<")?;
                    for (i, a) in args.iter().enumerate() {
                        if i > 0 { write!(f, ", ")?; }
                        write!(f, "{a}")?;
                    }
                    write!(f, ">")
                }
            }
            Type::Var(v) => write!(f, "{v}"),
        }
    }
}
