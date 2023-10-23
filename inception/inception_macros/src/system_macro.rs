use inception_macros_core::*;
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{format_ident, quote};
use syn::{
    parse_macro_input, parse_quote, punctuated::Punctuated, FnArg, GenericArgument, GenericParam,
    ItemFn, PathArguments, Token, Type, TypeParamBound,
};

pub fn system(attributes: TokenStream, item: TokenStream) -> TokenStream {
    let inception = macros_util::crate_name("inception");
    let mut input = parse_macro_input!(item as ItemFn);
    let _attributes = parse_macro_input!(attributes as syn::AttributeArgs);
    let vis = input.vis.clone();
    input.vis = parse_quote!(pub(super));
    let struct_name = input.sig.ident.clone();
    let fn_name = format_ident!("__{}", input.sig.ident);
    input.sig.ident = fn_name;
    let fn_name = &input.sig.ident;
    let output = &input.sig.output;
    expand_inputs(input.sig.inputs.iter_mut().map(|x| match x {
        syn::FnArg::Receiver(_) => panic!("Only functions without `self` receiver are allowed"),
        syn::FnArg::Typed(x) => &mut *x.ty,
    }));
    expand_generics(&mut input.sig.generics.params, &inception, true);
    let expanded_args = input
        .sig
        .inputs
        .iter()
        .map(|x| {
            if let FnArg::Typed(x) = x {
                &x.ty
            } else {
                panic!("system functions cannot have a self argument");
            }
        })
        .collect::<Vec<_>>();
    let mut stripped_args = input.sig.inputs.clone();
    strip_lifetimes(&mut stripped_args);
    let stripped_args = stripped_args.iter().map(|x| {
        if let FnArg::Typed(x) = x {
            &x.ty
        } else {
            panic!("system functions cannot have a self argument");
        }
    });
    let expanded_fn_ptr = quote! {
        fn(#(#expanded_args),*) -> (#output)
    };
    let _fn_ptr = quote! {
        fn(#(#stripped_args),*) -> (#output)
    };
    let where_clause_predicates = input
        .sig
        .generics
        .where_clause
        .iter()
        .flat_map(|x| x.predicates.iter())
        .filter(|x| match x {
            // syn::WherePredicate::Type(x) => x.ident != "W",
            syn::WherePredicate::Type(x) => !is_world(&x.bounds),
            _ => false,
        })
        .collect::<Vec<_>>();

    let struct_generics = input
        .sig
        .generics
        .params
        .iter()
        .filter(|x| match x {
            syn::GenericParam::Type(x) => !is_world(&x.bounds),
            _ => false,
        })
        .collect::<Vec<_>>();
    let struct_type_generics = struct_generics
        .iter()
        .map(|x| match x {
            syn::GenericParam::Type(x) => &x.ident,
            syn::GenericParam::Lifetime(x) => &x.lifetime.ident,
            syn::GenericParam::Const(x) => &x.ident,
        })
        .collect::<Vec<_>>();
    let turbofish = input.sig.generics.params.iter().filter_map(|x| match x {
        GenericParam::Type(x) => Some(&x.ident),
        GenericParam::Const(x) => Some(&x.ident),
        _ => None,
    });
    let param_names = input
        .sig
        .inputs
        .iter()
        .enumerate()
        .map(|(_i, x)| match x {
            FnArg::Receiver(_) => panic!("system functions cannot have a self argument"),
            FnArg::Typed(x) => match &*x.pat {
                syn::Pat::Ident(x) => {
                    let name = &x.ident;
                    quote! {
                        #name
                    }
                },
                _ => panic!("Only `name: Type` is supported"),
            },
        })
        .collect::<Vec<_>>();
    let param_names_str = input.sig.inputs.iter().enumerate().map(|(i, x)| match x {
        FnArg::Receiver(_) => panic!("system functions cannot have a self argument"),
        FnArg::Typed(x) => match &*x.pat {
            syn::Pat::Ident(x) => {
                let name = &x.ident;
                quote! {
                    #i => Some(core::any::type_name::<#name>())
                }
            },
            _ => panic!("Only `name: Type` is supported"),
        },
    });
    let system_inputs = input
        .sig
        .inputs
        .iter()
        .enumerate()
        .filter_map(|(i, x)| {
            if let FnArg::Typed(x) = x {
                let ident = if let syn::Pat::Ident(x) = &*x.pat {
                    &x.ident
                } else {
                    return None;
                };
                if let Type::Path(path) = &*x.ty {
                    if let Some(segment) = path.path.segments.last() {
                        if segment.ident == "In" || segment.ident == "PhantomIn" {
                            match &segment.arguments {
                                PathArguments::AngleBracketed(args) => {
                                    if let Some(arg) = args.args.iter().find_map(|x| match x {
                                        GenericArgument::Type(Type::Path(path)) => {
                                            if path.path.segments.iter().all(|x| x.ident != "W") {
                                                return Some(path);
                                            }
                                            None
                                        },
                                        _ => None,
                                    }) {
                                        let mut param = quote! { #ident }.to_string();
                                        param.push('_');
                                        let param = syn::Ident::new(&param, Span::call_site());
                                        return Some((i, param, arg));
                                    }
                                },
                                _ => return None,
                            }
                        }
                    }
                }
                None
            } else {
                panic!("system functions cannot have a self argument");
            }
        })
        .collect::<Vec<_>>();
    let phantom_generics = struct_generics.iter().enumerate().map(|(i, x)| match x {
        syn::GenericParam::Type(x) => {
            let field = format_ident!("_{i}");
            let ident = &x.ident;
            quote! {
                #field: core::marker::PhantomData<#ident>
            }
        },
        syn::GenericParam::Lifetime(x) => {
            let field = format_ident!("_{i}");
            let lifetime = &x.lifetime;
            quote! {
                #field: core::marker::PhantomData<#lifetime ()>
            }
        },
        syn::GenericParam::Const(_) => quote! {},
    });
    let system_input_fields = system_inputs.iter().map(|x| {
        let field = &x.1;
        let ty = x.2;
        quote! {
            #field: #ty
        }
    });
    let struct_fields = system_input_fields.chain(phantom_generics).peekable();
    let struct_body = quote! {{
        #(#struct_fields),*
    }};
    let default_fields = struct_generics.iter().enumerate().map(|(i, x)| match x {
        syn::GenericParam::Type(_) | syn::GenericParam::Lifetime(_) => {
            let field = format_ident!("_{i}");
            quote! {
                #field: core::default::Default::default()
            }
        },
        syn::GenericParam::Const(_) => quote! {},
    });
    let new_inputs = system_inputs.iter().map(|x| {
        let field = &x.1;
        let ty = x.2;
        quote! {
            #field: #ty
        }
    });
    let new_inputs_constructed = system_inputs.iter().map(|x| {
        let field = &x.1;
        quote! {
            #field
        }
    });
    let create_input_item = system_inputs.iter().map(|x| {
        let param_id = x.0;
        let field = &x.1;
        quote! {
            #inception::InputItem {
                data: Some(self.#field),
                param_name: core::marker::PhantomData::<<Self as inception::Mapper::<#param_id>>::Name>,
            }
        }
    });
    let impl_mapper = (0..16).map(|param_id| {
        let param_name = match param_names.get(param_id) {
            Some(name) => quote! {#name},
            None => quote! {()},
        };
        quote! {
            #[automatically_derived]
            impl<#(#struct_generics),*> #inception::Mapper<#param_id> for
                System<#(#struct_type_generics),*>
            where
                #(#where_clause_predicates),*
            {
                type Name = #param_name;
            }
        }
    });
    // `SystemLabel` is disabled for generic systems.
    let system_label = if struct_type_generics.is_empty() {
        quote! {
            System
        }
    } else {
        quote! {
            #inception::UnknownSystem
        }
    };

    let output = quote! {
        #vis mod #struct_name {
            #![allow(non_camel_case_types)]
            use super::*;
            use inception::*;

            pub struct System<#(#struct_type_generics),*> #struct_body
            #(
                pub struct #param_names;
                impl #inception::ParamLabel for #param_names {
                    type System = #system_label;
                }
            )*

            #[inline(always)]
            pub fn new<#(#struct_generics),*>(#(#new_inputs),*) -> System<#(#struct_type_generics),*>
            where
                #(#where_clause_predicates),*
            {
                System {
                    #(#new_inputs_constructed,)*
                    #(#default_fields),*
                }
            }

            #[automatically_derived]
            impl<'w, 's, #(#struct_generics),*> #inception::SystemBuilder<'w, 's> for
                System<#(#struct_type_generics),*>
            where
                #(#where_clause_predicates),*
            {
                type System<W: #inception::World, const N: usize> =
                    self::def::System<'w, 's, #(#struct_type_generics,)* N>;

                #[inline(always)]
                fn build<W: #inception::World, const N: usize>(
                    self,
                    ___build_world: &mut W,
                ) -> Self::System<W, N> {
                    use #inception::Nestable;
                    let mut ___build_inputs = #inception::StackedNest;
                    #(
                        let mut ___build_inputs = ___build_inputs.push(#create_input_item);
                    )*
                    self::def::System {
                        state: SystemState::new::<W, self::System<#(#struct_type_generics),*>, _>(
                            ___build_world,
                            &mut ___build_inputs,
                        ),
                        _p: Default::default(),
                    }
                }
            }

            #[automatically_derived]
            impl<#(#struct_generics),*> #inception::SystemParamNameMapper for
                System<#(#struct_type_generics),*>
            where
                #(#where_clause_predicates),*
            {
                #[inline(always)]
                fn get_param_name<const PARAM_ID: usize>() -> Option<&'static str> {
                    match PARAM_ID {
                        #(#param_names_str,)*
                        _ => None,
                    }
                }
            }

            #(#impl_mapper)*

            // Creating another module to avoid param name conflicts with their type names.
            mod def {
                use inception::*;
                use super::super::*;
                pub struct System<'w, 's, #(#struct_generics,)* const N: usize>
                where
                    #(#where_clause_predicates),*
                {
                    pub(super) state: SystemState<(#(#expanded_args,)*)>,
                    pub(super) _p: PhantomSystemParam<'w, 's, N>,
                }
                impl<'w, 's, #(#struct_generics,)* W: World, const N: usize> inception::System<'w, 's, W> for
                    System<'w, 's, #(#struct_type_generics,)* N>
                where
                    #(#where_clause_predicates),*
                {
                    #[inline(always)]
                    fn call(&'s mut self, world: &'w mut W) {
                        self.state
                            .call::<_, super::System<#(#struct_type_generics),*>, _>(
                            &mut #fn_name::<#(#turbofish),*>,
                            world
                        );
                    }
                }

                impl<'w, 's, #(#struct_generics,)* const N: usize> SystemParamPlugin for
                    System<'w, 's, #(#struct_type_generics,)* N>
                where
                    #(#where_clause_predicates),*
                {
                    type Build<B: EcsBuilder> =
                        <(#(#expanded_args,)*) as SystemParam>::Build<B, super::System<#(#struct_type_generics),*>, ()>;

                    #[inline(always)]
                    fn build<B: EcsBuilder>(mut builder: B) -> Self::Build<B> {
                        <(#(#expanded_args,)*) as SystemParam>::build::<
                            B,
                            super::System<#(#struct_type_generics),*>,
                            (),
                        >(builder)
                    }
                }
                #[allow(clippy::too_many_arguments)]
                #[inline(always)]
                #input
            }

        }
    };
    // eprintln!("{}", output.to_string());
    output.into()
}

fn is_world(bounds: &Punctuated<TypeParamBound, Token![+]>) -> bool {
    bounds.iter().any(|x| match x {
        syn::TypeParamBound::Trait(x) => x
            .path
            .segments
            .last()
            .map(|x| x.ident == "World")
            .unwrap_or(false),
        _ => false,
    })
}

fn strip_lifetimes(inputs: &mut Punctuated<FnArg, Token![,]>) {
    fn remove_lifetimes(x: &mut Type) {
        match x {
            Type::Reference(x) => {
                if let Some(lifetime) = &mut x.lifetime {
                    if lifetime.ident == "w" || lifetime.ident == "s" {
                        x.lifetime = None;
                    }
                }
            },
            Type::Tuple(x) => {
                for x in &mut x.elems {
                    remove_lifetimes(x);
                }
            },
            Type::Path(x) => {
                if let PathArguments::AngleBracketed(x) =
                    &mut x.path.segments.last_mut().unwrap().arguments
                {
                    x.args = x
                        .args
                        .iter()
                        .cloned()
                        .filter(|x| match x {
                            GenericArgument::Lifetime(x) => x.ident != "w" && x.ident != "s",
                            _ => true,
                        })
                        .collect();
                    for x in &mut x.args {
                        if let GenericArgument::Type(x) = x {
                            remove_lifetimes(x);
                        }
                    }
                }
            },
            _ => {},
        }
    }
    for input in inputs {
        match input {
            FnArg::Receiver(_x) => panic!("system functions cannot have a self argument"),
            FnArg::Typed(x) => match &mut *x.ty {
                syn::Type::Reference(x) => {
                    if let Some(lifetime) = &mut x.lifetime {
                        if lifetime.ident == "w" || lifetime.ident == "s" {
                            x.lifetime = None;
                        }
                    }
                },
                syn::Type::Path(path) => {
                    let segment = path.path.segments.last_mut().unwrap();
                    if let PathArguments::AngleBracketed(param_generics) = &mut segment.arguments {
                        param_generics.args = param_generics
                            .args
                            .iter()
                            .cloned()
                            .filter(|x| match x {
                                GenericArgument::Lifetime(x) => x.ident != "w" && x.ident != "s",
                                _ => true,
                            })
                            .collect();

                        for arg in &mut param_generics.args {
                            if let GenericArgument::Type(x) = arg {
                                remove_lifetimes(x);
                            }
                        }
                    }
                },
                _ => (),
            },
        }
    }
}
