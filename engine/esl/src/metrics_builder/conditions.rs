//! Update a performance metric when some condition is met.
use super::*;

/// Executes an update function of a performance metric when the position is closed.
pub struct OnPositionClosed;

impl Condition for OnPositionClosed {
    type Params<'w, 's, W: World, const N: usize> = EntityEvents<'w, 's, PositionClosed, N>;

    #[inline(always)]
    fn run<'w, 's, W: World, const N: usize>(params: Self::Params<'w, 's, W, N>) -> Skip {
        if params.is_empty() {
            Skip::True
        } else {
            Skip::False
        }
    }
}

pub struct Always;

impl Condition for Always {
    type Params<'w, 's, W: World, const N: usize> = ();

    #[inline(always)]
    fn run<'w, 's, W: World, const N: usize>(_params: Self::Params<'w, 's, W, N>) -> Skip {
        Skip::False
    }
}
