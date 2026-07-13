use vinglish_hir::symbol::TypeSymbol;
use vinglish_hir::types::{Type, TypeVar};
use std::collections::HashMap;

/// A type scheme: ∀ vars. ty (for let-polymorphism).
#[derive(Debug, Clone)]
pub struct TypeScheme {
    pub vars: Vec<TypeVar>,
    pub ty: Type,
}

impl TypeScheme {
    pub fn mono(ty: Type) -> Self {
        Self { vars: vec![], ty }
    }

    /// Instantiate the scheme — replace bound vars with fresh ones.
    pub fn instantiate(&self) -> Type {
        if self.vars.is_empty() {
            return self.ty.clone();
        }
        let mapping: HashMap<u32, TypeVar> =
            self.vars.iter().map(|v| (v.0, TypeVar::fresh())).collect();
        subst_type(&self.ty, &mapping)
    }
}

fn subst_type(ty: &Type, mapping: &HashMap<u32, TypeVar>) -> Type {
    match ty {
        Type::Var(v) => {
            if let Some(new_v) = mapping.get(&v.0) {
                Type::Var(*new_v)
            } else {
                ty.clone()
            }
        }
        Type::List(t) => Type::List(Box::new(subst_type(t, mapping))),
        Type::Dict(k, v) => Type::Dict(
            Box::new(subst_type(k, mapping)),
            Box::new(subst_type(v, mapping)),
        ),
        Type::Optional(t) => Type::Optional(Box::new(subst_type(t, mapping))),
        Type::Result(ok, err) => Type::Result(
            Box::new(subst_type(ok, mapping)),
            Box::new(subst_type(err, mapping)),
        ),
        Type::Function(args, ret) => {
            let args = args.iter().map(|a| subst_type(a, mapping)).collect();
            Type::Function(args, Box::new(subst_type(ret, mapping)))
        }
        Type::Named(n, args) => {
            let args = args.iter().map(|a| subst_type(a, mapping)).collect();
            Type::Named(n.clone(), args)
        }
        other => other.clone(),
    }
}

/// A lexical scope chain mapping names to type schemes.
#[derive(Debug, Default, Clone)]
pub struct TypeEnv {
    pub scopes: Vec<HashMap<String, TypeScheme>>,
    pub structs: HashMap<String, TypeSymbol>,
}

impl TypeEnv {
    pub fn new() -> Self {
        let mut env = Self {
            scopes: vec![HashMap::new()],
            structs: HashMap::new(),
        };
        env.seed_builtins();
        env
    }

    /// Seed the environment with built-in functions.
    fn seed_builtins(&mut self) {
        use Type::*;
        let builtins: &[(&str, Type)] = &[
            ("print_number", Function(vec![Int], Box::new(Unit))),
            ("abs", Function(vec![Int], Box::new(Int))),
            ("sqrt", Function(vec![Float], Box::new(Float))),
            ("min", Function(vec![Int, Int], Box::new(Int))),
            ("max", Function(vec![Int, Int], Box::new(Int))),
            ("to_text", Function(vec![Int], Box::new(Text))),
            ("to_number", Function(vec![Text], Box::new(Int))),
        ];
        for (name, ty) in builtins {
            self.define(name, TypeScheme::mono(ty.clone()));
        }

        // Polymorphic builtins
        let tv1 = TypeVar::fresh();
        self.define(
            "print",
            TypeScheme {
                vars: vec![tv1],
                ty: Function(vec![Var(tv1)], Box::new(Unit)),
            },
        );

        let tv2 = TypeVar::fresh();
        self.define(
            "println",
            TypeScheme {
                vars: vec![tv2],
                ty: Function(vec![Var(tv2)], Box::new(Unit)),
            },
        );

        let tv_len = TypeVar::fresh();
        self.define(
            "len",
            TypeScheme {
                vars: vec![tv_len],
                ty: Function(vec![Named("List".into(), vec![Var(tv_len)])], Box::new(Int)),
            },
        );
    }

    pub fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    pub fn define(&mut self, name: &str, scheme: TypeScheme) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_string(), scheme);
        }
    }

    pub fn get(&self, name: &str) -> Option<TypeScheme> {
        for scope in self.scopes.iter().rev() {
            if let Some(scheme) = scope.get(name) {
                return Some(scheme.clone());
            }
        }
        None
    }

    pub fn define_struct(&mut self, name: &str, symbol: TypeSymbol) {
        self.structs.insert(name.to_string(), symbol);
    }

    pub fn lookup(&self, name: &str) -> Option<&TypeScheme> {
        for scope in self.scopes.iter().rev() {
            if let Some(scheme) = scope.get(name) {
                return Some(scheme);
            }
        }
        None
    }

    /// Return all names defined in any scope (for diagnostic suggestions).
    pub fn all_names(&self) -> Vec<String> {
        let mut names = Vec::new();
        for scope in &self.scopes {
            names.extend(scope.keys().cloned());
        }
        names.sort();
        names.dedup();
        names
    }
}
