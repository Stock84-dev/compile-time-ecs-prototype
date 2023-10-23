use num_traits::Float;

pub trait NumExt<T = Self> {
    fn max_mut(&mut self, value: T);
    fn min_mut(&mut self, value: T);
    fn average(&mut self, new_entry: T, new_count: T);
    /// NOTE: only works for integers. If it is divisible by specified number and has remainder then
    /// increases self to the number that is divisible by specified number.
    fn div_ceil(&self, denominator: T) -> Self;
    // returns true if number is within <base * (1 - rel), base * (1 + rel)>
    fn within_percent(&self, base: T, rel: T) -> bool;
}

impl<T: Float> NumExt<T> for T {
    #[inline(always)]
    fn max_mut(&mut self, value: T) {
        if *self < value {
            *self = value;
        }
    }

    #[inline(always)]
    fn min_mut(&mut self, value: T) {
        if *self > value {
            *self = value;
        }
    }

    #[inline(always)]
    fn average(&mut self, new_entry: T, new_count: T) {
        *self = *self * ((new_count - Self::one()) / new_count) + new_entry / new_count;
    }

    #[inline(always)]
    fn div_ceil(&self, denominator: T) -> Self {
        (*self + denominator - T::one()) / denominator
    }

    #[inline(always)]
    fn within_percent(&self, base: T, rel: T) -> bool {
        *self > base * (T::one() - rel) && *self < base * (T::one() + rel)
    }
}
