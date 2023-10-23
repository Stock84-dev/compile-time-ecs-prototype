use core::str::FromStr;

use derive_syn_parse::Parse;
use proc_macro::TokenStream;
use proc_macro2::Ident;
use quote::{format_ident, quote};
use syn::{parse_macro_input, LitInt};

#[derive(Parse)]
struct EntitiesBuilderInput {
    struct_ident: Ident,
    _comma: syn::Token![,],
    max_impls: LitInt,
}

pub fn entities_builder(input: TokenStream) -> TokenStream {
    let inception = macros_util::crate_name("inception");
    let input = parse_macro_input!(input as EntitiesBuilderInput);
    let struct_ident = input.struct_ident;
    let n_max_impls = input.max_impls.base10_parse::<usize>().unwrap();
    let name = struct_ident.to_string();
    // remove characters before first number
    let name = name.trim_start_matches(char::is_alphabetic);
    let n_impls = usize::from_str(name).expect("Failed to parse number from param name");
    let generics = (0..n_impls)
        .map(|x| format_ident!("E{}", x))
        .collect::<Vec<_>>();
    let type_ident = format_ident!("DefaultEntitiesBuilder{}", n_impls);
    let bounded_generics = (0..n_impls)
        .map(|x| {
            let ty = format_ident!("E{}", x);
            quote! {
                #ty: EntityComponent + 'static
            }
        })
        .collect::<Vec<_>>();
    let struct_fields = (0..n_impls).map(|x| {
        let field = format_ident!("e{}", x);
        let ty = format_ident!("E{}", x);
        quote! {
            #field: EntityData<#ty>
        }
    });
    let constructors = (0..n_impls).map(|x| {
        let field = format_ident!("e{}", x);
        quote! {
            #field: EntityData::new(Entity(#x))
        }
    });
    let extend_constructors = (0..n_impls).map(|x| {
        let field = format_ident!("e{}", x);
        quote! {
            #field: self.#field.add(component.clone())
        }
    });
    let extend_generics = (0..n_impls).map(|x| {
        let ty = format_ident!("E{}", x);
        quote! {
            Nested<#ty, C>
        }
    });
    let stacked_nests_ty = (0..n_impls)
        .map(|_| format_ident!("StackedNest"))
        .collect::<Vec<_>>();
    let insert_fns = (0..n_max_impls).map(|x| {
        let insert_ty = format_ident!("Add{}", x);
        let insert_fn = format_ident!("add{}", x);
        if x >= n_impls {
            return quote! {
                type #insert_ty<C: 'static> = Self;
                #[inline(always)]
                fn #insert_fn<C: 'static>(self, component: C) -> Self::#insert_ty<C> {
                    panic!("Failed to insert a component of an entity at {}, the world only has {} entities.",
                        #x, #n_impls);
                    self
                }
            };
        }
        let other_fields = (0..n_impls).map(|y| {
            let field = format_ident!("e{}", y);
            if y == x {
                quote! {
                    #field: self.#field.add(component)
                }
            } else {
                quote! {
                    #field: self.#field
                }
            }
        });
        let generics = (0..n_impls).map(|y| {
            let ty = format_ident!("E{}", y);
            if y == x {
                quote! {
                    Nested<#ty, C>
                }
            } else {
                quote! {
                    #ty
                }
            }
        });
        quote! {
            type #insert_ty<C: 'static> = #struct_ident<#(#generics),*>;
            #[inline(always)]
            fn #insert_fn<C: 'static>(self, component: C) -> Self::#insert_ty<C> {
                #struct_ident {
                    #(#other_fields,)*
                }
            }
        }
    });
    let query_impl = (0..n_impls).map(|x| {
        let field = format_ident!("e{}", x);
        quote! {
            self.#field.query::<F, Q>(&mut f);
        }
    });
    let query_entity_impl = (0..n_impls).map(|x| {
        let field = format_ident!("e{}", x);
        quote! {
            self.#field.query_entity::<F, Q>(entity, &mut f);
        }
    });
    let for_each_impl = (0..n_impls).map(|x| {
        let field = format_ident!("e{}", x);
        quote! {
            f.call_mut(&mut self.#field);
        }
    });
    let get_components_impl = (0..n_impls).map(|x| {
        let field = format_ident!("e{}", x);
        quote! {
            if entity == self.#field.entity {
                return Q::get_component(&self.#field.components);
            }
        }
    });
    let get_component_impl = (0..n_impls).map(|x| {
        let field = format_ident!("e{}", x);
        quote! {
            if entity == self.#field.entity {
                return self.#field.get_component();
            }
        }
    });
    let get_component_mut_impl = (0..n_impls).map(|x| {
        let field = format_ident!("e{}", x);
        quote! {
            if entity == self.#field.entity {
                return self.#field.get_component_mut();
            }
        }
    });

    let out = quote! {
        pub struct #struct_ident<#(#generics),*> {
            #(#struct_fields),*
        }

        pub type #type_ident = #struct_ident<#(#stacked_nests_ty),*>;

        impl #struct_ident<#(#stacked_nests_ty),*> {
            #[inline(always)]
            pub fn new() -> Self {
                Self {
                    #(#constructors),*
                }
            }
        }

        impl<#(#bounded_generics),*> EntitiesBuilder for #struct_ident<#(#generics),*> {
            type Add<C: 'static, ER: EntityRelay> = ER::Add<Self, C>;
            type ExtendEntities<C: Clone + 'static> = #struct_ident<#(#extend_generics),*>;
            #(#insert_fns)*
            #[inline(always)]
            fn extend_entities<C: Clone + 'static>(self, component: C) -> Self::ExtendEntities<C> {
                #struct_ident {
                    #(#extend_constructors),*
                }
            }
            #[inline(always)]
            fn add<C: 'static, ER: EntityRelay>(self, component: C, entity: ER) -> Self::Add<C, ER> {
                ER::add(self, component)
            }
        }

        impl<#(#bounded_generics),*> Entities for #struct_ident<#(#generics),*> {
            #[inline(always)]
            fn query<'w, F, Q>(&'w mut self, mut f: F)
            where
                F: FnMut(<Q as WorldQuery>::Item<'w>),
                Q: WorldQuery,
            {
                unsafe {
                    #(#query_impl)*
                }
            }

            #[inline(always)]
            fn query_entity<'w, F, Q>(&'w mut self, entity: Entity, mut f: F)
            where
                F: FnMut(<Q as WorldQuery>::Item<'w>),
                Q: WorldQuery,
            {
                #(#query_entity_impl)*
            }

            #[inline(always)]
            fn for_each<F>(&mut self, mut f: F)
            where
                F: EntityFnMut,
            {
                #(#for_each_impl)*
            }

            #[inline(always)]
            fn get_components<'w, Q: WorldQuery>(&'w mut self, entity: Entity) -> Option<Q::Item<'w>> {
                unsafe {
                    #(#get_components_impl)*
                    None
                }
            }

            #[inline(always)]
            fn components<'w, Q: WorldQuery>(&'w mut self, entity: Entity) -> Q::Item<'w> {
                expect_components::<Q, _>(entity, self.get_components::<Q>(entity))
            }

            #[inline(always)]
            fn get_component<T: 'static>(&self, entity: Entity) -> Option<&T> {
                #(#get_component_impl)*
                None
            }

            #[inline(always)]
            fn get_component_mut<T: 'static>(&mut self, entity: Entity) -> Option<&mut T> {
                #(#get_component_mut_impl)*
                None
            }
        }
    };

    // eprintln!("{}", out.to_string());
    out.into()
}
