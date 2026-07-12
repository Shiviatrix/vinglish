use inkwell::context::Context;
use inkwell::types::{BasicTypeEnum, BasicMetadataTypeEnum, StructType};
use eng_hir::symbol::{SymbolTable, TypeId, SymbolKind};
use eng_hir::types::Type;

pub struct TypeLowering<'ctx> {
    context: &'ctx Context,
}

impl<'ctx> TypeLowering<'ctx> {
    pub fn new(context: &'ctx Context) -> Self {
        Self { context }
    }

    pub fn lower_type(&self, ty: &Type, symbol_table: &SymbolTable) -> Option<BasicTypeEnum<'ctx>> {
        match ty {
            Type::Int => Some(self.context.i64_type().into()),
            Type::Float => Some(self.context.f64_type().into()),
            Type::Bool => Some(self.context.bool_type().into()),
            Type::Text => {
                Some(self.context.ptr_type(inkwell::AddressSpace::default()).into())
            }
            Type::Unit => None, // void
            Type::Named(name, _) => {
                if let Some(type_id) = symbol_table.lookup(name) {
                    if let Some(SymbolKind::Type(_)) = symbol_table.get(type_id) {
                        return Some(self.lower_struct_type(symbol_table, TypeId(type_id)).into());
                    }
                }
                let struct_type = self.context.opaque_struct_type(name);
                Some(struct_type.into())
            }
            Type::List(_) | Type::Dict(_, _) | Type::Optional(_) | Type::Result(_, _) => {
                Some(self.context.ptr_type(inkwell::AddressSpace::default()).into())
            }
            Type::Function(_, _) => {
                Some(self.context.ptr_type(inkwell::AddressSpace::default()).into())
            }
            Type::Reference(_, _) | Type::Pointer(_) => {
                Some(self.context.ptr_type(inkwell::AddressSpace::default()).into())
            }
            Type::Var(_) => None,
        }
    }

    pub fn lower_struct_type(&self, symbol_table: &SymbolTable, type_id: TypeId) -> StructType<'ctx> {
        if let Some(ts) = symbol_table.get_type(type_id) {
            let name = &ts.name;
            if let Some(existing) = self.context.get_struct_type(name) {
                if !existing.is_opaque() {
                    return existing;
                }
            }
            let struct_type = self.context.opaque_struct_type(name);

            let field_types: Vec<BasicTypeEnum<'ctx>> = ts.fields.iter().map(|f| {
                self.lower_type(&f.ty, symbol_table).unwrap_or(self.context.i64_type().into())
            }).collect();

            struct_type.set_body(&field_types, false);
            struct_type
        } else {
            self.context.opaque_struct_type("unknown")
        }
    }

    /// Convert a type to a `BasicMetadataTypeEnum` for function parameter types.
    pub fn lower_to_metadata_type(&self, ty: &Type, symbol_table: &SymbolTable) -> BasicMetadataTypeEnum<'ctx> {
        match self.lower_type(ty, symbol_table) {
            Some(t) => t.into(),
            None => self.context.i64_type().into(), // fallback for Unit params
        }
    }


    pub fn i64_type(&self) -> BasicTypeEnum<'ctx> {
        self.context.i64_type().into()
    }

    pub fn f64_type(&self) -> BasicTypeEnum<'ctx> {
        self.context.f64_type().into()
    }

    pub fn bool_type(&self) -> BasicTypeEnum<'ctx> {
        self.context.bool_type().into()
    }

    pub fn ptr_type(&self) -> BasicTypeEnum<'ctx> {
        self.context.ptr_type(inkwell::AddressSpace::default()).into()
    }
}
