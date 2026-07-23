extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{Lit, parse_macro_input};
use std::str::FromStr;

/// AOT transpile ScriptGo (SGL) into zero-cost Rust loops at compile time!
#[proc_macro]
pub fn sgl_compile(input: TokenStream) -> TokenStream {
    let input_lit = parse_macro_input!(input as Lit);
    if let Lit::Str(str_lit) = input_lit {
        let mut sgl_code = str_lit.value();
        
        // Transpile SGL to Rust
        // Basic transpilation rules for zero-cost iterators:
        sgl_code = sgl_code.replace("let ", "let mut ");
        sgl_code = sgl_code.replace(": Int", ": u32");
        sgl_code = sgl_code.replace(": Float", ": f64");
        
        let block_code = format!("{{ #[allow(unused_assignments)] #[allow(clippy::assign_op_pattern)] {} }}", sgl_code);
        
        match TokenStream::from_str(&block_code) {
            Ok(ts) => ts,
            Err(e) => {
                let err = format!("Failed to parse transpiled SGL: {:?}", e);
                let err_ts = quote! { compile_error!(#err); };
                TokenStream::from(err_ts)
            }
        }
    } else {
        TokenStream::from(quote! { compile_error!("Expected a string literal containing SGL code"); })
    }
}
