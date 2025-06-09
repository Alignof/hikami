//! Hikami extension manager.
//!
//! It load extension emulation module as crate and expand function calls from macro.

extern crate proc_macro;
extern crate proc_macro2;

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::Ident;

include!(concat!(env!("OUT_DIR"), "/dependencies.rs"));

/// Initialize all global variable.
#[proc_macro]
pub fn initialize(_input: TokenStream) -> TokenStream {
    let calls = CRATES.iter().map(|name| {
        // crate name format: hikami_modulename
        let module_name = name
            .strip_prefix("hikami_")
            .expect("Crate name should start with 'hikami_'");
        let mut struct_name_chars = module_name.chars();
        let struct_name = match struct_name_chars.next() {
            None => String::new(),
            Some(c) => c.to_uppercase().collect::<String>() + struct_name_chars.as_str(),
        };
        let global_var_name = format!("{}_DATA", module_name.to_uppercase());

        let module_ident = Ident::new(module_name, Span::call_site());
        let struct_ident = Ident::new(&struct_name, Span::call_site());
        let global_var_ident = Ident::new(&global_var_name, Span::call_site());

        quote! {
            use #module_ident::{#struct_ident, #global_var_ident};
            unsafe {
                #global_var_ident.lock().get_or_init(#struct_ident::new);
            }
        }
    });

    let expanded = quote! {
        #(#calls)*
    };

    TokenStream::from(expanded)
}
