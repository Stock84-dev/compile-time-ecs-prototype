use core::marker::PhantomData;

use all_tuples::all_tuples;
use either::Either;
use ergnomics::*;

use crate::{
    entities::{EntityFetch, EntityFnMut},
    world::BasicWorld,
    *,
};

pub trait StaticSystem<W: World> {
    fn call(&mut self, world: &mut W);
}

impl<W: World, T: System<'static, 'static, W> + 'static> StaticSystem<W> for T {
    #[inline(always)]
    fn call(&mut self, world: &mut W) {
        unsafe {
            let me = core::mem::transmute::<&mut T, &'static mut T>(self);
            let world = core::mem::transmute::<&mut W, &'static mut W>(world);
            System::<'static, 'static, W>::call(me, world);
        }
    }
}

pub trait System<'w, 's, W: World> {
    fn call(&'s mut self, world: &'w mut W);
}

impl<'w, 's, W: World> System<'w, 's, W> for StackedNest {
    #[inline(always)]
    fn call(&'s mut self, _world: &'w mut W) {}
}

impl<'w, 's, W, Inner, F> System<'w, 's, W> for Nested<Inner, F>
where
    W: World,
    Inner: System<'w, 's, W>,
    F: System<'w, 's, W>,
{
    #[inline(always)]
    fn call(&'s mut self, world: &'w mut W) {
        let world_ptr = world as *const _ as *mut W;
        // SAFETY: Systems are implemented for specific lifetime
        // to satisfy that fn(T): IntoSystem is implemented where T: 'w. This then forces
        // lifetime propagation upwards. And HRTB cannot be used from the caller because it is
        // implemented for specific lifetime. The caller assigns 'static lifetime instead. As long
        // as the system doesn't store any references then this should be OK.
        unsafe {
            self.item.call(&mut *world_ptr);
            self.inner.call(&mut *world_ptr);
        }
    }
}

impl<'w, 's, W: World, F: System<'w, 's, W>> System<'w, 's, W> for Option<F> {
    #[inline(always)]
    fn call(&'s mut self, world: &'w mut W) {
        if let Some(f) = self {
            f.call(world);
        }
    }
}

impl<'w, 's, W: World, L: System<'w, 's, W>, R: System<'w, 's, W>> System<'w, 's, W>
    for Either<L, R>
{
    #[inline(always)]
    fn call(&'s mut self, world: &'w mut W) {
        match self {
            Either::Left(l) => l.call(world),
            Either::Right(r) => r.call(world),
        }
    }
}

pub struct CustomFn<F> {
    system: F,
}

// pub trait IntoSystem<'w, 's, W: World + 'static + 'w + 's, P: SystemParam> {
//     type System<SB: SystemBuilder<'w, 's> + SystemParamNameMapper + 'w + 's>: System<'w, 's, W>;
//     fn into_system<SB: SystemBuilder<'w, 's> + SystemParamNameMapper + 'w, I: Input>(
//         self,
//         world: &mut W,
//         inputs: &mut I,
//     ) -> Self::System<SB>
//     where
//         Self: Sized;
// }
// impl<'w, 's, W, F, P> IntoSystem<'w, 's, W, P> for F
// where
//     W: World,
//     F: SystemWithParams<'w, 's, W, P>,
//     P: SystemParam<Item<'w, 's, W> = P>,
// {
//     type System<SB: SystemBuilder<'w, 's> + SystemParamNameMapper + 'w> =
//         SystemContainer<SB, CustomFn<F>, P>;
//
//     #[inline(always)]
//     fn into_system<SB: SystemBuilder<'w, 's> + SystemParamNameMapper + 'w, I: Input>(
//         self,
//         world: &mut W,
//         inputs: &mut I,
//     ) -> Self::System<SB>
//     where
//         Self: Sized,
//     {
//         SystemContainer {
//             system: CustomFn { system: self },
//             states: P::State::init::<W, SB, (), I>(inputs, world),
//             _system_builder: PhantomData,
//         }
//     }
// }
pub trait IntoInferredSystem<'w, 's, SB: SystemParamNameMapper, W: World> {
    type System: System<'w, 's, W>;
}
macro_rules! impl_into_inferred_system {
    ($($param: ident),*) => {
        impl<'w, 's, SB, W, $($param),*> IntoInferredSystem<'w, 's, SB, W> for fn($($param),*)
        where
            SB: SystemParamNameMapper + 'w + 's,
            W: World,
            ($($param,)*): SystemParam<Item<'w, 's, W> = ($($param,)*)>,
        {
            type System = SystemContainer<SB, CustomFn<Self>, ($($param,)*)>;
        }
    };
}

all_tuples!(impl_into_inferred_system, 0, 16, P);

macro_rules! impl_mapper_for_fn {
    ($($param_id: expr),*; $param: tt) => {
        $(
            impl_mapper_for_fn!(@impl, $param_id, $param);
        )*
    };
    (@impl, $param_id: expr, ($($param: ident),*)) => {
        impl<$($param),*> crate::system_param::Mapper<$param_id> for fn($($param),*) {
            type Name = ();
        }
    };
}

const PARAM_NAMES: [&'static str; 16] = [
    "0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12", "13", "14", "15",
];

macro_rules! impl_param_name_mapper {
    ($($param: ident),*) => {
        impl<$($param: 'static),*> SystemParamNameMapper for fn($($param),*)
        {
            fn get_param_name<const PARAM_ID: usize>() -> Option<&'static str> {
                PARAM_NAMES.get(PARAM_ID).copied()
            }
        }
        impl_mapper_for_fn!(0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15; ($($param),*));
    };
}

all_tuples!(impl_param_name_mapper, 0, 16, P);

impl<'w, 's, W, F, P> SystemWithParams<'w, 's, W, P> for CustomFn<F>
where
    F: SystemWithParams<'w, 's, W, P>,
    W: World,
    P: SystemParam<Item<'w, 's, W> = P>,
{
    #[inline(always)]
    fn call(&mut self, param: P) {
        self.system.call(param);
    }
}

pub struct SystemContainer<SB, F, Params: SystemParam> {
    _system_builder: PhantomData<SB>,
    system: F,
    states: Params::State,
}

impl<SB, F, Params> SystemParamPlugin for SystemContainer<SB, F, Params>
where
    SB: SystemParamNameMapper + 'static,
    Params: SystemParam,
{
    type Build<B: EcsBuilder> = Params::Build<B, SB, ()>;

    #[inline(always)]
    fn build<B: EcsBuilder>(mut builder: B) -> Self::Build<B> {
        Params::build::<B, SB, ()>(builder)
    }
}
struct ForEach<'a, 'w, 's, W, SB, F, P: SystemParam> {
    world: *mut W,
    states: &'s mut P::State,
    system: &'a mut F,
    _w: PhantomData<&'w SB>,
}
impl<'a, 'w, 's, W, P, SB, F> EntityFnMut for ForEach<'a, 'w, 's, W, SB, F, P>
where
    W: World,
    P: SystemParam,
    SB: SystemParamNameMapper,
    F: SystemWithParams<'w, 's, W, P>,
{
    #[inline(always)]
    fn call_mut<E: EntityFetch>(&mut self, entity: &mut E) {
        unsafe {
            let lifetime_params = some!(P::get_param_for_entity::<_, SB, (), E>(
                entity,
                self.states,
                &mut *self.world,
            ));
            let params = core::mem::transmute_copy(&lifetime_params);
            core::mem::forget(lifetime_params);
            self.system.call(params);
        };
    }
}

pub struct SystemState<P: SystemParam> {
    states: P::State,
}

impl<P: SystemParam> SystemState<P> {
    #[inline(always)]
    pub fn new<W: World, SB: SystemParamNameMapper + 'static, I: Input>(
        world: &mut W,
        inputs: &mut I,
    ) -> Self {
        Self {
            states: P::State::init::<W, SB, (), I>(inputs, world),
        }
    }

    #[inline(always)]
    pub fn call<
        'w,
        's,
        W: World,
        SB: SystemParamNameMapper + 'static,
        S: SystemWithParams<'w, 's, W, P>,
    >(
        &'s mut self,
        system: &mut S,
        world: &'w mut W,
    ) {
        if P::IS_QUERY {
            let for_each = ForEach {
                world,
                states: &mut self.states,
                system,
                _w: PhantomData::<&SB>,
            };
            world.for_each(for_each);
        } else {
            let lifetime_params = P::get_param::<_, SB, ()>(&mut self.states, world);
            let params = unsafe { core::mem::transmute_copy(&lifetime_params) };
            core::mem::forget(lifetime_params);
            system.call(params);
        }
    }
}

impl<'w, 's, W: World, F, P, SB> System<'w, 's, W> for SystemContainer<SB, F, P>
where
    SB: SystemParamNameMapper + 'w,
    F: SystemWithParams<'w, 's, W, P>,
    P: SystemParam<Item<'w, 's, W> = P>,
{
    #[inline(always)]
    fn call(&'s mut self, world: &'w mut W) {
        if P::IS_QUERY {
            let for_each = ForEach {
                world,
                states: &mut self.states,
                system: &mut self.system,
                _w: PhantomData::<&SB>,
            };
            world.for_each(for_each);
        } else {
            let lifetime_params = P::get_param::<_, SB, ()>(&mut self.states, world);
            let params = unsafe { core::mem::transmute_copy(&lifetime_params) };
            core::mem::forget(lifetime_params);
            self.system.call(params);
        }
    }
}

pub trait SystemBuilder<'w, 's> {
    type System<W: World + 'static + 'w + 's, const N: usize>: System<'w, 's, W> + SystemParamPlugin;

    fn build<W: World + 'static, const N: usize>(self, world: &mut W) -> Self::System<W, N>;

    #[inline(always)]
    fn load_plugins<B: EcsBuilder, const N: usize>(
        builder: B,
    ) -> <Self::System<BasicWorld, N> as SystemParamPlugin>::Build<B> {
        Self::System::build(builder)
    }
}

impl<'w, 's, Inner, F> SystemBuilder<'w, 's> for Nested<Inner, F>
where
    Inner: SystemBuilder<'w, 's>,
    F: SystemBuilder<'w, 's>,
{
    type System<W: World, const N: usize> = Nested<Inner::System<W, N>, F::System<W, N>>;

    #[inline(always)]
    fn build<W: World, const N: usize>(self, world: &mut W) -> Self::System<W, N> {
        Nested {
            inner: self.inner.build(world),
            item: self.item.build(world),
        }
    }
}

impl<Inner, F> SystemParamPlugin for Nested<Inner, F>
where
    Inner: SystemParamPlugin,
    F: SystemParamPlugin,
{
    type Build<B: EcsBuilder> = F::Build<Inner::Build<B>>;

    #[inline(always)]
    fn build<B: EcsBuilder>(builder: B) -> Self::Build<B> {
        let builder = Inner::build(builder);
        F::build(builder)
    }
}

impl<'w, 's> SystemBuilder<'w, 's> for StackedNest {
    type System<W: World, const N: usize> = StackedNest;

    #[inline(always)]
    fn build<W: World, const N: usize>(self, _world: &mut W) -> Self::System<W, N> {
        StackedNest
    }
}

pub trait SystemWithParams<'w, 's, W, P> {
    fn call(&mut self, param: P);
}

macro_rules! impl_system {
    ($($param:ident),*) => {
        impl<'w, 's, W, F, $($param),*> SystemWithParams<'w, 's, W, ($($param,)*)> for F
        where
            W: World,
            F: FnMut($($param),*),
        {
            #[inline(always)]
            fn call(&mut self, params: ($($param,)*)) {
                #[allow(non_snake_case)]
                let ($($param,)*) = params;
                (self)($($param),*);
            }
        }
    };
}
all_tuples!(impl_system, 0, 16, P);
