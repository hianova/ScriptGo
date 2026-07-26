extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input, punctuated::Punctuated, FnArg, ItemFn, ItemMod, Lit,
    LitInt, Meta, Pat, PatType, ReturnType, Token, Type,
};
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
        
        let block_code = format!("{{ {} }}", sgl_code);
        
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

struct PackageArgs {
    name: Option<String>,
    kind: Option<String>,
}

impl Parse for PackageArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut name = None;
        let mut kind = None;

        if input.is_empty() {
            return Ok(PackageArgs { name, kind });
        }

        let metas = Punctuated::<Meta, Token![,]>::parse_terminated(input)?;
        for meta in metas {
            match meta {
                Meta::NameValue(nv) => {
                    let key = nv.path.get_ident().map(|i| i.to_string());
                    if key.as_deref() == Some("name") {
                        if let syn::Expr::Lit(syn::ExprLit { lit: Lit::Str(s), .. }) = &nv.value {
                            name = Some(s.value());
                        }
                    } else if key.as_deref() == Some("kind")
                        && let syn::Expr::Lit(syn::ExprLit { lit: Lit::Str(s), .. }) = &nv.value
                    {
                        kind = Some(s.value());
                    }
                }
                Meta::Path(p) => {
                    if let Some(ident) = p.get_ident() {
                        let id_str = ident.to_string();
                        if id_str == "hardware" || id_str == "syscall" {
                            kind = Some(id_str);
                        } else {
                            name = Some(id_str);
                        }
                    }
                }
                _ => {}
            }
        }

        Ok(PackageArgs { name, kind })
    }
}

struct CmdArgs {
    id: Option<u32>,
}

impl Parse for CmdArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.is_empty() {
            return Ok(CmdArgs { id: None });
        }

        if let Ok(lit) = input.parse::<LitInt>() {
            let id = lit.base10_parse::<u32>()?;
            return Ok(CmdArgs { id: Some(id) });
        }

        let metas = Punctuated::<Meta, Token![,]>::parse_terminated(input)?;
        for meta in metas {
            if let Meta::NameValue(nv) = meta {
                let key = nv.path.get_ident().map(|i| i.to_string());
                if (key.as_deref() == Some("id") || key.as_deref() == Some("cmd"))
                    && let syn::Expr::Lit(syn::ExprLit { lit: Lit::Int(l), .. }) = &nv.value
                {
                    let id = l.base10_parse::<u32>()?;
                    return Ok(CmdArgs { id: Some(id) });
                }
            }
        }

        Ok(CmdArgs { id: None })
    }
}

fn to_pascal_case(s: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = true;
    for c in s.chars() {
        if c == '_' || c == '-' {
            capitalize_next = true;
        } else if capitalize_next {
            result.push(c.to_ascii_uppercase());
            capitalize_next = false;
        } else {
            result.push(c);
        }
    }
    result
}

fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c == '-' {
            result.push('_');
        } else if c.is_uppercase() {
            if i > 0 {
                result.push('_');
            }
            result.push(c.to_ascii_lowercase());
        } else {
            result.push(c);
        }
    }
    result
}

fn is_vm_type(ty: &Type) -> bool {
    let s = quote!(#ty).to_string();
    s.contains("ScriptVm")
}

fn is_string_type(ty: &Type) -> bool {
    let s = quote!(#ty).to_string();
    s.contains("String")
}

fn is_str_slice_type(ty: &Type) -> bool {
    let s = quote!(#ty).to_string();
    s.contains("& str") || s.contains("&str")
}

fn is_vec_u8_type(ty: &Type) -> bool {
    let s = quote!(#ty).to_string();
    s.contains("Vec < u8 >") || s.contains("Vec<u8>")
}

fn is_byte_slice_type(ty: &Type) -> bool {
    let s = quote!(#ty).to_string();
    s.contains("& [u8]") || s.contains("&[u8]")
}

fn is_bool_type(ty: &Type) -> bool {
    let s = quote!(#ty).to_string();
    s == "bool"
}

struct ProcessedCmd {
    match_arm: TokenStream2,
}

fn build_cmd_match_arm(id: u32, func: &ItemFn) -> ProcessedCmd {
    let fn_name = func.sig.ident.clone();

    let mut vm_param_idx = None;
    let mut data_params = Vec::new();

    for (idx, arg) in func.sig.inputs.iter().enumerate() {
        if let FnArg::Typed(PatType { ty, pat, .. }) = arg {
            if is_vm_type(ty) {
                vm_param_idx = Some(idx);
            } else {
                let name = match &**pat {
                    Pat::Ident(pi) => pi.ident.clone(),
                    _ => format_ident!("arg_{}", idx),
                };
                data_params.push((name, ty.as_ref()));
            }
        }
    }

    let mut param_extractors = Vec::new();
    let mut fn_call_args = Vec::new();
    let mut data_offset: usize = 0;

    let has_vm_param = vm_param_idx.is_some();
    if has_vm_param {
        fn_call_args.push(quote!(vm));
    }

    for (param_ident, param_type) in &data_params {
        let offset_lit = data_offset;
        let reg_expr = if offset_lit == 0 {
            quote!(arg_idx)
        } else {
            quote!((arg_idx.wrapping_add(#offset_lit)) % 256)
        };

        if is_string_type(param_type) {
            param_extractors.push(quote! {
                let #param_ident = match vm.read_string(vm.registers[#reg_expr] as usize, Some(65536)) {
                    Ok(s) => s,
                    Err(_) => {
                        vm.registers[dest_idx] = 0;
                        return;
                    }
                };
            });
            fn_call_args.push(quote!(#param_ident));
            data_offset = data_offset.wrapping_add(1);
        } else if is_str_slice_type(param_type) {
            let temp_ident = format_ident!("{}_str", param_ident);
            param_extractors.push(quote! {
                let #temp_ident = match vm.read_string(vm.registers[#reg_expr] as usize, Some(65536)) {
                    Ok(s) => s,
                    Err(_) => {
                        vm.registers[dest_idx] = 0;
                        return;
                    }
                };
                let #param_ident = #temp_ident.as_str();
            });
            fn_call_args.push(quote!(#param_ident));
            data_offset = data_offset.wrapping_add(1);
        } else if is_vec_u8_type(param_type) || is_byte_slice_type(param_type) {
            let len_reg_expr = quote!((arg_idx.wrapping_add(#offset_lit + 1)) % 256);
            let temp_ident = format_ident!("{}_vec", param_ident);
            if is_byte_slice_type(param_type) {
                param_extractors.push(quote! {
                    let #temp_ident = match vm.read_bytes(vm.registers[#reg_expr] as usize, vm.registers[#len_reg_expr] as usize, false) {
                        Ok(b) => b,
                        Err(_) => {
                            vm.registers[dest_idx] = 0;
                            return;
                        }
                    };
                    let #param_ident = #temp_ident.as_slice();
                });
            } else {
                param_extractors.push(quote! {
                    let #param_ident = match vm.read_bytes(vm.registers[#reg_expr] as usize, vm.registers[#len_reg_expr] as usize, false) {
                        Ok(b) => b,
                        Err(_) => {
                            vm.registers[dest_idx] = 0;
                            return;
                        }
                    };
                });
            }
            fn_call_args.push(quote!(#param_ident));
            data_offset = data_offset.wrapping_add(2);
        } else if is_bool_type(param_type) {
            param_extractors.push(quote! {
                let #param_ident = vm.registers[#reg_expr] != 0;
            });
            fn_call_args.push(quote!(#param_ident));
            data_offset = data_offset.wrapping_add(1);
        } else {
            param_extractors.push(quote! {
                let #param_ident = vm.registers[#reg_expr];
            });
            fn_call_args.push(quote!(#param_ident as _));
            data_offset = data_offset.wrapping_add(1);
        }
    }

    let sig_output = &func.sig.output;
    let ret_type_str = quote!(#sig_output).to_string();

    let return_handler = if ret_type_str.contains("Result") && ret_type_str.contains("String") {
        quote! {
            match res {
                Ok(out_str) => {
                    let alloc_addr = match vm.get_host_context_mut() {
                        Some(ctx) => ctx.allocate_vm_memory(out_str.len() + 1),
                        None => 512,
                    };
                    if vm.write_string(alloc_addr, &out_str, true).is_ok() {
                        vm.registers[dest_idx] = alloc_addr as u32;
                    } else {
                        vm.registers[dest_idx] = 0;
                    }
                }
                Err(code) => {
                    vm.registers[dest_idx] = code as u32;
                }
            }
        }
    } else if ret_type_str.contains("Result") && (ret_type_str.contains("Vec") || ret_type_str.contains("u8")) {
        quote! {
            match res {
                Ok(out_bytes) => {
                    let alloc_addr = match vm.get_host_context_mut() {
                        Some(ctx) => ctx.allocate_vm_memory(out_bytes.len()),
                        None => 512,
                    };
                    if vm.write_bytes(alloc_addr, &out_bytes).is_ok() {
                        vm.registers[dest_idx] = alloc_addr as u32;
                    } else {
                        vm.registers[dest_idx] = 0;
                    }
                }
                Err(code) => {
                    vm.registers[dest_idx] = code as u32;
                }
            }
        }
    } else if ret_type_str.contains("Result") {
        quote! {
            match res {
                Ok(val) => {
                    vm.registers[dest_idx] = val as u32;
                }
                Err(code) => {
                    vm.registers[dest_idx] = code as u32;
                }
            }
        }
    } else if ret_type_str.contains("String") {
        quote! {
            let alloc_addr = match vm.get_host_context_mut() {
                Some(ctx) => ctx.allocate_vm_memory(res.len() + 1),
                None => 512,
            };
            if vm.write_string(alloc_addr, &res, true).is_ok() {
                vm.registers[dest_idx] = alloc_addr as u32;
            } else {
                vm.registers[dest_idx] = 0;
            }
        }
    } else if ret_type_str.contains("Vec") {
        quote! {
            let alloc_addr = match vm.get_host_context_mut() {
                Some(ctx) => ctx.allocate_vm_memory(res.len()),
                None => 512,
            };
            if vm.write_bytes(alloc_addr, &res).is_ok() {
                vm.registers[dest_idx] = alloc_addr as u32;
            } else {
                vm.registers[dest_idx] = 0;
            }
        }
    } else if func.sig.output == ReturnType::Default {
        quote! {
            vm.registers[dest_idx] = 1;
        }
    } else if ret_type_str.contains("bool") {
        quote! {
            vm.registers[dest_idx] = if res { 1 } else { 0 };
        }
    } else {
        quote! {
            vm.registers[dest_idx] = res as u32;
        }
    };

    let match_arm = quote! {
        #id => {
            #(#param_extractors)*
            let res = #fn_name(#(#fn_call_args),*);
            #return_handler
        }
    };

    ProcessedCmd {
        match_arm,
    }
}

/// Package attribute macro to generate VM dispatchers and registration extension traits
#[proc_macro_attribute]
pub fn sgl_package(attr: TokenStream, item: TokenStream) -> TokenStream {
    let pkg_args = parse_macro_input!(attr as PackageArgs);
    let mut item_mod = parse_macro_input!(item as ItemMod);

    let mod_ident = item_mod.ident.clone();
    let default_name = mod_ident.to_string();
    let pkg_name = pkg_args.name.unwrap_or(default_name);
    let pkg_kind = pkg_args.kind.unwrap_or_else(|| "hardware".to_string());

    let trait_name = format_ident!("{}RegisterExt", to_pascal_case(&pkg_name));
    let reg_method = format_ident!("register_{}", to_snake_case(&pkg_name));

    let mut processed_cmds = Vec::new();
    let mut next_auto_id: u32 = 1;

    if let Some((_, items)) = &mut item_mod.content {
        for item in items.iter_mut() {
            if let syn::Item::Fn(func) = item {
                let mut cmd_id = None;
                let mut is_cmd = false;

                func.attrs.retain(|attr| {
                    let attr_str = quote!(#attr).to_string();
                    if attr_str.contains("sgl_cmd") || attr_str.contains("sgl_syscall") || attr_str.contains("sgl_hardware_call") {
                        is_cmd = true;
                        if let Meta::List(ml) = &attr.meta
                            && let Ok(parsed_args) = syn::parse2::<CmdArgs>(ml.tokens.clone())
                            && let Some(id) = parsed_args.id
                        {
                            cmd_id = Some(id);
                        }
                        false
                    } else {
                        true
                    }
                });

                if is_cmd || func.vis != syn::Visibility::Inherited {
                    let id = cmd_id.unwrap_or_else(|| {
                        let assigned = next_auto_id;
                        next_auto_id += 1;
                        assigned
                    });
                    processed_cmds.push(build_cmd_match_arm(id, func));
                }
            }
        }

        let match_arms = processed_cmds.iter().map(|c| &c.match_arm);

        let dispatch_fn: syn::Item = syn::parse_quote! {
            pub fn dispatch(vm: &mut script_go::sgl::vm::ScriptVm, dest_reg: usize, cmd_reg: usize, arg_reg: usize) {
                let dest_idx = dest_reg % 256;
                let cmd_idx = cmd_reg % 256;
                let arg_idx = arg_reg % 256;

                let reg_cmd = vm.registers[cmd_idx];
                let direct_cmd = cmd_reg as u32;
                let direct_arg = arg_reg as u32;

                let cmd_id = if reg_cmd != 0 {
                    reg_cmd
                } else if direct_cmd != 0 {
                    direct_cmd
                } else {
                    direct_arg
                };

                let catch_res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    match cmd_id {
                        #(#match_arms)*
                        _ => {}
                    }
                }));

                if catch_res.is_err() {
                    vm.registers[dest_idx] = 0;
                }
            }
        };

        let dispatch_syscall_fn: syn::Item = syn::parse_quote! {
            pub fn dispatch_syscall(_a: u32, _b: u32, _c: u32) {
                // Syscall handler signature adapter
            }
        };

        let reg_method_name = if pkg_kind == "syscall" {
            quote!(register_syscall_handler_ext)
        } else {
            quote!(register_hardware_handler)
        };

        let register_fn: syn::Item = syn::parse_quote! {
            pub fn register(vm: &mut script_go::sgl::vm::ScriptVm) {
                vm.#reg_method_name(dispatch);
            }
        };

        items.push(dispatch_fn);
        items.push(dispatch_syscall_fn);
        items.push(register_fn);
    }

    let reg_method_name = if pkg_kind == "syscall" {
        quote!(register_syscall_handler_ext)
    } else {
        quote!(register_hardware_handler)
    };

    let expanded = quote! {
        #item_mod

        pub trait #trait_name {
            fn #reg_method(&mut self);
        }

        impl #trait_name for script_go::sgl::vm::ScriptVm {
            fn #reg_method(&mut self) {
                self.#reg_method_name(#mod_ident::dispatch);
            }
        }
    };

    TokenStream::from(expanded)
}

/// Proc macro attribute #[sgl_syscall]
#[proc_macro_attribute]
pub fn sgl_syscall(attr: TokenStream, item: TokenStream) -> TokenStream {
    if let Ok(func) = syn::parse::<ItemFn>(item.clone()) {
        let cmd_args = parse_macro_input!(attr as CmdArgs);
        let id = cmd_args.id.unwrap_or(1);
        let fn_name = &func.sig.ident;
        let handler_name = format_ident!("{}_handler", fn_name);
        let reg_name = format_ident!("register_{}", fn_name);

        let cmd = build_cmd_match_arm(id, &func);
        let match_arm = &cmd.match_arm;

        let expanded = quote! {
            #func

            pub fn #handler_name(vm: &mut script_go::sgl::vm::ScriptVm, dest_reg: usize, cmd_reg: usize, arg_reg: usize) {
                let dest_idx = dest_reg % 256;
                let _cmd_idx = cmd_reg % 256;
                let arg_idx = arg_reg % 256;

                let catch_res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    match #id {
                        #match_arm
                        _ => {
                            vm.registers[dest_idx] = 0;
                        }
                    }
                }));

                if catch_res.is_err() {
                    vm.registers[dest_idx] = 0;
                }
            }

            pub fn #reg_name(vm: &mut script_go::sgl::vm::ScriptVm) {
                vm.register_hardware_handler(#handler_name);
            }
        };
        TokenStream::from(expanded)
    } else {
        item
    }
}

/// Proc macro attribute #[sgl_hardware_call]
#[proc_macro_attribute]
pub fn sgl_hardware_call(attr: TokenStream, item: TokenStream) -> TokenStream {
    sgl_syscall(attr, item)
}

/// Proc macro attribute #[sgl_cmd]
#[proc_macro_attribute]
pub fn sgl_cmd(attr: TokenStream, item: TokenStream) -> TokenStream {
    sgl_syscall(attr, item)
}

struct PathList {
    paths: Punctuated<syn::Path, Token![,]>,
}

impl Parse for PathList {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let paths = Punctuated::<syn::Path, Token![,]>::parse_terminated(input)?;
        Ok(PathList { paths })
    }
}

/// Macro for combining multiple VM handlers into a single hardware call handler
#[proc_macro]
pub fn sgl_combine_handlers(input: TokenStream) -> TokenStream {
    let path_list = parse_macro_input!(input as PathList);
    let handlers = path_list.paths.iter();

    let expanded = quote! {
        |vm: &mut script_go::sgl::vm::ScriptVm, a: usize, b: usize, c: usize| {
            let initial_dest = vm.registers[a % 256];
            #(
                #handlers(vm, a, b, c);
                if vm.registers[a % 256] != initial_dest {
                    return;
                }
            )*
        }
    };

    TokenStream::from(expanded)
}
