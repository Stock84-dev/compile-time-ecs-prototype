use std::prelude::v1::*;
use inception::*;
use num_traits::{FromPrimitive, Num};

pub trait Value {
    type Value: Num + PartialOrd + FromPrimitive + Copy + Default + 'static;
    fn get(&self) -> Self::Value;
    fn get_copied(self) -> Self::Value
    where
        Self: Sized,
    {
        self.get()
    }
}

impl Value for f32 {
    type Value = f32;

    fn get(&self) -> Self::Value {
        *self
    }
}

impl<T: Value> Value for &T {
    type Value = T::Value;

    fn get(&self) -> Self::Value {
        T::get(*self)
    }
}

impl<T: Value> Value for &mut T {
    type Value = T::Value;

    fn get(&self) -> Self::Value {
        T::get(*self)
    }
}

pub trait ValueSystemParam: Value {
    type Build<B: EcsBuilder, SB: SystemParamNameMapper>: EcsBuilder;
    type State: SystemParamState;
    type Config: 'static;
    // cast lifetimes
    type Item<'world, 'state, Wrld: World + 'world + 'state>: SystemParam<State = Self::State> + Value;
    const IS_QUERY: bool = false;

    fn get_param<'world, 'state, Wrld: World, SB: SystemParamNameMapper>(
        state: &'state mut Self::State,
        world: &'world mut Wrld,
    ) -> Self::Item<'world, 'state, Wrld>;

    fn get_param_for_entity<'world, 'state, W: World, SB: SystemParamNameMapper, E: EntityFetch>(
        _entity: &'world mut E,
        state: &'state mut Self::State,
        world: &'world mut W,
    ) -> Option<Self::Item<'world, 'state, W>> {
        Some(Self::get_param::<_, SB>(state, world))
    }

    fn build<B: EcsBuilder, SB: SystemParamNameMapper>(
        builder: B,
    ) -> Self::Build<B, SB>;
}

// impl<P: SystemParam + Value> ValueSystemParam for P {
//     type Build<B: EcsBuilder, SB: SystemParamNameMapper> = P::Build<B, SB>;
//     type Config = P::Config;
//     type Item<'world, 'state, Wrld: World> = P::Item<'world, 'state, Wrld>;
//     type PluginConfig = P::PluginConfig;
//     type State = P::State;
//
//     fn get_param<'world, 'state, Wrld: World, SB: SystemParamNameMapper>(
//         config: &mut Self::Config,
//         context: &SystemParamContext,
//         state: &'state mut Self::State,
//         world: &'world mut Wrld,
//     ) -> Self::Item<'world, 'state, Wrld> {
//         P::get_param::<Wrld, SB>(config, context, state, world)
//     }
//
//     fn get_param_for_entity<'world, 'state, W: World, SB: SystemParamNameMapper, E: EntityFetch>(
//         entity: &'world mut E,
//         config: &mut Self::Config,
//         context: &SystemParamContext,
//         state: &'state mut Self::State,
//         world: &'world mut W,
//     ) -> Option<Self::Item<'world, 'state, W>> {
//         P::get_param_for_entity::<W, SB, E>(entity, config, context, state, world)
//     }
//
//     fn build<B: EcsBuilder, SB: SystemParamNameMapper>(
//         config: Self::PluginConfig,
//         context: &SystemParamContext,
//         builder: B,
//     ) -> Self::Build<B, SB> {
//         P::build(config, context, builder)
//     }
// }
