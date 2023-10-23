use std::ops::{Deref, DerefMut};

use inception::*;

pub struct LoopIndexResource(pub usize);

pub struct LoopIndex<'w, 's, const N: usize> {
    data: &'w mut LoopIndexResource,
    _marker: PhantomSystemParam<'w, 's, N>,
}
impl<'w, 's, const N: usize> LoopIndex<'w, 's, N> {
    pub fn max_mut(&mut self, other: usize) {
        self.data.0 = self.data.0.max(other);
    }
}

impl<'w, 's, const N: usize> SystemParam for LoopIndex<'w, 's, N> {
    // cast lifetimes
    type Item<'world, 'state, Wrld: World> = LoopIndex<'world, 'state, N>;
    type State = ();

    type Build<B: EcsBuilder, SB: SystemParamNameMapper + 'static, ParamName: 'static> =
        impl EcsBuilder;

    #[inline(always)]
    fn get_param<'world, 'state, Wrld: World, SB: SystemParamNameMapper, ParamName>(
        _state: &'state mut Self::State,
        world: &'world mut Wrld,
    ) -> Self::Item<'world, 'state, Wrld> {
        LoopIndex {
            data: world.resource_mut::<LoopIndexResource>(),
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

impl<'w, 's, const N: usize> Deref for LoopIndex<'w, 's, N> {
    type Target = usize;

    fn deref(&self) -> &Self::Target {
        &self.data.0
    }
}

impl<'w, 's, const N: usize> DerefMut for LoopIndex<'w, 's, N> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data.0
    }
}
