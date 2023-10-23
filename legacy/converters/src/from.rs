use proc_macro2::{Ident, TokenStream};
use quote::{quote, ToTokens};
use syn::{punctuated::Punctuated, Data, DataEnum, DataStruct, Field, Fields, Token};

use crate::{
    error,
    parse::{FieldArgs, Input, MacroArgs},
    take_arg, EnumVariant, NamedField, RhsEnumVariant,
};

pub fn quote_body(
    input: &mut Input,
    convert_stream: &TokenStream,
    maybe_try: &TokenStream,
) -> TokenStream {
    match input.macro_args.constructor.take() {
        Some(c) => c,
        None => match &mut input.input.data {
            Data::Struct(s) => {
                let fields = quote_struct_fields(
                    &mut input.macro_args,
                    &mut input.field_args,
                    s,
                    convert_stream,
                    maybe_try,
                );

                quote! {
                    Self {
                        #fields
                    }
                }
            },
            Data::Enum(e) => {
                let fields = quote_enum_fields(
                    &mut input.macro_args,
                    &mut input.field_args,
                    &input.input.ident,
                    e,
                    convert_stream,
                    maybe_try,
                );

                quote! {
                    match other {
                        #fields
                    }
                }
            },
            Data::Union(u) => return error(u.union_token.span, "unions are not supported"),
        },
    }
}

fn quote_enum_fields(
    macro_args: &mut MacroArgs,
    field_args: &mut Vec<Vec<FieldArgs>>,
    for_type: &Ident,
    data_enum: &mut DataEnum,
    convert_stream: &TokenStream,
    maybe_try: &TokenStream,
) -> TokenStream {
    let other_type = &macro_args.kind;
    let variants = field_args.into_iter().enumerate().map(|(i, mut args)| {
        let variant = &data_enum.variants[i];
        let variant_ident = &variant.ident;
        if args.is_empty() {
            if macro_args.skipping {
                quote! {
                    #other_type::#variant_ident {..} => {
                        #for_type::#variant_ident { ..::core::default::Default::default() }
                    },
                }
            } else {
                let enum_variant = EnumVariant::new(&variant.fields);
                let rhs = RhsEnumVariant {
                    enum_variant: &enum_variant,
                    rhs: |ident: &dyn ToTokens| {
                        quote! { #convert_stream(#ident)#maybe_try }
                    },
                };
                quote! {
                    #other_type::#variant_ident { #enum_variant } => {
                        #for_type::#variant_ident { #rhs }
                    },
                }
            }
        } else {
            let arg = take_arg(&macro_args.kind, args);
            if arg.skip {
                // return quote! {
                //     #other_type::#variant_ident {..} => {
                //         #for_type::#variant_ident { ..::core::default::Default::default() }
                //     },
                // };
                return quote!();
            }
            let enum_variant = EnumVariant::new(&variant.fields);
            let rhs = RhsEnumVariant {
                enum_variant: &enum_variant,
                rhs: |ident: &dyn ToTokens| {
                    quote! { #convert_stream(#ident)#maybe_try }
                },
            };
            if let Some(with) = arg.with {
                let name = if let Some(rename) = &arg.rename {
                    rename
                } else {
                    variant_ident
                };
                return quote! {
                    #other_type::#name { #enum_variant } => {
                        #with
                    },
                };
            }
            if let Some(rename) = arg.rename {
                return quote! {
                    #other_type::#rename { #enum_variant } => {
                        #for_type::#variant_ident { #rhs }
                    },
                };
            }
            if arg.include {
                return quote! {
                    #other_type::#variant_ident { #enum_variant } => {
                        #for_type::#variant_ident { #rhs }
                    },
                };
            }
            unreachable!()
        }
    });
    if let Some(default) = &macro_args.default {
        quote! {
            #(#variants)*
            _ => #default,
        }
    } else {
        quote! { #(#variants)* }
    }
}

fn quote_struct_fields(
    macro_args: &mut MacroArgs,
    field_args: &mut Vec<Vec<FieldArgs>>,
    data_struct: &mut DataStruct,
    convert_stream_start: &TokenStream,
    maybe_try: &TokenStream,
) -> TokenStream {
    let no_fields = Punctuated::<Field, Token![,]>::new();
    let fields = match &data_struct.fields {
        Fields::Named(named) => &named.named,
        Fields::Unnamed(unnamed) => &unnamed.unnamed,
        Fields::Unit => &no_fields,
    };
    let fields = field_args.iter_mut().enumerate().map(|(i, mut args)| {
        let field = &fields[i];
        let field_name = NamedField {
            i,
            name: &field.ident,
        };
        if args.is_empty() {
            if macro_args.skipping {
                quote! { #field_name: ::core::default::Default::default(), }
            } else {
                quote! { #field_name: #convert_stream_start(other.#field_name)#maybe_try, }
            }
        } else {
            let arg = take_arg(&macro_args.kind, args);
            if arg.include {
                return quote! { #field_name: #convert_stream_start(other.#field_name)#maybe_try, };
            }
            if arg.skip {
                return quote! { #field_name: ::core::default::Default::default(), };
            }
            if let Some(rename) = arg.rename {
                return quote! { #field_name: #convert_stream_start(other.#rename)#maybe_try, };
            }
            if let Some(with) = arg.with {
                return quote! { #field_name: #with, };
            }
            unreachable!()
        }
    });

    quote! { #(#fields)* }
}
