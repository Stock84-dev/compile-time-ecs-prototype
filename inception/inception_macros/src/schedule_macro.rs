use core::str::FromStr;

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{format_ident, quote};
use syn::{parse::Parse, punctuated::Punctuated, *};

struct Schedule {
    items: Punctuated<ScheduleItem, Token![,]>,
}

struct StageItem {
    stage: syn::Ident,
    alias: syn::Ident,
}

impl Parse for StageItem {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let stage = input.parse()?;
        let _ = <Token![as]>::parse(input)?;
        let alias = input.parse()?;
        Ok(StageItem { stage, alias })
    }
}

enum ScheduleItem {
    ScheduleName(syn::Ident),
    Stage(StageItem),
    Looped(Schedule),
}

impl Parse for Schedule {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let items = input.parse_terminated(ScheduleItem::parse)?;
        Ok(Schedule { items })
    }
}

impl Parse for ScheduleItem {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        if input.peek(Token![loop]) {
            let content;
            let _ = <Token![loop]>::parse(input)?;
            let _ = braced!(content in input);
            let schedule = content.parse()?;
            Ok(ScheduleItem::Looped(schedule))
        } else if input.peek(Token![struct]) {
            let _ = <Token![struct]>::parse(input)?;
            let ident = input.parse()?;
            Ok(ScheduleItem::ScheduleName(ident))
        } else {
            let item = input.parse()?;
            Ok(ScheduleItem::Stage(item))
        }
    }
}

struct Stage {
    id: usize,
    stage: syn::Ident,
    generic_type: syn::Ident,
    alias: syn::Ident,
}

fn get_stages(schedule: &Schedule, schedule_name: &mut syn::Ident, stages: &mut Vec<Stage>) {
    for item in &schedule.items {
        match item {
            ScheduleItem::Stage(x) => {
                let name = x.stage.to_string();
                let id = usize::from_str(&name[5..]).expect(
                    "The first item of a stage must start with concrete type like `Stage0`.",
                );
                stages.push(Stage {
                    id,
                    stage: x.stage.clone(),
                    generic_type: format_ident!("S{}", id),
                    alias: x.alias.clone(),
                });
            },
            ScheduleItem::Looped(x) => get_stages(x, schedule_name, stages),
            ScheduleItem::ScheduleName(name) => *schedule_name = name.clone(),
        }
    }
}

fn quote_run(
    inception: &proc_macro2::TokenStream,
    item: &ScheduleItem,
    token_stream: proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    match item {
        ScheduleItem::ScheduleName(_) => token_stream,
        ScheduleItem::Stage(s) => {
            let name = &s.stage;
            quote! {
                #token_stream
                self.#name.run(world);
            }
        },
        ScheduleItem::Looped(looped) => {
            let mut acc = proc_macro2::TokenStream::default();
            for item in &looped.items {
                acc = quote_run(inception, item, acc);
            }
            quote! {
                #token_stream
                while !world.resource_mut::<#inception::resources::Break>().0 {
                    #acc
                }
                world.resource_mut::<#inception::resources::Break>().0 = false;
            }
        },
    }
}

pub fn schedule(input: TokenStream) -> TokenStream {
    let inception = macros_util::crate_name("inception");
    let mut docs_string = input.to_string();
    let input = parse_macro_input!(input as Schedule);
    let mut i = 0;
    docs_string.retain(|x| x != '\n');
    while i < docs_string.len() {
        let c = docs_string.chars().nth(i).unwrap();
        if c == ',' || c == '{' {
            docs_string.insert(i + 1, '\n');
        }
        i += 1;
    }
    docs_string.insert_str(0, "```ignore\n inception::schedule! {\n");
    docs_string.push_str("}\n```");
    let docs = quote! {
        #[doc = #docs_string]
    };
    let mut stages = Vec::new();
    let mut schedule_name = syn::Ident::new("Schedule", Span::call_site());
    get_stages(&input, &mut schedule_name, &mut stages);
    let schedule_builder = format_ident!("{}Builder", schedule_name);
    let generic_stages = stages.iter().map(|x| &x.generic_type).collect::<Vec<_>>();
    let mut run = proc_macro2::TokenStream::default();
    for item in &input.items {
        run = quote_run(&inception, item, run);
    }
    let default_stages_ty = stages
        .iter()
        .map(|_| quote! {#inception::StackedNest})
        .collect::<Vec<_>>();
    let default_stages_field = stages.iter().map(|x| {
        let name = &x.stage;
        quote! {#name: #inception::StackedNest}
    });
    let struct_fields = stages
        .iter()
        .map(|x| {
            let name = &x.stage;
            let generic_type = &x.generic_type;
            quote! {
                #name: #generic_type
            }
        })
        .collect::<Vec<_>>();
    let build_stages_fields = stages.iter().map(|x| {
        let name = &x.stage;
        quote! {
            #name: self.#name.build_stage(world),
        }
    });

    let blanked_impls = (0..32)
        .filter(|x| !stages.iter().any(|s| s.id == *x))
        .map(|x| {
            let f = format_ident!("add_system_to_stage{}", x);
            let ret = format_ident!("AddSystemToStage{}", x);
            let stage_ident = format_ident!("Stage{}", x);
            quote! {
                type #ret<System: #inception::SystemBuilder<'static, 'static> + 'static> = Self;

                fn #f<System>(
                    self,
                    system: System,
                ) -> Self::#ret<System>
                where
                    System: #inception::SystemBuilder<'static, 'static> + 'static
                {
                    panic!(
                        "Schedule `{}` does not have a stage `{}`.",
                        stringify!(#schedule_name),
                        stringify!(#stage_ident)
                    );
                }
            }
        });

    let add_system_to_stage_impls = stages.iter().map(|x| {
        let stage_ident = &x.stage;
        let f = format_ident!("add_system_to_stage{}", x.id);
        let ret = format_ident!("AddSystemToStage{}", x.id);
        let pass_fields = stages.iter().filter(|stage| stage.id != x.id).map(|stage| {
            let name = &stage.stage;
            quote! {
                #name: self.#name
            }
        });
        let output_generics = stages.iter().map(|stage| {
            let ty = &stage.generic_type;
            if stage.id == x.id {
                quote! {
                    #inception::Nested<#ty, #inception::AddSystemToStageCommand<System>>
                }
            } else {
                quote! {
                    #ty
                }
            }
        });
        quote! {
            type #ret<System: #inception::SystemBuilder<'static, 'static> + 'static> =
                #schedule_builder<#(#output_generics),*>;

            #[inline(always)]
            fn #f<System>(
                self,
                system: System,
            ) -> Self::#ret<System>
            where
                System: #inception::SystemBuilder<'static, 'static> + 'static
            {
                #schedule_builder {
                    #(#pass_fields,)*
                    #stage_ident: #inception::Nested {
                        item: #inception::AddSystemToStageCommand {
                            builder: system,
                        },
                        inner: self.#stage_ident,
                    }
                }
            }
        }
    });

    let assert_ty_eq = stages.iter().map(|x| {
        let stage = &x.stage;
        let alias = &x.alias;
        quote! {
            inception::static_assertions::assert_type_eq_all!(#stage, #alias);
        }
    });

    let out = quote! {
        #(#assert_ty_eq)*
        #[allow(non_snake_case)]
        pub struct #schedule_builder<#(#generic_stages),*> {
            #(#struct_fields),*
        }

        #[allow(non_snake_case)]
        #docs
        pub struct #schedule_name<#(#generic_stages),*> {
            #(#struct_fields),*
        }

        impl #schedule_name<#(#default_stages_ty),*> {
            #[inline(always)]
            pub fn builder() -> #schedule_builder<#(#default_stages_ty),*> {
                #schedule_builder {
                    #(#default_stages_field),*
                }
            }
        }
        impl<#(#generic_stages: #inception::StageBuilder),*> #inception::ScheduleBuilderTrait for
            #schedule_builder<#(#generic_stages),*> {
            #(#add_system_to_stage_impls)*
            #(#blanked_impls)*
        }

        impl<#(#generic_stages: #inception::StageBuilder),*> #inception::StageBuilder for
            #schedule_builder<#(#generic_stages),*> {
            type BuildStage<W: #inception::World, const N_EVENTS: usize> = #schedule_name<
                #(<#generic_stages as #inception::StageBuilder>::BuildStage<W, N_EVENTS>),*
            >;
            #[inline(always)]
            fn build_stage<W: #inception::World, const N_EVENTS: usize>(
                self,
                world: &mut W,
            ) -> Self::BuildStage<W, N_EVENTS> {
                #schedule_name {
                    #(#build_stages_fields)*
                }
            }
        }

        impl<W: #inception::World, #(#generic_stages: #inception::Stage<W>),*> Stage<W> for
            #schedule_name<#(#generic_stages),*> {
            #[inline(always)]
            fn run(&mut self, world: &mut W) {
                #run
            }
        }
    };
    // eprintln!("{}", out.to_string());
    out.into()
}
