mod from;
mod into;
mod parse;

use std::convert::{TryFrom, TryInto};

use proc_macro2::{Ident, Punct, Spacing, Span, TokenStream, TokenTree};
use quote::{quote, quote_each_token, ToTokens, TokenStreamExt};
use syn::{
    parenthesized,
    parse::{Parse, ParseBuffer, Parser},
    parse_macro_input, parse_str,
    punctuated::Punctuated,
    spanned::Spanned,
    Attribute, Data, DataEnum, DataStruct, DeriveInput, Error, Field, Fields, Index, LitStr, Path,
    Result, Token, Type, TypePath,
};

use crate::parse::{FieldArgs, Input, MacroArgs};

macro_rules! define_macros {
    ($($macro:ident)+) => {
        $(
            #[proc_macro_attribute]
            pub fn $macro(
                args: proc_macro::TokenStream,
                input: proc_macro::TokenStream,
            ) -> proc_macro::TokenStream {
                match parse::parse(args, input, concat!("converters::", stringify!($macro))) {
                    Ok(ok) => quote(ok, stringify!($macro)).into(),
                    Err(e) => return e.into_compile_error().into(),
                }
            }
        )+
    }
}

define_macros!(from into try_from try_into);

fn quote(mut input: Input, kind: &str) -> TokenStream {
    let mut output = match kind {
        "from" => {
            let convert_stream = quote!(::core::convert::Into::into);
            let maybe_try = quote!();
            let other = quote!(other);
            let body = from::quote_body(&mut input, &convert_stream, &maybe_try);
            let for_type = &input.input.ident;
            let trait_type = &input.macro_args.kind;
            quote! {
                #[automatically_derived]
                impl ::core::convert::From<#trait_type> for #for_type {
                    #[inline]
                    fn from(other: #trait_type) -> Self {
                        #body
                    }
                }
            }
        },
        "try_from" => {
            let convert_stream = quote!(::core::convert::TryInto::try_into);
            let maybe_try = quote!(?);
            let other = quote!(other);
            let body = from::quote_body(&mut input, &convert_stream, &maybe_try);
            let for_type = &input.input.ident;
            let trait_type = &input.macro_args.kind;
            let error_kind = ErrorKind {
                kind: &input.macro_args.error,
            };
            let o = quote! {
                #[automatically_derived]
                impl ::core::convert::TryFrom<#trait_type> for #for_type {
                    type Error = #error_kind;

                    #[inline]
                    fn try_from(other: #trait_type) -> std::result::Result<Self, Self::Error> {
                        Ok(#body)
                    }
                }
            };
            println!("{}", o);
            o
        },
        "into" => {
            let convert_stream = quote!(::core::convert::Into::into);
            let maybe_try = quote!();
            let other = quote!(self);
            let body = into::quote_body(&mut input, &convert_stream, &maybe_try);
            let for_type = &input.input.ident;
            let trait_type = &input.macro_args.kind;
            quote! {
                #[automatically_derived]
                impl ::core::convert::Into<#trait_type> for #for_type {
                    #[inline]
                    fn into(self) -> #trait_type {
                        #body
                    }
                }
            }
        },
        "try_into" => {
            let convert_stream = quote!(::core::convert::TryInto::try_into);
            let maybe_try = quote!(?);
            let other = quote!(self);
            let body = into::quote_body(&mut input, &convert_stream, &maybe_try);
            let for_type = &input.input.ident;
            let trait_type = &input.macro_args.kind;
            let error_kind = ErrorKind {
                kind: &input.macro_args.error,
            };
            quote! {
                #[automatically_derived]
                impl ::core::convert::TryInto<#trait_type> for #for_type {
                    type Error = #error_kind;
                    #[inline]
                    fn try_into(self) -> std::result::Result<#trait_type, Self::Error> {
                        Ok(#body)
                    }
                }
            }
        },
        _ => unreachable!(),
    };
    input.input.to_tokens(&mut output);
    output
}

struct NamedField<'a> {
    name: &'a Option<Ident>,
    i: usize,
}

impl<'a> ToTokens for NamedField<'a> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self.name {
            None => {
                Index {
                    index: self.i as u32,
                    span: Span::call_site(),
                }
                .to_tokens(tokens);
            },
            Some(name) => name.to_tokens(tokens),
        }
    }
}

struct ErrorKind<'a> {
    kind: &'a Option<Type>,
}

impl<'a> ToTokens for ErrorKind<'a> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        if let Some(kind) = self.kind {
            kind.to_tokens(tokens);
        } else {
            quote!(std::boxed::Box<dyn std::error::Error>).to_tokens(tokens);
        }
    }
}

struct EnumVariant<'a> {
    fields: &'a Fields,
    aliases: Vec<Ident>,
}

impl<'a> EnumVariant<'a> {
    fn new(fields: &Fields) -> EnumVariant {
        let aliases = if !fields.is_empty() {
            fields
                .iter()
                .enumerate()
                .map(|(i, x)| Ident::new(&format!("arg{}", i), Span::call_site()))
                .collect()
        } else {
            vec![]
        };
        EnumVariant { fields, aliases }
    }
}

impl<'a> ToTokens for EnumVariant<'a> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let comma = Punct::new(',', Spacing::Alone);
        if !self.aliases.is_empty() {
            let colon = Punct::new(':', Spacing::Alone);
            for (i, alias) in self.aliases.iter().enumerate() {
                let index = Index {
                    index: i as u32,
                    span: Span::call_site(),
                };
                index.to_tokens(tokens);
                colon.to_tokens(tokens);
                alias.to_tokens(tokens);
                comma.to_tokens(tokens);
            }
        } else {
            for field in self.fields {
                field.to_tokens(tokens);
                comma.to_tokens(tokens);
            }
        }
    }
}

struct RhsEnumVariant<'a, Rhs> {
    enum_variant: &'a EnumVariant<'a>,
    rhs: Rhs,
}
impl<'a, Rhs> ToTokens for RhsEnumVariant<'a, Rhs>
where
    Rhs: Fn(&dyn ToTokens) -> TokenStream,
{
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let comma = Punct::new(',', Spacing::Alone);
        let colon = Punct::new(':', Spacing::Alone);
        if !self.enum_variant.aliases.is_empty() {
            for (i, alias) in self.enum_variant.aliases.iter().enumerate() {
                let index = Index {
                    index: i as u32,
                    span: Span::call_site(),
                };
                index.to_tokens(tokens);
                colon.to_tokens(tokens);
                alias.to_tokens(tokens);
                (self.rhs)(&index).to_tokens(tokens);
                comma.to_tokens(tokens);
            }
        } else {
            for field in self.enum_variant.fields {
                field.to_tokens(tokens);
                colon.to_tokens(tokens);
                field.to_tokens(tokens);
                (self.rhs)(&field).to_tokens(tokens);
                comma.to_tokens(tokens);
            }
        }
    }
}

fn take_arg(macro_kind: &Type, args: &mut Vec<FieldArgs>) -> FieldArgs {
    let mut at = 0;
    args.iter().enumerate().find(|(i, arg)| {
        if let Some(kind) = &arg.kind {
            if kind == macro_kind {
                at = *i;
                return true;
            }
            return false;
        }
        false
    });
    args.remove(at)
}

fn error(span: Span, msg: &str) -> TokenStream {
    Error::new(span, msg).into_compile_error()
}

// #[automatically_derived]
// impl ::core::convert::TryFrom<CandlesGetRequest> for GetTradeBucketedRequest {
//     type Error = std::boxed::Box<dyn std::error::Error>;
//     #[inline]
//     fn try_from(other: CandlesGetRequest) -> std::result::Result<Self, Self::Error> {
//         Ok(Self {
//             bin_size: ::core::convert::TryInto::try_into(other.timeframe)?,
//             partial: ::core::default::Default::default(),
//             symbol: ::core::convert::TryInto::try_into(other.symbol)?,
//             filter: ::core::default::Default::default(),
//             columns: ::core::default::Default::default(),
//             count: ::core::convert::TryInto::try_into(other.count)?,
//             reverse: ::core::default::Default::default(),
//             start: ::core::convert::TryInto::try_into(other.start_time)?,
//             end: ::core::convert::TryInto::try_into(other.end_time)?,
//         })
//     }
// }
//
// struct CandlesGetRequest {
//     timeframe: u32,
//     symbol: String,
//     count: Option<u32>,
//     start_time: Option<u32>,
//     end_time: Option<u32>,
// }
//
// #[derive(Debug, Clone, Eq, PartialEq)]
// enum BinSize {
//     M1,
//     M5,
// }
//
// impl Default for BinSize {
//     fn default() -> Self {
//         BinSize::M1
//     }
// }
//
// impl TryFrom<u32> for BinSize {
//     type Error = ();
//
//     fn try_from(value: u32) -> std::result::Result<Self, Self::Error> {
//         match value {
//             60 => Ok(BinSize::M1),
//             300 => Ok(BinSize::M5),
//             _ => Err(()),
//         }
//     }
// }
//
// #[derive(Clone, Default, Eq, PartialEq, Debug)]
// struct GetTradeBucketedRequest {
//     bin_size: BinSize,
//     partial: Option<bool>,
//     symbol: String,
//     filter: Option<()>,
//     columns: Option<()>,
//     count: Option<u32>,
//     reverse: Option<bool>,
//     start: Option<i64>,
//     end: Option<i64>,
// }

// impl<T, U> From<Option<U>> for Option<T> {
//     fn from(other: T) -> Self {
//         unimplemented!()
//     }
// }
//
// fn a() -> std::result::Result<(), Box<dyn std::error::Error>> {
//     let a: Option<i64> = Some(123);
//     let b: Option<u32> = a.try_into()?;
//
//     let a: i64 = 123;
//     let b: u32 = a.try_into()?;
//
//     Ok(())
// }
