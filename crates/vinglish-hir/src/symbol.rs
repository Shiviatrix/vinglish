use crate::types::{Type, TypeVar};
use vinglish_parser::ast::Visibility;
use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SymbolId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TypeId(pub SymbolId);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FunctionId(pub SymbolId);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VariableId(pub SymbolId);

impl fmt::Display for VariableId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "var_{}", self.0 .0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SsaValueId(pub u32);

impl fmt::Display for SsaValueId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ssa_{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FieldId(pub usize);

#[derive(Debug, Clone)]
pub struct SymbolTable {
    symbols: Vec<SymbolKind>,
    names: HashMap<String, SymbolId>,
    interned_types: HashMap<Type, TypeId>,
}

#[derive(Debug, Clone)]
pub enum SymbolKind {
    Type(TypeSymbol),
    Function(FunctionSymbol),
    Variable(VariableSymbol),
    InternedType(Type),
}

impl Default for SymbolTable {
    fn default() -> Self {
        Self::new()
    }
}

impl SymbolTable {
    pub fn new() -> Self {
        Self {
            symbols: Vec::new(),
            names: HashMap::new(),
            interned_types: HashMap::new(),
        }
    }

    pub fn num_symbols(&self) -> usize {
        self.symbols.len()
    }

    pub fn intern_type(&mut self, ty: Type) -> TypeId {
        if let Some(&id) = self.interned_types.get(&ty) {
            return id;
        }
        let id = SymbolId(self.symbols.len() as u32);
        self.symbols.push(SymbolKind::InternedType(ty.clone()));
        let type_id = TypeId(id);
        self.interned_types.insert(ty, type_id);
        type_id
    }

    pub fn get_interned_type(&self, id: TypeId) -> Option<&Type> {
        match self.symbols.get(id.0 .0 as usize) {
            Some(SymbolKind::InternedType(ty)) => Some(ty),
            Some(SymbolKind::Type(_ts)) => {
                // Should Named types return themselves?
                // The Type enum handles everything, so if we interned a Named type it's in InternedType.
                None
            }
            _ => None,
        }
    }

    pub fn define_type(&mut self, name: String, symbol: TypeSymbol) -> TypeId {
        let id = SymbolId(self.symbols.len() as u32);
        self.symbols.push(SymbolKind::Type(symbol));
        self.names.insert(name, id);
        TypeId(id)
    }

    pub fn define_func(&mut self, name: String, symbol: FunctionSymbol) -> FunctionId {
        let id = SymbolId(self.symbols.len() as u32);
        self.symbols.push(SymbolKind::Function(symbol));
        self.names.insert(name, id);
        FunctionId(id)
    }

    pub fn define_var(&mut self, name: String, symbol: VariableSymbol) -> VariableId {
        let id = SymbolId(self.symbols.len() as u32);
        self.symbols.push(SymbolKind::Variable(symbol));
        self.names.insert(name, id);
        VariableId(id)
    }

    pub fn define_anon_func(&mut self, symbol: FunctionSymbol) -> FunctionId {
        let id = SymbolId(self.symbols.len() as u32);
        self.symbols.push(SymbolKind::Function(symbol));
        FunctionId(id)
    }

    pub fn define_anon_var(&mut self, symbol: VariableSymbol) -> VariableId {
        let id = SymbolId(self.symbols.len() as u32);
        self.symbols.push(SymbolKind::Variable(symbol));
        VariableId(id)
    }

    pub fn define_var_with_id(&mut self, id: SymbolId, symbol: VariableSymbol) {
        let idx = id.0 as usize;
        if idx >= self.symbols.len() {
            self.symbols
                .resize(idx + 1, SymbolKind::Variable(symbol.clone()));
        }
        self.symbols[idx] = SymbolKind::Variable(symbol);
    }

    pub fn get(&self, id: SymbolId) -> Option<&SymbolKind> {
        self.symbols.get(id.0 as usize)
    }

    pub fn get_type(&self, id: TypeId) -> Option<&TypeSymbol> {
        if let Some(SymbolKind::Type(ts)) = self.symbols.get(id.0 .0 as usize) {
            Some(ts)
        } else {
            None
        }
    }

    pub fn get_func(&self, id: FunctionId) -> Option<&FunctionSymbol> {
        if let Some(SymbolKind::Function(fs)) = self.symbols.get(id.0 .0 as usize) {
            Some(fs)
        } else {
            None
        }
    }

    pub fn get_var(&self, id: VariableId) -> Option<&VariableSymbol> {
        if let Some(SymbolKind::Variable(vs)) = self.symbols.get(id.0 .0 as usize) {
            Some(vs)
        } else {
            None
        }
    }

    pub fn get_type_mut(&mut self, id: TypeId) -> Option<&mut TypeSymbol> {
        if let Some(SymbolKind::Type(ts)) = self.symbols.get_mut(id.0 .0 as usize) {
            Some(ts)
        } else {
            None
        }
    }

    pub fn get_func_mut(&mut self, id: FunctionId) -> Option<&mut FunctionSymbol> {
        if let Some(SymbolKind::Function(fs)) = self.symbols.get_mut(id.0 .0 as usize) {
            Some(fs)
        } else {
            None
        }
    }

    pub fn get_var_mut(&mut self, id: VariableId) -> Option<&mut VariableSymbol> {
        if let Some(SymbolKind::Variable(vs)) = self.symbols.get_mut(id.0 .0 as usize) {
            Some(vs)
        } else {
            None
        }
    }

    pub fn lookup(&self, name: &str) -> Option<SymbolId> {
        self.names.get(name).copied()
    }

    pub fn names(&self) -> &HashMap<String, SymbolId> {
        &self.names
    }
}

#[derive(Debug, Clone)]
pub struct TypeSymbol {
    pub id: TypeId,
    pub name: String,
    pub visibility: Visibility,
    pub fields: Vec<FieldSymbol>,
    pub methods: HashMap<String, FunctionId>,
    pub generic_params: Vec<TypeVar>,
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct FunctionSymbol {
    pub id: FunctionId,
    pub name: String,
    pub visibility: Visibility,
    pub ty: Type,
    pub generic_params: Vec<TypeVar>,
    pub is_variant_constructor: Option<(TypeId, usize)>,
}

#[derive(Debug, Clone)]
pub struct VariableSymbol {
    pub id: VariableId,
    pub name: String,
    pub is_mut: bool,
    pub ty: Type,
}

#[derive(Debug, Clone)]
pub struct FieldSymbol {
    pub id: FieldId,
    pub name: String,
    pub ty: Type,
    pub visibility: Visibility,
}

impl TypeSymbol {
    pub fn new(id: TypeId, name: String, visibility: Visibility) -> Self {
        Self {
            id,
            name,
            visibility,
            fields: Vec::new(),
            methods: HashMap::new(),
            generic_params: Vec::new(),
            capabilities: Vec::new(),
        }
    }

    pub fn add_field(&mut self, name: String, ty: Type, visibility: Visibility) -> FieldId {
        let index = self.fields.len();
        let id = FieldId(index);
        self.fields.push(FieldSymbol {
            id,
            name,
            ty,
            visibility,
        });
        id
    }

    pub fn get_field(&self, name: &str) -> Option<&FieldSymbol> {
        self.fields.iter().find(|f| f.name == name)
    }

    pub fn add_method(&mut self, name: String, func_id: FunctionId) {
        self.methods.insert(name, func_id);
    }
}
