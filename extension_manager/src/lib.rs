//! Hikami extension manager.
//!
//! It load extension emulation module as crate and expand function calls from macro.

extern crate proc_macro;
extern crate proc_macro2;

use proc_macro::TokenStream;
use quote::quote;
use syn::Ident;

include!(concat!(env!("OUT_DIR"), "/dependencies.rs"));

/// Test: call `func` function in all of modules.
#[proc_macro]
pub fn call_all_funcs(_input: TokenStream) -> TokenStream {
    let calls = CRATES.iter().map(|name| {
        let name = name.trim().replace('-', "_");
        let ident = Ident::new(&name, proc_macro2::Span::call_site());
        quote! { #ident::func(); }
    });

    let expanded = quote! {
        #(#calls)*
    };

    TokenStream::from(expanded)
}
