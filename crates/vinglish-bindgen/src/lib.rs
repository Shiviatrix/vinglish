use proc_macro::TokenStream;
use quote::{format_ident, quote};
use std::fs::OpenOptions;
use std::io::Write;
use syn::{parse_macro_input, ItemFn, Type, ReturnType, FnArg, PatType, Pat};

#[proc_macro_attribute]
pub fn vinglish_bindgen(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(item as ItemFn);
    let original_name = &input_fn.sig.ident;
    let original_name_str = original_name.to_string();
    
    // The FFI wrapper name will be `ving_<original_name>`
    let ffi_name = format_ident!("ving_{}", original_name);
    
    let mut ving_args = Vec::new();
    let mut extraction_code = Vec::new();
    let mut call_args = Vec::new();
    
    for (i, input) in input_fn.sig.inputs.iter().enumerate() {
        if let FnArg::Typed(PatType { pat, ty, .. }) = input {
            let arg_name = if let Pat::Ident(pat_ident) = &**pat {
                &pat_ident.ident
            } else {
                panic!("Unsupported pattern in argument");
            };
            
            if let Type::Reference(type_ref) = &**ty {
                if let Type::Path(type_path) = &*type_ref.elem {
                    if type_path.path.is_ident("str") {
                        extraction_code.push(quote! {
                            let #arg_name = if let Some(vinglish_codegen::interp::Value::Text(s)) = args.get(#i) {
                                s.as_str()
                            } else {
                                return Err(vinglish_codegen::interp::InterpError::new("Expected text argument"));
                            };
                        });
                        call_args.push(quote! { #arg_name });
                        ving_args.push(format!("{}: text", arg_name));
                        continue;
                    }
                }
            } else if let Type::Path(type_path) = &**ty {
                if type_path.path.is_ident("i64") {
                    extraction_code.push(quote! {
                        let #arg_name = if let Some(vinglish_codegen::interp::Value::Int(i)) = args.get(#i) {
                            *i
                        } else {
                            return Err(vinglish_codegen::interp::InterpError::new("Expected number argument"));
                        };
                    });
                    call_args.push(quote! { #arg_name });
                    ving_args.push(format!("{}: number", arg_name));
                    continue;
                } else if type_path.path.is_ident("f64") {
                    extraction_code.push(quote! {
                        let #arg_name = if let Some(vinglish_codegen::interp::Value::Float(f)) = args.get(#i) {
                            *f
                        } else if let Some(vinglish_codegen::interp::Value::Int(i)) = args.get(#i) {
                            *i as f64
                        } else {
                            return Err(vinglish_codegen::interp::InterpError::new("Expected decimal argument"));
                        };
                    });
                    call_args.push(quote! { #arg_name });
                    ving_args.push(format!("{}: decimal", arg_name));
                    continue;
                } else if type_path.path.is_ident("bool") {
                    extraction_code.push(quote! {
                        let #arg_name = if let Some(vinglish_codegen::interp::Value::Bool(b)) = args.get(#i) {
                            *b
                        } else {
                            return Err(vinglish_codegen::interp::InterpError::new("Expected boolean argument"));
                        };
                    });
                    call_args.push(quote! { #arg_name });
                    ving_args.push(format!("{}: bool", arg_name));
                    continue;
                }
            }
            panic!("Unsupported argument type");
        }
    }
    
    let (call_ret, ving_ret) = match &input_fn.sig.output {
        ReturnType::Default => (
            quote! { 
                #original_name(#(#call_args),*);
                Ok(vinglish_codegen::interp::Value::Unit)
            }, 
            "".to_string()
        ),
        ReturnType::Type(_, ty) => {
            if let Type::Path(type_path) = &**ty {
                if type_path.path.is_ident("String") {
                    (
                        quote! { 
                            Ok(vinglish_codegen::interp::Value::Text(#original_name(#(#call_args),*)))
                        },
                        "returns text".to_string(),
                    )
                } else if type_path.path.is_ident("i64") {
                    (
                        quote! { 
                            Ok(vinglish_codegen::interp::Value::Int(#original_name(#(#call_args),*)))
                        },
                        "returns number".to_string(),
                    )
                } else if type_path.path.is_ident("f64") {
                    (
                        quote! { 
                            Ok(vinglish_codegen::interp::Value::Float(#original_name(#(#call_args),*)))
                        },
                        "returns decimal".to_string(),
                    )
                } else if type_path.path.is_ident("bool") {
                    (
                        quote! { 
                            Ok(vinglish_codegen::interp::Value::Bool(#original_name(#(#call_args),*)))
                        },
                        "returns bool".to_string(),
                    )
                } else {
                    panic!("Unsupported return type");
                }
            } else {
                panic!("Unsupported return type");
            }
        }
    };

    let binding_decl = format!("public foreign function {}({}) {}\n", original_name_str, ving_args.join(", "), ving_ret);
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open("target/vinglish_bindings.ving") {
        let _ = file.write_all(binding_decl.as_bytes());
    }

    let expanded = quote! {
        #input_fn
        
        #[unsafe(no_mangle)]
        pub fn #ffi_name(args: Vec<vinglish_codegen::interp::Value>) -> Result<vinglish_codegen::interp::Value, vinglish_codegen::interp::InterpError> {
            #(#extraction_code)*
            #call_ret
        }
    };
    
    TokenStream::from(expanded)
}
