use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input,
    token::Comma,
    Ident, LitInt, Path, Result,
};

struct AllTuples {
    macro_ident: Path,
    start: usize,
    end: usize,
    idents: Vec<Ident>,
}

impl Parse for AllTuples {
    fn parse(input: ParseStream) -> Result<Self> {
        let macro_ident = input.parse::<Path>()?;
        input.parse::<Comma>()?;
        let start = input.parse::<LitInt>()?.base10_parse()?;
        input.parse::<Comma>()?;
        let end = input.parse::<LitInt>()?.base10_parse()?;
        let mut idents = vec![];
        while input.parse::<Comma>().is_ok() {
            idents.push(input.parse::<Ident>()?);
        }

        Ok(AllTuples {
            macro_ident,
            start,
            end,
            idents,
        })
    }
}

#[proc_macro]
pub fn all_tuples(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as AllTuples);
    let len = input.end - input.start;
    let mut ident_tuples = Vec::with_capacity(len);
    for i in input.start..=input.end {
        let idents = input
            .idents
            .iter()
            .map(|ident| format_ident!("{}{}", ident, i));
        if input.idents.len() < 2 {
            ident_tuples.push(quote! {
                #(#idents)*
            });
        } else {
            ident_tuples.push(quote! {
                (#(#idents),*)
            });
        }
    }

    let macro_ident = &input.macro_ident;
    let invocations = (input.start..=input.end).map(|i| {
        let ident_tuples = &ident_tuples[..i];
        quote! {
            #macro_ident!(#(#ident_tuples),*);
        }
    });
    TokenStream::from(quote! {
        #(
            #invocations
        )*
    })
}

#[proc_macro]
pub fn repeat(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as AllTuples);
    let macro_ident = &input.macro_ident;
    let invocations = (input.start..input.end).map(|i| {
        let idents = input
            .idents
            .iter()
            .map(|ident| format_ident!("{}{}", ident, i));
        quote! {
            #macro_ident!(#(#idents),*);
        }
    });
    TokenStream::from(quote! {
        #(
            #invocations
        )*
    })
}

#[proc_macro]
pub fn param_to_const_expr(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as Ident);
    let name = input.to_string();
    // remove characters before first number
    let name = name.trim_start_matches(char::is_alphabetic);
    use core::str::FromStr;
    let number = usize::from_str(name).expect("Failed to parse number from param name");

    quote! {
        #number
    }
    .into()
}
