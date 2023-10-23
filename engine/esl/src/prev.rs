use std::{marker::PhantomData, prelude::v1::*};

use inception::{PhantomIn, *};

use crate::{stages::UpdatePrev, value::Value};

/// Stores the previous value of a system parameter along with the current one.
pub struct Prev<'w, 's, P: Value, const N: usize> {
    prev: P::Value,
    cur: P::Value,
    _p: PhantomData<P>,
    _marker: PhantomSystemParam<'w, 's, N>,
}

#[inline(always)]
fn get_param_values<'world, 'state, P, Wrld, SB, ParamName>(
    state: &'state mut P::State,
    world: &'world mut Wrld,
) -> (P::Value, P::Value)
where
    P: SystemParam + Value,
    Wrld: World,
    SB: SystemParamNameMapper,
    ParamName: 'static,
{
    todo!()
    // // SAFETY: P::Item is the same as P but with different lifetimes and world type.
    // // `SystemParam` works for any world type. This is needed because trait bounds cannot be
    // // added to associated types on impl. They would need to be added on the trait itself.
    // let cur = unsafe {
    //     let item = P::get_param::<Wrld, SB, ParamName>(config, context, state, world);
    //     let p: P = core::mem::transmute_copy(&item);
    //     core::mem::forget(item);
    //     p.get()
    // };
    // let config = entity.component_mut::<config::ConfigValue<SB, ParamName, State<P::Value>>>();
    // let config = &mut config.0;
    // // let config = world.config_mut::<State<P::Value>>(context);
    // let prev = config.prev;
    // config.prev = cur;
    // (prev, cur)
}

#[inline(always)]
fn get_param_values_for_entity<'world, 'state, P, Wrld, SB, ParamName, E>(
    entity: &'world mut E,
    state: &'state mut P::State,
    world: &'world mut Wrld,
) -> Option<(P::Value, P::Value)>
where
    P: SystemParam + Value,
    Wrld: World,
    SB: SystemParamNameMapper,
    E: EntityFetch,
    ParamName: 'static,
{
    // SAFETY: P::Item is the same as P but with different lifetimes and world type.
    // `SystemParam` works for any world type. This is needed because trait bounds cannot be
    // added to associated types on impl. They would need to be added on the trait itself.
    let cur = unsafe {
        let item = P::get_param_for_entity::<Wrld, SB, ParamName, E>(entity, state, world)?;
        let p: P = core::mem::transmute_copy::<<P as SystemParam>::Item<'_, '_, Wrld>, P>(&item);
        core::mem::forget(item);
        p.get()
    };
    let config = entity.config_mut::<SB, ParamName, State<P::Value>>();
    // let config = world.config_mut::<State<P::Value>>(context);
    let prev = config.prev;
    config.prev = cur;
    Some((prev, cur))
}

impl<'w, 's, P, const N: usize> SystemParam for Prev<'w, 's, P, N>
where
    // Multiple systems are used, so
    // states cannot be shared.
    P: SystemParam<State = ()> + Value + 'static,
{
    type Build<B: EcsBuilder, SB: SystemParamNameMapper + 'static, ParamName: 'static> =
        <<<P as SystemParam>::Build<B, SB, ParamName> as EcsBuilder>::ExtendGenericConfig<
            SB,
            ParamName,
            State<P::Value>,
        > as EcsBuilder>::AddSystemToStage<
            update_prev::System<P, SB, ParamName>,
            inception::stages::Stage3,
        >;
    type Item<'world, 'state, Wrld: World> = Prev<'world, 'state, P, N>;
    type State = P::State;

    const IS_QUERY: bool = P::IS_QUERY;

    #[inline(always)]
    fn get_param<'world, 'state, Wrld: World, SB: SystemParamNameMapper, ParamName: 'static>(
        state: &'state mut Self::State,
        world: &'world mut Wrld,
    ) -> Self::Item<'world, 'state, Wrld> {
        let (prev, cur) = get_param_values::<P, Wrld, SB, ParamName>(state, world);
        Prev {
            prev,
            cur,
            _marker: PhantomSystemParam::default(),
            _p: PhantomData,
        }
    }

    #[inline(always)]
    fn get_param_for_entity<'world, 'state, Wrld, SB, ParamName, E>(
        entity: &'world mut E,
        state: &'state mut Self::State,
        world: &'world mut Wrld,
    ) -> Option<Self::Item<'world, 'state, Wrld>>
    where
        Wrld: World,
        SB: SystemParamNameMapper,
        E: EntityFetch,
        ParamName: 'static,
    {
        let (prev, cur) =
            get_param_values_for_entity::<P, Wrld, SB, ParamName, E>(entity, state, world)?;
        Some(Prev {
            prev,
            cur,
            _marker: PhantomSystemParam::default(),
            _p: PhantomData,
        })
    }

    #[inline(always)]
    fn build<B: EcsBuilder, SB: SystemParamNameMapper, ParamName: 'static>(
        builder: B,
    ) -> Self::Build<B, SB, ParamName> {
        P::build::<B, SB, ParamName>(builder)
            .extend_generic_config::<SB, ParamName, _>(State {
                prev: P::Value::default(),
            })
            .add_system(update_prev::new::<P, SB, ParamName>(), UpdatePrev::new())
    }
}

#[derive(Clone)]
pub struct State<T> {
    prev: T,
}

impl<'w, 's, P: SystemParam + Value, const N: usize> Prev<'w, 's, P, N> {
    #[inline(always)]
    pub fn prev(&self) -> P::Value {
        self.prev
    }

    #[inline(always)]
    pub fn cur(&self) -> P::Value {
        self.cur
    }
}

impl<'w, 's, P: SystemParam + Value<Value = f32>, const N: usize> Prev<'w, 's, P, N> {
    #[inline(always)]
    pub fn crosses_from_above(&self, value: impl Value<Value = f32>) -> bool {
        let value = value.get();
        // dbg!(self.prev, self.cur);
        self.prev >= value && self.cur < value
    }

    #[inline(always)]
    pub fn crosses_from_below(&self, value: impl Value<Value = f32>) -> bool {
        let value = value.get();
        self.prev < value && self.cur >= value
    }
}

#[system]
fn update_prev<P, PS, PP>(_p: prev2::Prev<P, PS, PP>)
where
    P: Value + SystemParam<State = ()> + 'static,
    PS: SystemParamNameMapper,
    PP: 'static,
{
}

// Avoid recursion when caluclating the associated type of SystemParam::Build<...>. The above
// parameter adds a system to update its state. If that system would use the same parameter, it
// then spawn it again, and so on.
mod prev2 {
    use super::*;

    pub struct Prev<'w, 's, P: Value, PS, PP, const N: usize> {
        _p: PhantomData<(P, PS, PP)>,
        _marker: PhantomSystemParam<'w, 's, N>,
    }

    impl<'w, 's, P, PS, PP, const N: usize> SystemParam for Prev<'w, 's, P, PS, PP, N>
    where
        P: SystemParam<State = ()> + Value + 'static,
        PS: SystemParamNameMapper,
        PP: 'static,
    {
        type Build<B: EcsBuilder, SB: SystemParamNameMapper + 'static, ParamName: 'static> = B;
        type Item<'world, 'state, Wrld: World> = Prev<'world, 'state, P, PS, PP, N>;
        type State = ();

        const IS_QUERY: bool = P::IS_QUERY;

        #[inline(always)]
        fn get_param<'world, 'state, Wrld: World, SB: SystemParamNameMapper, ParamName: 'static>(
            state: &'state mut Self::State,
            world: &'world mut Wrld,
        ) -> Self::Item<'world, 'state, Wrld> {
            unsafe {
                get_param_values::<P, Wrld, PS, PP>(&mut (), world);
                Prev {
                    _marker: PhantomSystemParam::default(),
                    _p: PhantomData,
                }
            }
        }

        #[inline(always)]
        fn get_param_for_entity<'world, 'state, Wrld, SB, ParamName, E>(
            entity: &'world mut E,
            state: &'state mut Self::State,
            world: &'world mut Wrld,
        ) -> Option<Self::Item<'world, 'state, Wrld>>
        where
            Wrld: World,
            SB: SystemParamNameMapper,
            E: EntityFetch,
            ParamName: 'static,
        {
            unsafe {
                get_param_values_for_entity::<P, Wrld, PS, PP, E>(entity, &mut (), world)?;
                Some(Prev {
                    _marker: PhantomSystemParam::default(),
                    _p: PhantomData,
                })
            }
        }

        #[inline(always)]
        fn build<B: EcsBuilder, SB: SystemParamNameMapper + 'static, ParamName: 'static>(
            builder: B,
        ) -> Self::Build<B, SB, ParamName> {
            builder
        }
    }
}
