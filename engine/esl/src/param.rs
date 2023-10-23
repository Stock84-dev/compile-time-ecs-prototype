use inception::*;

use crate::Value;

pub struct ParamConfig(pub f32);

pub struct HyperParam<'w, 's, const N: usize> {
    value: f32,
    _marker: PhantomSystemParam<'w, 's, N>,
}

impl<'w, 's, const N: usize> Value for HyperParam<'w, 's, N> {
    type Value = f32;

    #[inline(always)]
    fn get(&self) -> Self::Value {
        self.value
    }
}

impl<'w, 's, const N: usize> SystemParam for HyperParam<'w, 's, N> {
    type Item<'world, 'state, Wrld: World> = HyperParam<'world, 'state, N>;
    type State = ();

    type Build<B: EcsBuilder, SB: SystemParamNameMapper + 'static, ParamName: 'static> =
        impl EcsBuilder;

    unimpl_get_param!();

    #[inline(always)]
    fn get_param_for_entity<'world, 'state, Wrld, SB, ParamName, E>(
        entity: &'world mut E,
        _state: &'state mut Self::State,
        world: &'world mut Wrld,
    ) -> Option<Self::Item<'world, 'state, Wrld>>
    where
        Wrld: World,
        SB: SystemParamNameMapper,
        E: EntityFetch,
        ParamName: 'static,
    {
        Some(HyperParam {
            value: entity.config::<SB, ParamName, ParamConfig>().0,
            _marker: PhantomSystemParam::default(),
        })
    }

    #[inline(always)]
    fn build<B: EcsBuilder, SB: SystemParamNameMapper + 'static, ParamName: 'static>(
        builder: B,
    ) -> Self::Build<B, SB, ParamName> {
        builder
    }
}

pub mod macros {
    #[macro_export]
    /// Expands into `HyperParam<'w, 's, N>`
    macro_rules! Param {
        ($($_tokens:tt)+) => {
            // Empty, `strategy` macro expands it
            ()
        };
    }
}
