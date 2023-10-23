use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{format_ident};
use syn::{
    parse_quote,
    punctuated::Punctuated,
    token::{Colon, Const, Gt, Lt},
    AngleBracketedGenericArguments, ConstParam, GenericArgument, GenericParam, Lifetime,
    PathArguments, Token, TypeParam, TypeParamBound, *,
};

pub fn is_lifetime_w(arg: &GenericArgument) -> bool {
    if let syn::GenericArgument::Lifetime(x) = arg {
        x.ident == "'w"
    } else {
        false
    }
}

pub fn is_lifetime_s(arg: &GenericArgument) -> bool {
    if let syn::GenericArgument::Lifetime(x) = arg {
        x.ident == "'s"
    } else {
        false
    }
}

// pub fn is_w(arg: &GenericArgument) -> bool {
//     if let syn::GenericArgument::Type(syn::Type::Path(path)) = arg {
//         path.path.segments.last().unwrap().ident == "W"
//     } else {
//         false
//     }
// }

pub fn is_n(arg: &GenericArgument) -> bool {
    if let syn::GenericArgument::Type(syn::Type::Path(path)) = arg {
        path.path.segments.last().unwrap().ident == "N"
    } else {
        false
    }
}

pub fn contains_lifetime_w(args: &AngleBracketedGenericArguments) -> bool {
    args.args.iter().next().map(is_lifetime_w).unwrap_or(false)
}

pub fn contains_lifetime_s(args: &AngleBracketedGenericArguments) -> bool {
    args.args.iter().next().map(is_lifetime_s).unwrap_or(false)
}

// pub fn contains_w(args: &AngleBracketedGenericArguments) -> bool {
//     args.args.last().map(is_w).unwrap_or(false)
// }

pub fn contains_n(args: &AngleBracketedGenericArguments) -> bool {
    args.args.last().map(is_n).unwrap_or(false)
}

pub fn expand_generic_arguments_with_static_lifetime(args: &mut PathArguments) {
    match args {
        PathArguments::AngleBracketed(args) => {
            if !contains_lifetime_w(args) {
                args.args.insert(0, parse_quote!('static))
            }
            if !contains_lifetime_s(args) {
                args.args.insert(1, parse_quote!('static))
            }
            if !contains_n(args) {
                args.args.push(parse_quote!(N));
            }
        },
        PathArguments::None => {
            *args = PathArguments::AngleBracketed(parse_quote!(<'static, 'static, N>));
        },
        _ => {},
    }
}

pub fn expand_generic_arguments(args: &mut PathArguments) {
    match args {
        PathArguments::AngleBracketed(args) => {
            if !contains_lifetime_w(args) {
                args.args.insert(0, parse_quote!('w))
            }
            if !contains_lifetime_s(args) {
                args.args.insert(1, parse_quote!('w))
            }
            if !contains_n(args) {
                args.args.push(parse_quote!(N));
            }
        },
        PathArguments::None => {
            *args = PathArguments::AngleBracketed(parse_quote!(<'w, 's, N>));
        },
        _ => {},
    }
}

pub fn expand_generics(
    params: &mut Punctuated<GenericParam, Token![,]>,
    inception: &TokenStream2,
    include_world: bool,
) {
    let mut contains_lifetime_w = false;
    let mut contains_lifetime_s = false;
    let mut contains_w = false;
    let mut contains_n = false;
    for param in &mut *params {
        match param {
            GenericParam::Lifetime(x) => {
                if x.lifetime.ident == "'w" {
                    contains_lifetime_w = true;
                }
                if x.lifetime.ident == "'s" {
                    contains_lifetime_s = true;
                }
            },
            GenericParam::Type(x) => {
                if x.ident == "W" {
                    contains_w = true;
                }
            },
            GenericParam::Const(x) => {
                if x.ident == "N" {
                    contains_n = true;
                }
            },
        }
    }
    if !contains_lifetime_w {
        params.insert(
            0,
            GenericParam::Lifetime(syn::LifetimeDef {
                attrs: vec![],
                lifetime: Lifetime::new("'w", Span::call_site()),
                colon_token: None,
                bounds: Punctuated::new(),
            }),
        );
    }
    if !contains_lifetime_s {
        params.insert(
            1,
            GenericParam::Lifetime(syn::LifetimeDef {
                attrs: vec![],
                lifetime: Lifetime::new("'s", Span::call_site()),
                colon_token: None,
                bounds: Punctuated::new(),
            }),
        );
    }
    if include_world && !contains_w {
        let mut bounds = Punctuated::new();
        bounds.push(TypeParamBound::Trait(syn::TraitBound {
            paren_token: None,
            modifier: syn::TraitBoundModifier::None,
            lifetimes: None,
            path: parse_quote!(#inception::World),
        }));
        params.push(GenericParam::Type(TypeParam {
            attrs: vec![],
            ident: format_ident!("W"),
            colon_token: None,
            bounds,
            eq_token: None,
            default: None,
        }));
    }

    if !contains_n {
        params.push(GenericParam::Const(ConstParam {
            attrs: Vec::new(),
            const_token: Const::default(),
            ident: format_ident!("N"),
            colon_token: Colon::default(),
            ty: parse_quote!(usize),
            eq_token: None,
            default: None,
        }));
    }
}

// adds 'w, 's to every parameter type, it also adds W at the end of it and 'w to every reference
pub fn expand_inputs_with_names<'a>(
    inputs: impl IntoIterator<Item = &'a mut Type>,
    world_lifetime: &str,
    state_lifetime: &str,
    world_name: &str,
    n_name: &str,
) {
    fn process_type(
        ty: &mut Type,
        world_lifetime: &str,
        state_lifetime: &str,
        world_name: &str,
        n_name: &str,
    ) {
        let process_generics =
            |contains_lifetime_w: bool,
             contains_lifetime_s: bool,
             contains_w: bool,
             contains_n: bool,
             args: &mut Punctuated<GenericArgument, Token![,]>| {
                if !contains_lifetime_w {
                    args.insert(
                        0,
                        GenericArgument::Lifetime(Lifetime::new(world_lifetime, Span::call_site())),
                    );
                }
                if !contains_lifetime_s {
                    args.insert(
                        1,
                        GenericArgument::Lifetime(Lifetime::new(state_lifetime, Span::call_site())),
                    );
                }
                let mut push_type = |name: &str| {
                    let mut segments = Punctuated::new();
                    segments.push(PathSegment {
                        ident: format_ident!("{}", name),
                        arguments: PathArguments::default(),
                    });
                    args.push(GenericArgument::Type(Type::Path(TypePath {
                        qself: None,
                        path: syn::Path {
                            leading_colon: None,
                            segments,
                        },
                    })));
                };
                if !contains_w {
                    push_type(world_name);
                }
                if !contains_n {
                    push_type(n_name);
                }
                fn add_lifetimes(kind: &mut Type, world_lifetime: &str) {
                    match kind {
                        Type::Reference(x) => {
                            if x.lifetime.is_none() {
                                x.lifetime = Some(Lifetime::new(world_lifetime, Span::call_site()));
                            }
                        },
                        Type::Tuple(x) => {
                            for x in &mut x.elems {
                                add_lifetimes(x, world_lifetime);
                            }
                        },
                        _ => {},
                    }
                }
                // add 'w to all references
                for arg in args {
                    if let GenericArgument::Type(x) = arg {
                        add_lifetimes(x, world_lifetime);
                    }
                }
            };
        match ty {
            syn::Type::Tuple(x) => {
                for x in &mut x.elems {
                    process_type(x, world_lifetime, state_lifetime, world_name, n_name);
                }
            },
            syn::Type::Path(path) => {
                let segment = path.path.segments.last_mut().unwrap();
                // W is only added to `Query` other parameters don't need it. It bloats
                // compile times a lot to the point where it is unusable. It would take 50
                // seconds for incremental compilation and 12 GiB of RAM.
                let is_query = segment.ident == "Query";
                match &mut segment.arguments {
                    PathArguments::None => {
                        let mut args = Punctuated::new();
                        process_generics(false, false, !is_query, false, &mut args);
                        segment.arguments =
                            PathArguments::AngleBracketed(AngleBracketedGenericArguments {
                                colon2_token: None,
                                lt_token: Lt::default(),
                                args,
                                gt_token: Gt::default(),
                            });
                    },
                    PathArguments::AngleBracketed(param_generics) => {
                        process_generics(
                            contains_lifetime_w(param_generics),
                            contains_lifetime_s(param_generics),
                            !is_query,
                            contains_n(param_generics),
                            &mut param_generics.args,
                        );
                    },
                    PathArguments::Parenthesized(_) => {},
                }
            },
            syn::Type::Reference(reference) => {
                reference.lifetime = Some(Lifetime::new(world_lifetime, Span::call_site()));
            },
            _ => {},
        }
    }
    for input in inputs {
        process_type(input, world_lifetime, state_lifetime, world_name, n_name);
    }
}
pub fn expand_inputs<'a>(inputs: impl IntoIterator<Item = &'a mut Type>) {
    expand_inputs_with_names(inputs, "'w", "'s", "W", "N");
}
