use proc_macro::TokenStream;
use proc_macro_error::proc_macro_error;
use quote::{format_ident, quote};
use syn::{parse_macro_input, DeriveInput, LitInt};

mod nest_macro;
mod schedule_macro;
mod system_macro;
mod system_param_macro;
mod entities_builder_macro;

#[proc_macro_error]
#[proc_macro_attribute]
/// A macro for defining systems. Generates a module with a struct called `System` and other structs
/// with their name equal to the name of function argument. Automatically adds generic parameters
/// to the system and their parameter types.
/// If the system has a parameter with `In<T>` or `PhantomIn<T>` type, then constructor will
/// require those arguments to be passed to it.
pub fn system(attributes: TokenStream, item: TokenStream) -> TokenStream {
    system_macro::system(attributes, item)
}

#[proc_macro_attribute]
pub fn system_param(attributes: TokenStream, item: TokenStream) -> TokenStream {
    system_param_macro::system_param(attributes, item)
}

#[proc_macro]
/// Converts range bounds to an array that is nested. Rustc cannot always optimize for loops away.
/// Recursion is used instead.
/// # Example
/// ```rust
/// use inception::nest;
/// let arr = nest!(0..2);
/// assert_eq!(arr, StackedNest.push(0).push(1));
pub fn nest(input: TokenStream) -> TokenStream {
    nest_macro::nest(input)
}

#[proc_macro]
pub fn schedule(input: TokenStream) -> TokenStream {
    schedule_macro::schedule(input)
}

#[proc_macro]
pub fn entities_builder(input: TokenStream) -> TokenStream {
    entities_builder_macro::entities_builder(input)
}

#[proc_macro_derive(SystemParamPlugin)]
pub fn system_param_plugin_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let out = quote! {
        impl #impl_generics inception::SystemParamPlugin for #struct_name #ty_generics #where_clause {
            type Build<B: inception::EcsBuilder> = B;

            fn build<B: inception::EcsBuilder>(builder: B) -> Self::Build<B> {
                builder
            }
        }
    };

    // eprintln!("{}", out.to_string());
    out.into()
}

#[proc_macro]
pub fn impl_system_param_plugin(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as LitInt);
    let num = input.base10_parse::<usize>().unwrap();
    let impls = (0..num).map(|x| {
        let mut build_ty = quote! {B};
        for y in 0..x {
            let name = format_ident!("P{}", y);
            build_ty = quote! {
                <#name as SystemParamPlugin<#build_ty>>::Build
            };
        }
        let params_bounds = (0..x).map(|y| {
            let name = format_ident!("P{}", y);
            let mut build_ty = quote! {B};
            for z in 0..y {
                let name = format_ident!("P{}", z);
                build_ty = quote! {
                    <#name as SystemParamPlugin<#build_ty>>::Build
                };
            }
            quote! {
                #name: SystemParamPlugin<#build_ty>
            }
        });
        let params = (0..x).map(|y| {
            format_ident!("P{}", y)
        }).collect::<Vec<_>>();
        let build = (0..x).map(|x| {
            let name = format_ident!("P{}", x);
            quote! {
                let mut context = context.clone();
                context.set_param_id::<SB>(param_id);
                let config = builder.take_config::<#name::PluginConfig>(&context);
                let mut builder = #name::build::<SB, <SB as Mapper<#x>>::Name>(config, &context, builder);
                param_id += 1;
            }
        });

        quote! {
            impl<B: EcsBuilder #(,#params)*> SystemParamPlugin<B> for (#(#params,)*)
            where
                #(#params_bounds),*
            {
                type Build = #build_ty;
                type PluginConfig = ();

                #[inline(always)]
                #[allow(unused_variables, unused_mut)]
                fn build<SB: SystemParamNameMapper + 'static, ParamName: 'static>(
                    _config: Self::PluginConfig,
                    context: &SystemParamContext,
                    builder: B,
                ) -> Self::Build {
                    #![allow(unused_assignments)]
                    let mut param_id = 0;
                    let mut builder = builder;
                    #(#build)*
                    builder
                }
            }
        }
    });

    let out = quote! {
        #(#impls)*
    };
    // eprintln!("{}", out.to_string());
    out.into()
}
