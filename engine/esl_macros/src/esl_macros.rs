use convert_base::Convert;
use darling::FromMeta;
use ergnomics::some_loop;
use inception_macros_core::*;
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse_macro_input, parse_quote, AttributeArgs, DeriveInput, FieldsNamed, FieldsUnnamed,
    GenericArgument, ItemFn, ItemImpl, ItemStruct, LitInt, PathArguments, Type,
};
const N_ORDERS: u64 = 9;

/// Expands some spicific parameters related to `esl`. Then applies `#[system]`. In the future this
/// will extract signal components of a system and combine them from other strategies to create a
/// new strategy.
#[proc_macro_attribute]
pub fn strategy(_attributes: TokenStream, item: TokenStream) -> TokenStream {
    let esl = macros_util::crate_name("esl");
    let mut input = parse_macro_input!(item as ItemFn);
    input.attrs.push(parse_quote!(#[system]));
    for arg in &mut input.sig.inputs {
        match arg {
            syn::FnArg::Receiver(_) => {},
            syn::FnArg::Typed(x) => {
                // Expand Prev<T> into Prev<T<'w, 's, N>>
                match &mut *x.ty {
                    Type::Macro(_) => {
                        x.ty = parse_quote!(#esl::HyperParam);
                    },
                    Type::Path(path) => {
                        let segment = some_loop!(path.path.segments.last_mut());
                        // Couldn't figure out a way on how to apply that Param in Prev<Param>
                        // contains any lifetime. Plugin requires that it is static or at least
                        // that it outlives 'w, 's. If I apply any
                        // lifetime to it, then the `system` doesn't fulfill the lifetime
                        // requirements. Because it uses specific lifetime when casting function
                        // item to function pointer. Introducing another lifetime might solve it.
                        // But this also works.
                        if segment.ident != "Prev" {
                            continue;
                        }
                        if let PathArguments::AngleBracketed(args) = &mut segment.arguments {
                            if let Some(GenericArgument::Type(Type::Path(x))) = args
                                .args
                                .iter_mut()
                                .find(|x| !(is_n(x) || is_lifetime_w(x) || is_lifetime_s(x)))
                            {
                                let segment = some_loop!(x.path.segments.last_mut());
                                expand_generic_arguments_with_static_lifetime(
                                    &mut segment.arguments,
                                );
                            }
                        }
                    },
                    _ => (),
                }
            },
        }
    }
    let out = quote! {
        #input
    };
    // eprintln!("{}", out);
    out.into()
}

#[proc_macro_attribute]
pub fn metric(_attr: TokenStream, input: TokenStream) -> TokenStream {
    let _esl = macros_util::crate_name("esl");
    let metric = parse_macro_input!(input as DeriveInput);
    let ident = &metric.ident;
    let value_ty = match &metric.data {
        syn::Data::Struct(x) => match &x.fields {
            syn::Fields::Named(x) => x.named.first(),
            syn::Fields::Unnamed(x) => x.unnamed.first(),
            syn::Fields::Unit => None,
        },
        _ => None,
    }
    .map(|x| &x.ty);
    let (impl_generics, ty_generics, where_clause) = metric.generics.split_for_impl();
    let extra_impl = match value_ty {
        Some(value_ty) => {
            quote! {
                impl #impl_generics Value for #ident #ty_generics #where_clause {
                    type Value = #value_ty;

                    #[inline(always)]
                    fn get(&self) -> Self::Value {
                        self.0
                    }
                }

                impl #impl_generics core::ops::Deref for #ident #ty_generics #where_clause {
                    type Target = #value_ty;

                    #[inline(always)]
                    fn deref(&self) -> &Self::Target {
                        &self.0
                    }
                }

                impl #impl_generics core::ops::DerefMut for #ident #ty_generics #where_clause {
                    #[inline(always)]
                    fn deref_mut(&mut self) -> &mut Self::Target {
                        &mut self.0
                    }
                }
            }
        },
        None => quote! {},
    };
    let out = quote! {
        #[derive(Clone, Copy, PartialEq)]
        #[repr(C)]
        #metric
        #extra_impl
    };
    // eprintln!("{}", out);
    out.into()
}

#[derive(FromMeta)]
struct ResourceValueAttr {
    #[darling(default)]
    skip_value: bool,
}

#[proc_macro_attribute]
pub fn resource_value(attr: TokenStream, input: TokenStream) -> TokenStream {
    let _esl = macros_util::crate_name("esl");
    let mut component = parse_macro_input!(input as DeriveInput);
    let attr_args = parse_macro_input!(attr as AttributeArgs);
    let args = match ResourceValueAttr::from_list(&attr_args) {
        Ok(v) => v,
        Err(e) => {
            return TokenStream::from(e.write_errors());
        },
    };
    let mut metric = format_ident!("{}Resource", component.ident);
    core::mem::swap(&mut component.ident, &mut metric);
    let vis = &component.vis;
    let (impl_generics, ty_generics, where_clause) = component.generics.split_for_impl();
    let component_ty = &component.ident;
    let mut n_fields = 0;
    let value_ty = match &component.data {
        syn::Data::Struct(x) => match &x.fields {
            syn::Fields::Named(x) => {
                n_fields = x.named.len();
                x.named.first()
            },
            syn::Fields::Unnamed(x) => {
                n_fields = x.unnamed.len();
                x.unnamed.first()
            },
            syn::Fields::Unit => None,
        },
        _ => None,
    }
    .map(|x| &x.ty);
    let extra_impl = match value_ty {
        Some(value_ty) => {
            let target = if n_fields == 1 {
                quote! { #value_ty }
            } else {
                quote! { #component_ty }
            };
            let param_field = if n_fields == 1 {
                quote! { self.component.0 }
            } else {
                quote! { self.component }
            };
            let out = if n_fields == 1 {
                let out = if args.skip_value {
                    quote! {}
                } else {
                    quote! {
                    impl #impl_generics Value for #component_ty #ty_generics #where_clause {
                        type Value = #target;

                        #[inline(always)]
                        fn get(&self) -> Self::Value {
                            self.0
                        }
                    }
                    }
                };
                quote! {
                    #out
                    impl #impl_generics core::ops::Deref for #component_ty #ty_generics #where_clause {
                        type Target = #target;

                        #[inline(always)]
                        fn deref(&self) -> &Self::Target {
                            &self.0
                        }
                    }

                    impl #impl_generics core::ops::DerefMut for #component_ty #ty_generics #where_clause {
                        #[inline(always)]
                        fn deref_mut(&mut self) -> &mut Self::Target {
                            &mut self.0
                        }
                    }
                }
            } else {
                quote! {}
            };

            let out = quote! {
                #out
                impl<'w, 's, const N: usize> core::ops::Deref for #metric<'w, 's, N> {
                    type Target = #target;

                    #[inline(always)]
                    fn deref(&self) -> &Self::Target {
                        &#param_field
                    }
                }

                impl<'w, 's, const N: usize> core::ops::DerefMut for #metric<'w, 's, N> {
                    #[inline(always)]
                    fn deref_mut(&mut self) -> &mut Self::Target {
                        &mut #param_field
                    }
                }
            };
            if !args.skip_value {
                quote! {
                    #out
                    impl<'w, 's, const N: usize> Value for #metric<'w, 's, N> {
                        type Value = #target;

                        #[inline(always)]
                        fn get(&self) -> Self::Value {
                            #param_field
                        }
                    }
                }
            } else {
                out
            }
        },
        _ => quote! {},
    };
    let out = quote! {
        #[derive(Clone, Copy, PartialEq, Default)]
        #[repr(C)]
        #component
        #vis struct #metric<'w, 's, const N: usize> {
            component: &'w mut #component_ty,
            _marker: inception::PhantomSystemParam<'w, 's, N>,
        }

        impl<'w, 's, const N: usize> inception::SystemParam for #metric<'w, 's, N> {
            type Item<'world, 'state, Wrld: inception::World> = #metric<'world, 'state, N>;
            type State = ();
            type Build<B: EcsBuilder, SB: SystemParamNameMapper + 'static, ParamName: 'static> = impl EcsBuilder;

            #[inline(always)]
            fn get_param<'world, 'state, Wrld: inception::World, SB: inception::SystemParamNameMapper, ParamName>(
                state: &'state mut Self::State,
                world: &'world mut Wrld,
            ) -> Self::Item<'world, 'state, Wrld> {
                #metric {
                    component: world.resource_mut::<#component_ty>(),
                    _marker: inception::PhantomSystemParam::default(),
                }
            }

            #[inline(always)]
            fn build<B: inception::EcsBuilder, SB: inception::SystemParamNameMapper + 'static, ParamName: 'static>(
                builder: B,
            ) -> Self::Build<B, SB, ParamName> {
                builder.add_resource(#component_ty::default())
            }
        }
        #extra_impl
    };
    // eprintln!("{}", out);
    out.into()
}

#[proc_macro_attribute]
pub fn input(_attr: TokenStream, input: TokenStream) -> TokenStream {
    let _esl = macros_util::crate_name("esl");
    let param = parse_macro_input!(input as DeriveInput);
    let param_ident = &param.ident;
    let resource_ident = format_ident!("{}Resource", param_ident);
    // let attr_args = parse_macro_input!(attr as AttributeArgs);
    let out = quote! {
        #[resource_value]
        #param

        impl InputField for #resource_ident {
            type Resource = #resource_ident;

            #[inline(always)]
            fn load(&self, resource: &mut Self::Resource) {
                *resource = *self;
            }
        }
    };
    // eprintln!("{}", out);
    out.into()
}
#[proc_macro_attribute]
pub fn impl_metric(_attr: TokenStream, input: TokenStream) -> TokenStream {
    let _esl = macros_util::crate_name("esl");
    let mut item_impl = parse_macro_input!(input as ItemImpl);
    // Metric is implemented for the component bacuase it is required to be static. There is no way
    // to make generic parameter inside `PhantomData` not static when the outer struct should be
    // static.
    // item_impl.self_ty = parse_quote!(#component);
    let mut method_id = None;
    for i in 0..item_impl.items.len() {
        if let syn::ImplItem::Method(m) = &mut item_impl.items[i] {
            expand_inputs(m.sig.inputs.iter_mut().map(|x| match x {
                syn::FnArg::Receiver(_) => {
                    panic!("Only functions without `self` receiver are allowed")
                },
                syn::FnArg::Typed(x) => &mut *x.ty,
            }));
            method_id = Some(i);
            break;
        }
    }
    let method = item_impl
        .items
        .remove(method_id.expect("Metric doesn't have an `update` function"));
    let method = if let syn::ImplItem::Method(m) = method {
        m
    } else {
        panic!();
    };
    let param_names = method.sig.inputs.iter().map(|x| {
        if let syn::FnArg::Typed(x) = x {
            let pat = &x.pat;
            quote! {
                #pat
            }
        } else {
            quote! {}
        }
    });
    let update_params = method.sig.inputs.iter().map(|x| {
        if let syn::FnArg::Typed(x) = x {
            let ty = &x.ty;
            quote! {
                #ty
            }
        } else {
            quote! {}
        }
    });
    let stmts = &method.block.stmts;
    let mut has_execution_order = false;
    for item in &item_impl.items {
        if let syn::ImplItem::Type(ty) = item {
            if ty.ident == "ExecutionOrder" {
                has_execution_order = true;
                break;
            }
        }
    }
    if !has_execution_order {
        let mut deps = method
            .sig
            .inputs
            .iter()
            .filter_map(|x| match x {
                syn::FnArg::Receiver(_) => {
                    panic!("Only functions without `self` receiver are allowed")
                },
                syn::FnArg::Typed(x) => match &*x.ty {
                    Type::Path(path) => {
                        let segment = path.path.segments.last().unwrap();
                        if segment.ident == "Metric" {
                            match &segment.arguments {
                                PathArguments::AngleBracketed(args) => {
                                    let n: Type = parse_quote! {N};
                                    args.args.iter().find(|x| match x {
                                        GenericArgument::Type(x) => {
                                            *x != *item_impl.self_ty && *x != n
                                        },
                                        _ => false,
                                    })
                                },
                                _ => panic!("`Metric` must be generic"),
                            }
                        } else {
                            None
                        }
                    },
                    _ => None,
                },
            })
            .peekable();
        if deps.peek().is_none() {
            item_impl.items.push(parse_quote! {
                type ExecutionOrder = Order0;
            });
        } else {
            item_impl.items.push(parse_quote! {
            type ExecutionOrder = <MaxExecutionOrder::<(#(<#deps as MetricTrait>::ExecutionOrder,)*)> as ExecutionOrder>::Next;
        });
        }
    }

    item_impl.items.push(parse_quote! {
        type UpdateParams<'w, 's, W: World, const N: usize> = (#(#update_params,)*);
    });
    item_impl.items.push(parse_quote! {
        #[inline(always)]
        fn update<'w, 's, W: World, const N: usize>(params: Self::UpdateParams<'w, 's, W, N>) {
            let (#(#param_names,)*) = params;
            #(#stmts)*
        }
    });
    let out = quote! {
        #item_impl
    };
    // eprintln!("{}", out);
    out.into()
}

#[proc_macro_derive(Value)]
pub fn value(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ItemStruct);
    let out = match input.fields {
        syn::Fields::Named(FieldsNamed { named: fields, .. })
        | syn::Fields::Unnamed(FieldsUnnamed {
            unnamed: fields, ..
        }) if fields.len() == 1 => {
            let field = fields.first().unwrap();
            let ty = &field.ty;
            let field = match &field.ident {
                Some(x) => quote! {#x},
                None => {
                    let field = LitInt::new("0", proc_macro2::Span::call_site());
                    quote! {#field}
                },
            };
            let ident = &input.ident;
            quote! {
                impl Value for #ident {
                    type Value = #ty;

                    #[inline(always)]
                    fn get(&self) -> Self::Value {
                        self.#field
                    }
                }
                impl core::ops::Deref for #ident {
                    type Target = #ty;

                    #[inline(always)]
                    fn deref(&self) -> &Self::Target {
                        &self.#field
                    }
                }
                impl core::ops::DerefMut for #ident {
                    #[inline(always)]
                    fn deref_mut(&mut self) -> &mut Self::Target {
                        &mut self.#field
                    }
                }
            }
        },
        _ => panic!("`Value can only be derived on structs with one field."),
    };
    // eprintln!("{}", out.to_string());
    out.into()
}

/// Creates a struct with `SystemParam`. This must be applied on `Indicator` impl of an indicator
/// state. The struct must be named `*State`.
#[proc_macro_attribute]
pub fn indicator(_attributes: TokenStream, input: TokenStream) -> TokenStream {
    let esl = macros_util::crate_name("esl");
    let input = parse_macro_input!(input as ItemImpl);
    let param;
    let state;
    let output;
    match &*input.self_ty {
        Type::Path(x) => {
            let state_name = &x.path.segments.last().unwrap().ident.to_string();
            let ident = state_name.strip_suffix("State").unwrap().to_string();
            param = format_ident!("{}", ident);
            state = format_ident!("{}State", ident);
            output = format_ident!("{}Output", ident);
        },
        _ => panic!("Only path is allowed for impl"),
    }

    let out = quote! {
        #input

        pub struct #param<'w, 's, const N: usize> {
            output: #output,
            _marker: inception::PhantomSystemParam<'w, 's, N>,
        }

        impl<'w, 's, const N: usize> core::ops::Deref for #param<'w, 's, N> {
            type Target = <#output as core::ops::Deref>::Target;

            #[inline(always)]
            fn deref(&self) -> &Self::Target {
                self.output.deref()
            }
        }

        impl<'w, 's, const N: usize> #esl::Value for #param<'w, 's, N> {
            type Value = <#output as #esl::Value>::Value;

            #[inline(always)]
            fn get(&self) -> Self::Value {
                self.output.get()
            }
        }

        impl<'w, 's, const N: usize> SystemParam for #param<'w, 's, N> {
            type Build<B: inception::EcsBuilder, SB: inception::SystemParamNameMapper + 'static, ParamName: 'static> =
                <crate::indicator::IndicatorPlugin<SB, ParamName, RsiState> as inception::SystemParamPlugin>::Build<B>;
            // cast lifetimes
            type Item<'world, 'state, Wrld: World> = #param<'world, 'state, N>;
            type State = ();

            inception::unimpl_get_param!();

            #[inline(always)]
            fn get_param_for_entity<'world, 'state, Wrld, SB, ParamName, E>(
                entity: &'world mut E,
                state: &'state mut Self::State,
                world: &'world mut Wrld,
            ) -> Option<Self::Item<'world, 'state, Wrld>>
            where
                Wrld: inception::World,
                SB: inception::SystemParamNameMapper,
                E: inception::EntityFetch,
                ParamName: 'static,
            {
                Some(#param {
                    output: *entity.config_mut::<SB, ParamName, #output>(),
                    _marker: inception::PhantomSystemParam::default(),
                })
            }

            #[inline(always)]
            fn build<B: inception::EcsBuilder, SB: inception::SystemParamNameMapper + 'static, ParamName: 'static>(
                builder: B,
            ) -> Self::Build<B, SB, ParamName> {
                crate::indicator::IndicatorPlugin::<SB, ParamName, #state>::build(
                    builder,
                )
            }
        }
    };
    out.into()
}

#[proc_macro]
pub fn impl_max_execution_order(_input: TokenStream) -> TokenStream {
    let implement = |x: Vec<u8>| {
        let orders = x.iter().map(|x| format_ident!("Order{}", x));
        let next = *x.iter().max().unwrap_or(&0) + 1;
        let next_order = format_ident!("Order{}", next);
        quote! {
            impl ExecutionOrder for MaxExecutionOrder<(#(#orders,)*)> {
                type Next = #next_order;
            }
        }
    };
    let n_orders_in_max = 3;
    let impls = (1..=n_orders_in_max).flat_map(|n_orders| {
        let mut base = Convert::new(10, N_ORDERS);
        (0..N_ORDERS.pow(n_orders)).map(move |x| {
            let mut output = base.convert::<u64, u8>(&[x]);
            while output.len() != n_orders as usize {
                output.push(0);
            }
            implement(output)
        })
    });
    let out = quote! {
        #(#impls)*
    };
    out.into()
}

#[proc_macro]
pub fn impl_add_metrics_and_trackers(_input: TokenStream) -> TokenStream {
    let impls = (0..N_ORDERS).map(|x| {
        let add_update_metric_fn = format_ident!("add_update_metric{}", x);
        let add_update_metric = format_ident!("AddUpdateMetric{}", x);
        let add_block_metric_fn = format_ident!("add_block_metric{}", x);
        let add_block_metric = format_ident!("AddBlockMetric{}", x);
        let update_metrics = format_ident!("update_metrics{}", x);
        let add_update_tracker_fn = format_ident!("add_update_tracker{}", x);
        let add_update_tracker = format_ident!("AddUpdateTracker{}", x);
        let add_block_tracker_fn = format_ident!("add_block_tracker{}", x);
        let add_block_tracker = format_ident!("AddBlockTracker{}", x);
        let block_metrics = format_ident!("block_metrics{}", x);
        let fields = (0..N_ORDERS).filter(|i| *i != x ).map(|x| {
            let update_metrics = format_ident!("update_metrics{}", x);
            let block_metrics = format_ident!("block_metrics{}", x);
            quote! {
                #update_metrics: self.#update_metrics,
                #block_metrics: self.#block_metrics,
            }

        });
        let update_metrics_fields = quote! {
            #block_metrics: self.#block_metrics,
            #update_metrics: Nested::new(
                MetricUpdateBuilderStruct {
                    _m: PhantomData::<M>,
                    _condition: PhantomData::<C>,
                },
                self.#update_metrics,
            )
        };
        let block_metrics_fields = quote! {
            #update_metrics: self.#update_metrics,
            #block_metrics: Nested::new(
                MetricUpdateBuilderStruct {
                    _m: PhantomData::<M>,
                    _condition: PhantomData::<C>,
                },
                self.#block_metrics,
            )
        };
        let update_trackers_fields = quote! {
            #block_metrics: self.#block_metrics,
            #update_metrics: Nested::new(
                TrackerUpdateBuilder {
                    metric_builder: MetricUpdateBuilderStruct {
                        _m: PhantomData::<M>,
                        _condition: PhantomData::<C>,
                    },
                },
                self.#update_metrics,
            )
        };
        let block_trackers_fields = quote! {
            #update_metrics: self.#update_metrics,
            #block_metrics: Nested::new(
                TrackerUpdateBuilder {
                    metric_builder: MetricUpdateBuilderStruct {
                        _m: PhantomData::<M>,
                        _condition: PhantomData::<C>,
                    },
                },
                self.#block_metrics,
            )
        };
        let common_fields = quote! {
            #(#fields)*
            mems: Nested::new(
                MetricMemStruct {
                    field_offset: self.field_offset,
                    _m: PhantomData::<M>,
                },
                self.mems,
            ),
            field_offset: self.field_offset + core::mem::size_of::<M>()
        };
        quote! {
            #[inline(always)]
            fn #add_update_metric_fn<C: Condition, M: MetricTrait>(self) -> Self::#add_update_metric<C, M> {
                MetricsBuilderStruct {
                    builder: self.builder.extend_entities(MetricComponent::<M>::default()),
                    #update_metrics_fields,
                    #common_fields,
                    n_trackers: self.n_trackers,
                }
            }
            #[inline(always)]
            fn #add_block_metric_fn<C: Condition, M: MetricTrait>(self) -> Self::#add_block_metric<C, M> {
                MetricsBuilderStruct {
                    builder: self.builder.extend_entities(MetricComponent::<M>::default()),
                    #block_metrics_fields,
                    #common_fields,
                    n_trackers: self.n_trackers,
                }
            }
            #[inline(always)]
            fn #add_update_tracker_fn<C: Condition, M: MetricTrait>(self) -> Self::#add_update_tracker<C, M> {
                MetricsBuilderStruct {
                    builder: self.builder.extend_entities(
                        MetricComponent::<M>::new(self.n_trackers)
                    ),
                    #update_trackers_fields,
                    #common_fields,
                    n_trackers: self.n_trackers + 1,
                }
            }
            #[inline(always)]
            fn #add_block_tracker_fn<C: Condition, M: MetricTrait>(self) -> Self::#add_block_tracker<C, M> {
                MetricsBuilderStruct {
                    builder: self.builder.extend_entities(
                        MetricComponent::<M>::new(self.n_trackers)
                    ),
                    #block_trackers_fields,
                    #common_fields,
                    n_trackers: self.n_trackers + 1,
                }
            }
        }
    });
    let out = quote! {
        #(#impls)*
    };
    out.into()
}
