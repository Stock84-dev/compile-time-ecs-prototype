use proc_macro2::TokenStream;
use quote::quote;

pub fn crate_name(crate_name: &str) -> TokenStream {
    let ident = proc_macro2::Ident::new(crate_name, proc_macro2::Span::call_site());
    quote! {#ident}
    // let crate_name = match proc_macro_crate::crate_name(crate_name) {
    //     Ok(x) => x,
    //     Err(_) => panic!("Crate `{}` isn't present in `Cargo.toml`", crate_name),
    // };
    // match crate_name {
    //     proc_macro_crate::FoundCrate::Itself => quote! {crate},
    //     proc_macro_crate::FoundCrate::Name(x) => {
    //         let ident = proc_macro2::Ident::new(&x, proc_macro2::Span::call_site());
    //         quote! {#ident}
    //     },
    // }
}
