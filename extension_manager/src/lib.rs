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

/// Import all global varables.
#[proc_macro]
pub fn import_global_variables(_input: TokenStream) -> TokenStream {
    let calls = CRATES.iter().map(|crate_name| {
        let ext_name = crate_name
            .strip_prefix("hikami_")
            .expect("Crate name should start with 'hikami_'");
        let global_var_name = format!("{}_DATA", ext_name.to_uppercase());

        let crate_ident = Ident::new(crate_name, Span::call_site());
        let global_var_ident = Ident::new(&global_var_name, Span::call_site());

        quote! {
            use #crate_ident::#global_var_ident;
        }
    });

    let expanded = quote! {
        #(#calls)*
    };

    TokenStream::from(expanded)
}

/// Initialize all global variables.
#[proc_macro]
pub fn initialize(_input: TokenStream) -> TokenStream {
    let calls = CRATES.iter().map(|crate_name| {
        let ext_name = crate_name
            .strip_prefix("hikami_")
            .expect("Crate name should start with 'hikami_'");
        // crate name format: hikami_extension-name
        let mut struct_name_chars = ext_name.chars();
        let struct_name = match struct_name_chars.next() {
            None => String::new(),
            Some(c) => c.to_uppercase().collect::<String>() + struct_name_chars.as_str(),
        };
        let global_var_name = format!("{}_DATA", ext_name.to_uppercase());

        let crate_ident = Ident::new(crate_name, Span::call_site());
        let struct_ident = Ident::new(&struct_name, Span::call_site());
        let global_var_ident = Ident::new(&global_var_name, Span::call_site());

        quote! {
            use #crate_ident::{#struct_ident, #global_var_ident};
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
