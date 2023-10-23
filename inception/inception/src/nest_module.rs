use std::prelude::v1::*;
use either::Either;
use ergnomics::OptionExt;
use inception_macros::SystemParamPlugin;

#[macro_export]
macro_rules! NestTy {
    ($item: ident, $($items: ident),+) => {
        Nested<NestTy![$($items),+], $item>
    };
    ($item: ident) => {
        Nested<StackedNest, $item>
    };
}

pub trait Nestable {
    #[inline(always)]
    fn push<T>(self, item: T) -> Nested<Self, T>
    where
        Self: Sized,
    {
        Nested { item, inner: self }
    }

    #[inline(always)]
    fn default() -> StackedNest
    where
        Self: Sized,
    {
        StackedNest
    }
}

pub trait Nest: Nestable {
    fn get_if_index_is_one<T: 'static>(&self, index: usize) -> Option<&T>;
    #[inline(always)]
    fn field<T: 'static>(&self) -> &T {
        self.get().expect_or_else(|| {
            format!(
                "Failed to get a field of type `{}`",
                std::any::type_name::<T>()
            )
        })
    }
    #[inline(always)]
    fn field_mut<T: 'static>(&mut self) -> &mut T {
        self.get_mut().expect_or_else(|| {
            format!(
                "Failed to get a field of type `{}`",
                std::any::type_name::<T>()
            )
        })
    }
    fn get<T: 'static>(&self) -> Option<&T>;
    fn get_mut<T: 'static>(&mut self) -> Option<&mut T>;
    fn len(&self) -> usize;

    #[inline(always)]
    fn get_by_index<T: 'static>(&self, index: usize) -> Option<&T> {
        self.get_if_index_is_one(self.len() - index)
    }
}

impl<N: Nestable> Nestable for Option<N> {}

impl<A: Nest + Nestable> Nest for Option<A> {
    #[inline(always)]
    fn get_if_index_is_one<T: 'static>(&self, index: usize) -> Option<&T> {
        self.as_ref()?.get_if_index_is_one::<T>(index)
    }

    #[inline(always)]
    fn get<T: 'static>(&self) -> Option<&T> {
        match self {
            Some(x) => x.get(),
            None => None,
        }
    }

    #[inline(always)]
    fn get_mut<T: 'static>(&mut self) -> Option<&mut T> {
        match self {
            Some(x) => x.get_mut(),
            None => None,
        }
    }

    #[inline(always)]
    fn len(&self) -> usize {
        match self {
            Some(x) => x.len(),
            None => 0,
        }
    }
}

impl<L, R> Nestable for Either<L, R> {}

impl<L: Nest, R: Nest> Nest for Either<L, R> {
    #[inline(always)]
    fn get_if_index_is_one<T: 'static>(&self, index: usize) -> Option<&T> {
        match self {
            Either::Left(x) => x.get_if_index_is_one::<T>(index),
            Either::Right(x) => x.get_if_index_is_one::<T>(index),
        }
    }

    #[inline(always)]
    fn get<T: 'static>(&self) -> Option<&T> {
        match self {
            Either::Left(x) => x.get::<T>(),
            Either::Right(x) => x.get::<T>(),
        }
    }

    #[inline(always)]
    fn get_mut<T: 'static>(&mut self) -> Option<&mut T> {
        match self {
            Either::Left(x) => x.get_mut::<T>(),
            Either::Right(x) => x.get_mut::<T>(),
        }
    }

    #[inline(always)]
    fn len(&self) -> usize {
        match self {
            Either::Left(x) => x.len(),
            Either::Right(x) => x.len(),
        }
    }
}

#[derive(Debug, Clone, SystemParamPlugin)]
pub struct StackedNest;

impl Nestable for StackedNest {}

impl Nest for StackedNest {
    #[inline(always)]
    fn get_if_index_is_one<T: 'static>(&self, _index: usize) -> Option<&T> {
        None
    }

    #[inline(always)]
    fn get<T: 'static>(&self) -> Option<&T> {
        None
    }

    #[inline(always)]
    fn get_mut<T: 'static>(&mut self) -> Option<&mut T> {
        None
    }

    #[inline(always)]
    fn len(&self) -> usize {
        0
    }

    #[inline(always)]
    fn get_by_index<T: 'static>(&self, _index: usize) -> Option<&T> {
        None
    }
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct Nested<A, T> {
    pub item: T,
    pub inner: A,
}

impl<A, T> Nested<A, T> {
    #[inline(always)]
    pub fn new(item: T, inner: A) -> Self {
        Self { item, inner }
    }
}

impl<N, T> Nestable for Nested<N, T> {}

impl<S: 'static, A: Nest> Nest for Nested<A, S> {
    #[inline(always)]
    fn get_if_index_is_one<T: 'static>(&self, index: usize) -> Option<&T> {
        if index == 1 {
            if core::any::TypeId::of::<T>() == core::any::TypeId::of::<S>() {
                unsafe { Some(&*(&self.item as *const S as *const T)) }
            } else {
                None
            }
        } else {
            self.inner.get_if_index_is_one(index - 1)
        }
    }

    #[inline(always)]
    fn get<T: 'static>(&self) -> Option<&T> {
        if core::any::TypeId::of::<T>() == core::any::TypeId::of::<S>() {
            unsafe { Some(&*(&self.item as *const S as *const T)) }
        } else {
            self.inner.get()
        }
    }

    #[inline(always)]
    fn get_mut<T: 'static>(&mut self) -> Option<&mut T> {
        if core::any::TypeId::of::<T>() == core::any::TypeId::of::<S>() {
            unsafe { Some(&mut *(&mut self.item as *mut S as *mut T)) }
        } else {
            self.inner.get_mut()
        }
    }

    #[inline(always)]
    fn len(&self) -> usize {
        self.inner.len() + 1
    }
}
