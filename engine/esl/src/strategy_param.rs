use core::marker::PhantomData;

use inception::*;

pub struct PhantomStrategyParam<'w, 's, const N: usize, W> {
    _marker: PhantomSystemParam<'w, 's, W>,
}

impl<'w, 's, const N: usize, F, W> Default for PhantomStrategyParam<'w, 's, N, F, W> {
    fn default() -> Self {
        Self {
            _f: Default::default(),
            _marker: Default::default(),
        }
    }
}
