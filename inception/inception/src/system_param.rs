use core::marker::PhantomData;

use all_tuples::{all_tuples, param_to_const_expr};
use ergnomics::*;

use crate::{entities::EntityFetch, input::Input, world::World, EcsBuilder, Entity};

/// Used in the `system` macro as a placeholder when the system is generic.
pub struct UnknownSystem;
pub trait ParamLabel: 'static {
    type System: 'static;
}

pub trait SystemParamState: 'static {
    fn init<W: crate::World, SB: crate::SystemParamNameMapper, ParamName: 'static, I: Input>(
        inputs: &mut I,
        world: &mut W,
    ) -> Self;
}

pub trait SystemParam: Sized {
    // Individual system params don't need `SB`. It needs to passed because the compiler thinks
    // that `SB` is used when using type inference with impl EcsBuilder.
    type Build<B: EcsBuilder, SB: SystemParamNameMapper + 'static, ParamName: 'static>: EcsBuilder;
    type State: SystemParamState;
    /// Used to cast lifetimes. This must be set to the Self but with the lifetimes and type of the
    /// world.
    type Item<'world, 'state, Wrld: World + 'static + 'world + 'state>;
    /// Determines wether to call `get_param_for_entity` or `get_param`. If it is a query, it will
    /// call `get_param_for_entity`.
    const IS_QUERY: bool = false;

    /// This gets called before calling a system, and is used to get the parameter.
    fn get_param<'world, 'state, Wrld: World, SB: SystemParamNameMapper, ParamName: 'static>(
        state: &'state mut Self::State,
        world: &'world mut Wrld,
    ) -> Self::Item<'world, 'state, Wrld>;

    /// Gets called for each entity in the world. If the system has any parameter that is a query,
    /// this will be called instead of `get_param` for all parameters.
    #[inline(always)]
    fn get_param_for_entity<'world, 'state, Wrld, SB, ParamName: 'static, E>(
        entity: &'world mut E,
        state: &'state mut Self::State,
        world: &'world mut Wrld,
    ) -> Option<Self::Item<'world, 'state, Wrld>>
    where
        Wrld: World,
        SB: SystemParamNameMapper,
        E: EntityFetch,
    {
        Some(Self::get_param::<_, SB, ParamName>(state, world))
    }

    /// Load any plugins that are needed for this system param.
    fn build<B: EcsBuilder, SB: SystemParamNameMapper + 'static, ParamName: 'static>(
        builder: B,
    ) -> Self::Build<B, SB, ParamName>;
}

/// Helper macro to imlement `SystemParam` for parameters that operate on each entity.
#[macro_export]
macro_rules! unimpl_get_param {
    () => {
        const IS_QUERY: bool = true;
        fn get_param<'world, 'state, Wrld: World, SB: SystemParamNameMapper, ParamName>(
            _state: &'state mut Self::State,
            _world: &'world mut Wrld,
        ) -> Self::Item<'world, 'state, Wrld> {
            unimplemented!()
        }
    };
}

#[macro_export]
macro_rules! impl_no_plugin {
    () => {
        type Build<B: EcsBuilder, SB: SystemParamNameMapper + 'static, ParamName: 'static> = B;

        #[inline(always)]
        fn build<B: EcsBuilder, SB: SystemParamNameMapper + 'static, ParamName: 'static>(
            builder: B,
        ) -> Self::Build<B, SB, ParamName> {
            builder
        }
    };
}

impl<T: 'static> SystemParam for &T {
    // cast lifetimes
    type Item<'world, 'state, Wrld: World> = &'world T;
    type State = ();

    unimpl_get_param!();

    impl_no_plugin!();

    #[inline(always)]
    fn get_param_for_entity<'world, 'state, Wrld, SB, ParamName, E>(
        entity: &'world mut E,
        _state: &'state mut Self::State,
        _world: &'world mut Wrld,
    ) -> Option<Self::Item<'world, 'state, Wrld>>
    where
        Wrld: World,
        SB: SystemParamNameMapper,
        E: EntityFetch,
    {
        entity.get_component()
    }
}
impl<T: 'static> SystemParam for &mut T {
    // cast lifetimes
    type Item<'world, 'state, Wrld: World> = &'world mut T;
    type State = ();

    unimpl_get_param!();

    impl_no_plugin!();

    #[inline(always)]
    fn get_param_for_entity<'world, 'state, Wrld, SB, ParamName, E>(
        entity: &'world mut E,
        _state: &'state mut Self::State,
        _world: &'world mut Wrld,
    ) -> Option<Self::Item<'world, 'state, Wrld>>
    where
        Wrld: World,
        SB: SystemParamNameMapper,
        E: EntityFetch,
    {
        entity.get_component_mut()
    }
}

pub trait Mapper<const N: usize> {
    type Name;
}

pub trait SystemParamNameMapper:
    Mapper<0>
    + Mapper<1>
    + Mapper<2>
    + Mapper<3>
    + Mapper<4>
    + Mapper<5>
    + Mapper<6>
    + Mapper<7>
    + Mapper<8>
    + Mapper<9>
    + Mapper<10>
    + Mapper<11>
    + Mapper<12>
    + Mapper<13>
    + Mapper<14>
    + Mapper<15>
    + Mapper<15>
    + 'static
{
    fn get_param_name<const PARAM_ID: usize>() -> Option<&'static str>;
}

pub struct PhantomSystemParam<'w, 's, const N: usize> {
    _w: PhantomData<&'w ()>,
    _s: PhantomData<&'s ()>,
}

impl<'w, 's, const N: usize> Default for PhantomSystemParam<'w, 's, N> {
    #[inline(always)]
    fn default() -> Self {
        Self {
            _w: Default::default(),
            _s: Default::default(),
        }
    }
}

/// A system parameter that can be used to access a resource.
#[derive(Deref, DerefMut)]
pub struct Res<'w, 's, T, const N: usize> {
    #[deref]
    #[deref_mut]
    data: &'w mut T,
    _marker: PhantomSystemParam<'w, 's, N>,
}

impl<'w, 's, T: 'static, const N: usize> SystemParam for Res<'w, 's, T, N> {
    // cast lifetimes
    type Item<'world, 'state, Wrld: World> = Res<'world, 'state, T, N>;
    type State = ();

    type Build<B: EcsBuilder, SB: SystemParamNameMapper + 'static, ParamName: 'static> =
        impl EcsBuilder;

    #[inline(always)]
    fn get_param<'world, 'state, Wrld: World, SB: SystemParamNameMapper, ParamName>(
        _state: &'state mut Self::State,
        world: &'world mut Wrld,
    ) -> Self::Item<'world, 'state, Wrld> {
        Res {
            data: world.resource_mut::<T>(),
            _marker: Default::default(),
        }
    }

    #[inline(always)]
    fn build<B: EcsBuilder, SB: SystemParamNameMapper + 'static, ParamName: 'static>(
        builder: B,
    ) -> Self::Build<B, SB, ParamName> {
        builder
    }
}

/// Gets entity id from the current entity.
#[derive(Deref, DerefMut)]
pub struct EntityParam<'w, 's, const N: usize> {
    #[deref]
    #[deref_mut]
    entity: Entity,
    _marker: PhantomSystemParam<'w, 's, N>,
}

impl<'w, 's, const N: usize> SystemParam for EntityParam<'w, 's, N> {
    // cast lifetimes
    type Item<'world, 'state, Wrld: World> = EntityParam<'world, 'state, N>;
    type State = ();

    type Build<B: EcsBuilder, SB: SystemParamNameMapper + 'static, ParamName: 'static> =
        impl EcsBuilder;

    unimpl_get_param!();

    #[inline(always)]
    fn get_param_for_entity<'world, 'state, Wrld, SB, ParamName, E>(
        entity: &'world mut E,
        _state: &'state mut Self::State,
        _world: &'world mut Wrld,
    ) -> Option<Self::Item<'world, 'state, Wrld>>
    where
        Wrld: World,
        SB: SystemParamNameMapper,
        E: EntityFetch,
    {
        Some(EntityParam {
            entity: entity.entity(),
            _marker: Default::default(),
        })
    }

    #[inline(always)]
    fn build<B: EcsBuilder, SB: SystemParamNameMapper + 'static, ParamName: 'static>(
        builder: B,
    ) -> Self::Build<B, SB, ParamName> {
        builder
    }
}

macro_rules! impl_system_param {
    ($($param:ident),*) => {
        impl<$($param: SystemParam),*> SystemParam for ($($param,)*)
        {
            type State = ($($param::State,)*);
            type Item<'w, 's, Wrld: World> = ($($param::Item<'w, 's, Wrld>,)*);
            type Build<B: EcsBuilder, SB: SystemParamNameMapper + 'static, ParamName: 'static> = impl EcsBuilder;
            const IS_QUERY: bool = $($param::IS_QUERY ||)* false;

            #[inline(always)]
            #[allow(unused_variables, unused_mut)]
            fn get_param<'world, 'state, Wrld: World, SB: SystemParamNameMapper, ParamName: 'static>(
                state: &'state mut Self::State,
                world: &'world mut Wrld,
            ) -> Self::Item<'world, 'state, Wrld> {
                #[allow(non_snake_case)]
                let ($($param,)*) = state;
                let world = world as *mut Wrld;
                #[allow(unused_unsafe)]
                unsafe {
                    ($({
                        #![allow(unused_assignments)]
                        let world = &mut *world;
                        let param = $param::get_param::<_, SB, <SB as Mapper<{param_to_const_expr!($param)}>>::Name>($param, world);
                        param
                    },)*)
                }
            }

            #[inline(always)]
            #[allow(unused_variables, unused_mut)]
            fn get_param_for_entity<'world, 'state, Wrld: World, SB: SystemParamNameMapper, ParamName: 'static, E: EntityFetch>(
                entity: &'world mut E,
                state: &'state mut Self::State,
                world: &'world mut Wrld,
            ) -> Option<Self::Item<'world, 'state, Wrld>> {
                #[allow(non_snake_case)]
                let ($($param,)*) = state;
                let world = world as *mut Wrld;
                let entity = entity as *mut E;
                #[allow(unused_unsafe)]
                unsafe {
                    Some(($({
                        #![allow(unused_assignments)]
                        let world = &mut *world;
                        let param = $param::get_param_for_entity::<_, SB, <SB as Mapper<{param_to_const_expr!($param)}>>::Name, E>(
                            &mut *entity,
                            $param,
                            world
                        )?;
                        param
                    },)*))
                }
            }

            #[inline(always)]
            #[allow(unused_variables, unused_mut)]
            fn build<B: EcsBuilder, SB: SystemParamNameMapper + 'static, ParamName: 'static>(
                builder: B,
            ) -> Self::Build<B, SB, ParamName> {
                let mut builder = builder;
                $(
                    let mut builder = $param::build::<_, SB, <SB as Mapper<{param_to_const_expr!($param)}>>::Name>(builder);
                )*
                builder
            }
        }
    };
}
all_tuples!(impl_system_param, 0, 16, P);

macro_rules! impl_system_param_state {
    ($($param:ident),*) => {
        impl<$($param: SystemParamState),*> SystemParamState for ($($param,)*) {
            #[inline(always)]
            #[allow(unused_variables, unused_mut)]
            fn init<W: crate::World, SB: crate::SystemParamNameMapper, ParamName, I: Input>(
                inputs: &mut I,
                world: &mut W,
            ) -> Self {
                let world = world as *mut W;
                ($(
                    unsafe {
                        let state = $param::init::<
                            W,
                            SB,
                            <SB as Mapper<{param_to_const_expr!($param)}>>::Name,
                            I,
                        >(inputs, &mut *world);
                        state
                    },
                )*)
            }
        }
    };
}
all_tuples!(impl_system_param_state, 0, 16, P);
