extern crate proc_macro;

use proc_macro::TokenStream;
use quote::{quote, format_ident};
use syn::{parse_macro_input, ItemFn, FnArg, Pat, Type, ReturnType};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;

#[proc_macro_attribute]
pub fn vinglish_export(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let original_name = &input.sig.ident;
    let export_name = format_ident!("eng_{}", original_name);
    
    let mut c_args = Vec::new();
    let mut rust_args = Vec::new();
    let mut v_args = Vec::new();
    
    for arg in &input.sig.inputs {
        if let FnArg::Typed(pat_type) = arg {
            if let Pat::Ident(pat_ident) = &*pat_type.pat {
                let arg_name = &pat_ident.ident;
                let ty = &*pat_type.ty;
                
                if let Type::Path(type_path) = ty {
                    if let Some(segment) = type_path.path.segments.last() {
                        let type_name = segment.ident.to_string();
                        match type_name.as_str() {
                            "String" => {
                                c_args.push(quote! { #arg_name: *const std::os::raw::c_char });
                                rust_args.push(quote! { 
                                    unsafe { std::ffi::CStr::from_ptr(#arg_name) }.to_string_lossy().into_owned() 
                                });
                                v_args.push(format!("{}: string", arg_name));
                            },
                            "i32" => {
                                c_args.push(quote! { #arg_name: i32 });
                                rust_args.push(quote! { #arg_name });
                                v_args.push(format!("{}: number", arg_name));
                            },
                            "f64" => {
                                c_args.push(quote! { #arg_name: f64 });
                                rust_args.push(quote! { #arg_name });
                                v_args.push(format!("{}: number", arg_name));
                            },
                            _ => panic!("Unsupported type in #[vinglish_export]: {}", type_name),
                        }
                    }
                }
            }
        }
    }
    
    let (c_ret_type, rust_ret_handler, v_ret) = match &input.sig.output {
        ReturnType::Default => (quote! {}, quote! { #original_name(#(#rust_args),*); }, String::new()),
        ReturnType::Type(_, ty) => {
            if let Type::Path(type_path) = &**ty {
                if let Some(segment) = type_path.path.segments.last() {
                    let type_name = segment.ident.to_string();
                    match type_name.as_str() {
                        "i32" => (quote! { -> i32 }, quote! { #original_name(#(#rust_args),*) }, " returns number".to_string()),
                        "f64" => (quote! { -> f64 }, quote! { #original_name(#(#rust_args),*) }, " returns number".to_string()),
                        _ => panic!("Unsupported return type in #[vinglish_export]: {}", type_name),
                    }
                } else {
                    (quote! {}, quote! { #original_name(#(#rust_args),*); }, String::new())
                }
            } else {
                (quote! {}, quote! { #original_name(#(#rust_args),*); }, String::new())
            }
        }
    };
    
    // Generate Vinglish foreign function declaration
    let v_decl = format!("public foreign function {}({}){}\n", export_name, v_args.join(", "), v_ret);
    
    // Append to temporary file for the Vinglish compiler to pick up
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    // The macro runs in `rt_rust`, so the workspace root is its parent
    let interfaces_file = PathBuf::from(manifest_dir).parent().unwrap().join(".vinglish_interfaces.tmp");
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&interfaces_file) {
        let _ = file.write_all(v_decl.as_bytes());
    }

    let expanded = quote! {
        #input
        
        #[no_mangle]
        pub extern "C" fn #export_name(#(#c_args),*) #c_ret_type {
            #rust_ret_handler
        }
    };
    
    TokenStream::from(expanded)
}
