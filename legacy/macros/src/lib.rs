use std::{
    fmt::{write, Write},
    str::FromStr,
};

use convert_case::{Case, Casing};
use proc_macro2::{Ident, TokenStream, TokenTree};
use quote::{format_ident, quote, TokenStreamExt};
use syn::{
    parse::{Parse, ParseBuffer, ParseStream, Result},
    parse_macro_input,
    punctuated::Punctuated,
    token::Comma,
    Attribute, Block, Data, DeriveInput, Fields, ItemFn, LitInt, Token,
};

struct Attr {
    inner: Attribute,
}

impl Parse for Attr {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Attr {
            inner: input.call(Attribute::parse_outer)?.pop().unwrap(),
        })
    }
}

#[proc_macro_attribute]
/// Creates an empty function when feature is enabled
pub fn mock(
    args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let function = parse_macro_input!(input as ItemFn);
    let attr = parse_macro_input!(args as Attr).inner;
    let tokens = &attr.tokens;
    let not_attr_tokens = quote! {
        (not#tokens)
    };
    let mut not_attr = attr.clone();
    not_attr.tokens = not_attr_tokens;
    let mut mock_function = function.clone();
    mock_function.block = Box::new(
        syn::parse_macro_input::parse::<Block>(quote!({ Default::default() }).into()).unwrap(),
    );
    let stream = quote! {
        #not_attr
        #function
        #attr
        #mock_function
    };
    stream.into()
}

/// Creates token stream from string literal.
/// ```
/// use macros::destringify;
/// let mut foo_bar = false;
/// destringify!("foo_bar") = true;
///
/// assert!(foo_bar);
/// ```
#[proc_macro]
pub fn destringify(args: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let literal = args.to_string();
    proc_macro::TokenStream::from_str(&literal[1..literal.len() - 1]).unwrap()
}

#[proc_macro]
pub fn camel_case(args: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let s = args.to_string().to_case(Case::Camel);
    proc_macro::TokenStream::from_str(&s).unwrap()
}

#[proc_macro]
pub fn screaming_snake_case(args: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let s = args.to_string().to_case(Case::ScreamingSnake);
    proc_macro::TokenStream::from_str(&s).unwrap()
}

#[proc_macro_attribute]
/// Creates an empty function when feature is enabled
pub fn label(
    _args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let function = parse_macro_input!(input as ItemFn);

    let stream = quote! {
        #function
    };
    stream.into()
}

#[proc_macro_derive(Insert)]
pub fn insert(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    // Parse the input tokens into a syntax tree.
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let fields;
    match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(named_fields) => {
                fields = &named_fields.named;
            },
            _ => panic!("Only structs with named fileds are supported"),
        },
        _ => panic!("only structs are supported"),
    }
    let mut args = String::from("INSERT INTO {}(");
    for field in fields.iter().map(|x| x.ident.as_ref().unwrap()) {
        write!(&mut args, "{},", field.to_string()).unwrap();
    }
    args.pop();
    args.push_str(") VALUES (");
    for (i, _) in fields.iter().enumerate() {
        write!(&mut args, "${},", i + 1).unwrap();
    }
    args.pop();
    args.push_str(");");
    let binds = fields.iter().map(|x| {
        let name = x.ident.as_ref().unwrap();
        quote! {
            .bind(&self.#name)
        }
    });
    let expanded = quote! {
        impl #impl_generics #name #ty_generics #where_clause {
            async fn insert(
                &self,
                table_name: &str,
                pool: &sqlx::PgPool,
            ) -> sqlx::Result<sqlx::postgres::PgRow> {
                sqlx::query(&format!(#args, table_name))
                #(#binds)*
                .fetch_one(pool).await
            }
        }
    };

    // Hand the output tokens back to the compiler.
    proc_macro::TokenStream::from(expanded)
}

struct EnumerateArgs {
    items: Punctuated<TokenTree, Token![;]>,
}

impl Parse for EnumerateArgs {
    fn parse(input: &ParseBuffer) -> Result<Self> {
        Ok(Self {
            items: Punctuated::parse_terminated(input)?,
        })
    }
}

#[proc_macro]
/// Each item is prefixed by iteration number
pub fn enumerate(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    println!("{}", input.to_string());
    let items = parse_macro_input!(input as EnumerateArgs).items;
    let iter = items.iter().enumerate().map(|(i, v)| quote! {#v = #i;});
    let s: proc_macro::TokenStream = quote! {
        #(#iter);*
    }
    .into();
    println!("{}", s.to_string());
    s
}

struct AllTuples {
    macro_ident: Ident,
    start: usize,
    end: usize,
    idents: Vec<Ident>,
}

impl Parse for AllTuples {
    fn parse(input: ParseStream) -> Result<Self> {
        let macro_ident = input.parse::<Ident>()?;
        input.parse::<Comma>()?;
        let start = input.parse::<LitInt>()?.base10_parse()?;
        input.parse::<Comma>()?;
        let end = input.parse::<LitInt>()?.base10_parse()?;
        input.parse::<Comma>()?;
        let mut idents = vec![input.parse::<Ident>()?];
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
pub fn all_tuples(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as AllTuples);
    let len = (input.start..=input.end).count();
    let mut ident_tuples = Vec::with_capacity(len);
    for i in input.start..=input.end {
        let idents = input
            .idents
            .iter()
            .map(|ident| format_ident!("{}{}", ident, i));
        let j = i - input.start;
        if input.idents.len() < 2 {
            ident_tuples.push(quote! {
                #(#idents, #j)*
            });
        } else {
            ident_tuples.push(quote! {
                (#(#idents, #j),*)
            });
        }
    }

    let macro_ident = &input.macro_ident;
    let invocations = (input.start..=input.end).map(|i| {
        let ident_tuples = &ident_tuples[0..i];
        quote! {
            #macro_ident!(#(#ident_tuples),*);
        }
    });
    proc_macro::TokenStream::from(quote! {
        #(
            #invocations
        )*
    })
}
