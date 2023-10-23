use core::any::TypeId;
use core::marker::PhantomData;

use ergnomics::*;

use crate::{
    entities::EntityFetch,
    nest_module::{Nest, Nested, StackedNest},
    Entity, Nestable, *,
};

pub struct ConfigValue<System, Param, T>(pub T, pub std::marker::PhantomData<(System, Param)>);

impl<System, Param, T: Clone> Clone for ConfigValue<System, Param, T> {
    fn clone(&self) -> Self {
        Self(self.0.clone(), self.1.clone())
    }
}

/// A system parameter that can be used to fetch a config parameter for each entity that has
/// `SystemParamContext` component. It will use that context to fetch the config `T`.
pub struct EntityConfig<'w, 's, S, P, T, const N: usize> {
    config: &'w mut T,
    _context: PhantomData<(S, P)>,
    _marker: PhantomSystemParam<'w, 's, N>,
}

impl<'w, 's, S, P, T: 'static, const N: usize> SystemParam for EntityConfig<'w, 's, S, P, T, N>
where
    S: 'static,
    P: 'static,
{
    // cast lifetimes
    type Item<'world, 'state, Wrld: World> = EntityConfig<'world, 'state, S, P, T, N>;
    type State = ();

    unimpl_get_param!();

    impl_no_plugin!();

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
        Some(EntityConfig {
            config: entity.config_mut::<S, P, T>(),
            _marker: PhantomSystemParam::default(),
            _context: PhantomData,
        })
    }
}

impl<'w, 's, S, P, T, const N: usize> core::ops::Deref for EntityConfig<'w, 's, S, P, T, N> {
    type Target = T;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        self.config
    }
}

impl<'w, 's, S, P, T, const N: usize> core::ops::DerefMut for EntityConfig<'w, 's, S, P, T, N> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.config
    }
}
