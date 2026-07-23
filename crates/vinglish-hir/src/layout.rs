//! Deterministic C data-layout contract for the currently supported native ABI.
//!
//! This is deliberately a *target contract*, not a guess about the compiler
//! host. Production codegen must select the same contract as its C compiler.

use crate::symbol::{FieldId, SymbolTable, TypeId};
use crate::types::Type;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CAbi {
    pub pointer_size: u32,
    pub pointer_align: u32,
    pub long_size: u32,
    pub long_align: u32,
    pub double_size: u32,
    pub double_align: u32,
}

impl CAbi {
    /// LP64 SysV/macOS contract: the ABI used by the workspace's current C
    /// backend (`long`, `double`, and pointers are all naturally 8-aligned).
    pub const LP64: Self = Self { pointer_size: 8, pointer_align: 8, long_size: 8, long_align: 8, double_size: 8, double_align: 8 };
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldLayout { pub id: FieldId, pub offset: u32, pub size: u32, pub align: u32 }
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeLayout { pub type_id: TypeId, pub size: u32, pub align: u32, pub fields: Vec<FieldLayout> }

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayoutError { UnknownType(TypeId), RecursiveByValue(TypeId), UnresolvedType(String) }

pub struct LayoutResolver<'a> { symbols: &'a SymbolTable, abi: CAbi }

impl<'a> LayoutResolver<'a> {
    pub fn new(symbols: &'a SymbolTable, abi: CAbi) -> Self { Self { symbols, abi } }

    pub fn layout_type(&self, id: TypeId) -> Result<TypeLayout, LayoutError> {
        let symbol = self.symbols.get_type(id).ok_or(LayoutError::UnknownType(id))?;
        let mut fields = Vec::with_capacity(symbol.fields.len());
        let mut offset = 0;
        let mut record_align = 1;
        for field in &symbol.fields {
            let (size, align) = self.layout_value(&field.ty, id)?;
            offset = align_up(offset, align);
            fields.push(FieldLayout { id: field.id, offset, size, align });
            offset += size;
            record_align = record_align.max(align);
        }
        Ok(TypeLayout { type_id: id, size: align_up(offset, record_align), align: record_align, fields })
    }

    pub fn field_offset(&self, record: TypeId, field: FieldId) -> Result<u32, LayoutError> {
        self.layout_type(record)?.fields.into_iter().find(|layout| layout.id == field).map(|layout| layout.offset).ok_or(LayoutError::UnknownType(record))
    }

    fn layout_value(&self, ty: &Type, containing: TypeId) -> Result<(u32, u32), LayoutError> {
        match ty {
            Type::Int | Type::Bool | Type::Unit => Ok((self.abi.long_size, self.abi.long_align)),
            Type::Float => Ok((self.abi.double_size, self.abi.double_align)),
            Type::Text | Type::Reference(_, _) | Type::Pointer(_) | Type::List(_) | Type::Dict(_, _) | Type::Function(_, _) => Ok((self.abi.pointer_size, self.abi.pointer_align)),
            Type::Optional(_) | Type::Result(_, _) => Ok((self.abi.pointer_size, self.abi.pointer_align)),
            Type::Named(name, _) => {
                let symbol_id = self.symbols.lookup(name).ok_or_else(|| LayoutError::UnresolvedType(name.clone()))?;
                let id = TypeId(symbol_id);
                if id == containing { return Err(LayoutError::RecursiveByValue(id)); }
                let layout = self.layout_type(id)?;
                Ok((layout.size, layout.align))
            }
            Type::Var(_) => Err(LayoutError::UnresolvedType(ty.to_string())),
        }
    }
}

pub(crate) fn align_up(value: u32, align: u32) -> u32 { (value + align - 1) & !(align - 1) }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn alignment_rounds_up() { assert_eq!(align_up(9, 8), 16); }
}
