use proc_macro::TokenStream;

use quote::quote;
use syn::{
    parse_macro_input, Expr, ExprRange, Lit, RangeLimits,
};

fn get_start(start: &Option<Box<Expr>>) -> usize {
    match start {
        Some(x) => match x.as_ref() {
            Expr::Lit(x) => match &x.lit {
                Lit::Int(x) => x.base10_parse().unwrap(),
                _ => panic!("range start must be an integer literal"),
            },
            _ => panic!("range start must be an integer literal"),
        },
        None => 0,
    }
}

fn get_end(end: &Option<Box<Expr>>, limits: &RangeLimits) -> usize {
    let end = match end {
        Some(x) => match x.as_ref() {
            Expr::Lit(x) => match &x.lit {
                Lit::Int(x) => x.base10_parse().unwrap(),
                _ => panic!("range end must be an integer literal"),
            },
            _ => panic!("range end must be an integer literal"),
        },
        None => panic!("range end must be an integer literal"),
    };
    match limits {
        RangeLimits::HalfOpen(_) => end,
        RangeLimits::Closed(_) => end + 1,
    }
}

pub fn nest(input: TokenStream) -> TokenStream {
    let inception = macros_util::crate_name("inception");
    let input = parse_macro_input!(input as ExprRange);
    // get start and end of range
    let start = get_start(&input.from);
    let end = get_end(&input.to, &input.limits);
    let nest = quote! {#inception::StackedNest};
    let push = (start..end).map(|x| {
        quote! {
            .push(#x)
        }
    });
    let out = quote! {
        {
            use #inception::Nestable;
            #nest
            #(#push)*
        }
    };
    out.into()
}
