use crate::passes::CompilerContext;
use vinglish_hir::symbol::{FunctionId, FunctionSymbol, SymbolId, TypeId, TypeSymbol};
use vinglish_hir::types::Type;
use vinglish_parser::ast::Visibility;

pub fn import_c_header(path: &str, ctx: &mut CompilerContext) {
    let bindings_res = bindgen::Builder::default()
        .header_contents("wrapper.h", &format!("#include \"{}\"", path))
        .generate();

    let bindings = match bindings_res {
        Ok(b) => b,
        Err(_) => {
            ctx.type_errors.push(crate::type_pass::TypeError::new(
                format!("Failed to parse C header '{}'", path),
                vinglish_lexer::Span::dummy(),
            ));
            return;
        }
    };

    let rust_code = bindings.to_string();
    let file = match syn::parse_file(&rust_code) {
        Ok(f) => f,
        Err(_) => return,
    };

    let mut type_aliases = std::collections::HashMap::new();

    for item in &file.items {
        if let syn::Item::Type(t) = item {
            let name = t.ident.to_string();
            let ty = syn_type_to_vinglish_type(&t.ty, &type_aliases);
            type_aliases.insert(name, ty);
        }
    }

    for item in file.items {
        match item {
            syn::Item::ForeignMod(foreign_mod) => {
                for foreign_item in foreign_mod.items {
                    if let syn::ForeignItem::Fn(f) = foreign_item {
                        let name = f.sig.ident.to_string();
                        let mut params = Vec::new();
                        for input in f.sig.inputs {
                            if let syn::FnArg::Typed(pat_type) = input {
                                params.push(syn_type_to_vinglish_type(&pat_type.ty, &type_aliases));
                            }
                        }
                        let ret_ty = match f.sig.output {
                            syn::ReturnType::Default => Type::Unit,
                            syn::ReturnType::Type(_, ty) => {
                                syn_type_to_vinglish_type(&ty, &type_aliases)
                            }
                        };

                        let fn_ty = Type::Function(params, Box::new(ret_ty));

                        ctx.symbol_table.define_func(
                            name.clone(),
                            FunctionSymbol {
                                id: FunctionId(SymbolId(0)),
                                name: name.clone(),
                                visibility: Visibility::Public,
                                ty: fn_ty,
                                generic_params: vec![],
                                is_variant_constructor: None,
                                is_foreign: true,
                            },
                        );
                    }
                }
            }
            syn::Item::Struct(s) => {
                let name = s.ident.to_string();
                let type_id = ctx.symbol_table.define_type(
                    name.clone(),
                    TypeSymbol::new(TypeId(SymbolId(0)), name.clone(), Visibility::Public),
                );
                if let Some(ts) = ctx.symbol_table.get_type_mut(type_id) {
                    ts.id = type_id;
                    if let syn::Fields::Named(fields) = s.fields {
                        for field in fields.named {
                            if let Some(ident) = field.ident {
                                let field_name = ident.to_string();
                                let field_type =
                                    syn_type_to_vinglish_type(&field.ty, &type_aliases);
                                ts.add_field(field_name, field_type, Visibility::Public);
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

fn syn_type_to_vinglish_type(
    ty: &syn::Type,
    aliases: &std::collections::HashMap<String, Type>,
) -> Type {
    match ty {
        syn::Type::Path(p) => {
            if let Some(segment) = p.path.segments.last() {
                let ident = segment.ident.to_string();
                match ident.as_str() {
                    "i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64" | "usize"
                    | "isize" | "c_int" | "c_uint" | "c_long" | "c_ulong" | "c_short"
                    | "c_ushort" | "c_char" | "c_uchar" => Type::Int,
                    "f32" | "f64" | "c_float" | "c_double" => Type::Float,
                    "bool" => Type::Bool,
                    _ => {
                        if let Some(aliased) = aliases.get(&ident) {
                            aliased.clone()
                        } else {
                            Type::Named(ident, vec![])
                        }
                    }
                }
            } else {
                Type::Unit
            }
        }
        syn::Type::Ptr(ptr) => {
            let inner = syn_type_to_vinglish_type(&ptr.elem, aliases);
            Type::Pointer(Box::new(inner))
        }
        syn::Type::Reference(r) => {
            let inner = syn_type_to_vinglish_type(&r.elem, aliases);
            let mutable = r.mutability.is_some();
            Type::Reference(Box::new(inner), mutable)
        }
        _ => Type::Unit,
    }
}
