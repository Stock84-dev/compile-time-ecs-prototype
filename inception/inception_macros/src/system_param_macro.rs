use inception_macros_core::*;
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse_macro_input, parse_quote, punctuated::Punctuated, DeriveInput, FieldsNamed,
    FieldsUnnamed, GenericParam, Generics, Token, WhereClause, WherePredicate,
};

pub fn rename_generics(generics: &mut Punctuated<GenericParam, Token![,]>) {
    for param in generics.iter_mut() {
        match param {
            GenericParam::Type(x) => {
                if x.ident == "W" {
                    x.ident = parse_quote!(Wrld);
                }
            },
            GenericParam::Lifetime(x) => {
                if x.lifetime.ident == "w" {
                    x.lifetime.ident = parse_quote!(world);
                } else if x.lifetime.ident == "s" {
                    x.lifetime.ident = parse_quote!(state);
                }
            },
            GenericParam::Const(_x) => {},
        }
    }
}

pub fn system_param(_attributes: TokenStream, input: TokenStream) -> TokenStream {
    let inception = macros_util::crate_name("inception");
    let mut input = parse_macro_input!(input as DeriveInput);
    expand_generics(&mut input.generics.params, &inception, false);
    let states;
    let fields;
    let idents;
    let state_types;
    if let syn::Data::Struct(data_struct) = &mut input.data {
        match &mut data_struct.fields {
            syn::Fields::Named(FieldsNamed { named: x, .. })
            | syn::Fields::Unnamed(FieldsUnnamed { unnamed: x, .. }) => {
                let mut st = x.clone();
                let iter = st.iter_mut().map(|field| &mut field.ty);
                expand_inputs_with_names(iter, "'static", "'static", "W", "N");
                state_types = st;
                expand_inputs(
                    x.iter_mut()
                        // .filter(|x| {
                        //     // skip if it contains attribute skip
                        //     x.attrs.iter_mut().any(|x| {
                        //         if x.path.is_ident("system_param") {
                        //             let input: TokenStream = core::mem::take(&mut
                        // x.tokens).into();             let ident =
                        // parse_macro_input!(input as Ident);
                        // // syn::parse_macro_input::parse::<Ident>(&x.tokens)
                        //             ident.to_string() != "skip_expansion"
                        //         } else {
                        //             true
                        //         }
                        //     })
                        // })
                        .map(|field| &mut field.ty),
                );
                states = state_types
                    .iter()
                    .filter_map(|field| {
                        let ty = &field.ty;
                        Some(quote! {
                            <#ty as SystemParam>::State
                        })
                    })
                    .collect::<Vec<_>>();
                fields = x.clone();
                idents = x
                    .iter()
                    .enumerate()
                    .map(|(i, x)| x.ident.clone().unwrap_or_else(|| format_ident!("{}", i)))
                    .collect::<Vec<_>>();
            },
            _ => panic!("Unit structs aren't allowed"),
        }
    } else {
        panic!("Only structs are allowed");
    }
    let struct_name = &input.ident;
    let is_query = fields.iter().map(|x| {
        let ty = &x.ty;
        quote! { <#ty as SystemParam>::IS_QUERY}
    });
    let get = fields.iter().enumerate().map(|(_i, x)| {
        let ident = &x.ident;
        let ty = &x.ty;
        quote! {
            let #ident = <#ty>::get_param_for_entity::<Wrld, SB, ParamName, E>(
                &mut *__entity,
                &mut __state.#ident,
                &mut *__world,
            )?;
        }
    });
    let get_param_for_entity = quote! {
        #[inline(always)]
        fn get_param_for_entity<'world, 'state, Wrld, SB, ParamName, E>(
            __entity: &'world mut E,
            __state: &'state mut Self::State,
            __world: &'world mut Wrld,
        ) -> Option<Self::Item<'world, 'state, Wrld>>
        where
            Wrld: World,
            SB: SystemParamNameMapper,
            E: EntityFetch,
            ParamName: 'static,
        {
            let __world = __world as *mut Wrld;
            let __entity = __entity as *mut E;
            unsafe {
                #(#get)*
                Some(#struct_name {
                    #(#idents),*
                })
            }
        }
    };

    let get = fields.iter().enumerate().map(|(_i, x)| {
        let ident = &x.ident;
        let ty = &x.ty;
        quote! {
            let #ident = <#ty>::get_param::<Wrld, SB, ParamName>(
                &mut __state.#ident,
                &mut *__world
            );
        }
    });
    let get_param = quote! {
        #[inline(always)]
        fn get_param<'world, 'state, Wrld: World, SB: SystemParamNameMapper, ParamName>(
            __state: &'state mut Self::State,
            __world: &'world mut Wrld,
        ) -> Self::Item<'world, 'state, Wrld>
        where
            ParamName: 'static,
        {
            let __world = __world as *mut Wrld;
            unsafe {
                #(#get)*
                #struct_name {
                    #(#idents),*
                }
            }
        }
    };
    let builds = fields.iter().map(|x| {
        let ty = &x.ty;
        quote! {
            let mut builder =
                <#ty as SystemParam>::build::<_, SB, ParamName>(builder);
        }
    });
    let mut renamed_generics = input.generics.clone();
    rename_generics(&mut renamed_generics.params);
    let mut state_generics = Generics {
        lt_token: input.generics.lt_token,
        params: Punctuated::new(),
        gt_token: input.generics.gt_token,
        where_clause: None,
    };
    for x in &input.generics.params {
        if !matches!(x, GenericParam::Lifetime(_)) {
            state_generics.params.push(x.clone());
        }
    }
    if let Some(clause) = &input.generics.where_clause {
        let mut predicates: Punctuated<WherePredicate, Token![,]> = Default::default();
        for x in &clause.predicates {
            if !matches!(x, WherePredicate::Lifetime(_)) {
                predicates.push(x.clone());
            }
        }
        state_generics.where_clause = Some(WhereClause {
            where_token: Default::default(),
            predicates,
        });
    }
    let state_fields = fields.iter().zip(&states).map(|(field, state)| {
        let ident = &field.ident;
        quote! {
            #ident: #state
        }
    });
    let state_inits = fields.iter().zip(&states).map(|(field, state)| {
        let ident = &field.ident;
        quote! {
            let #ident = #state::init::<W, SB, ParamName, I>(inputs, &mut *world);
        }
    });
    let vis = &input.vis;
    let state_struct = format_ident!("{}State", struct_name);
    let (impl_generics, state_ty_generics, where_clause) = state_generics.split_for_impl();
    let state = quote! {
        #vis struct #state_struct #state_generics {
            #(#state_fields),*
        }
        impl #impl_generics SystemParamState for #state_struct #state_ty_generics #where_clause {
            #[inline(always)]
            fn init<W: crate::World, SB: crate::SystemParamNameMapper, ParamName: 'static, I: Input>(
                inputs: &mut I,
                world: &mut W,
            ) -> Self {
                unsafe {
                    let world = world as *mut W;
                    #(#state_inits)*
                    Self {
                        #(#idents),*
                    }
                }
            }
        }
    };
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let (_, renamed_ty_generics, _) = renamed_generics.split_for_impl();
    let out = quote! {
        #state
        #input
        impl #impl_generics SystemParam for #struct_name #ty_generics #where_clause {
            // cast lifetimes
            type Item<'world, 'state, Wrld: World> = #struct_name #renamed_ty_generics;
            type State = #state_struct #state_ty_generics;

            type Build<B: EcsBuilder, SB: SystemParamNameMapper + 'static, ParamName: 'static> =
                impl EcsBuilder;

            const IS_QUERY: bool = #(#is_query ||)* false;

            #get_param_for_entity

            #get_param

            #[inline(always)]
            fn build<B: EcsBuilder, SB: SystemParamNameMapper + 'static, ParamName: 'static>(
                mut builder: B,
            ) -> Self::Build<B, SB, ParamName> {
                #(#builds)*
                builder
            }
        }
    };

    // eprintln!("{}", out.to_string());
    out.into()
}
