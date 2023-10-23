#![no_std]
use alloc::string::String;

extern crate alloc;

#[macro_export]
macro_rules! some_loop {
    ($e:expr) => {
        match $e {
            Some(x) => x,
            None => continue,
        }
    };
}

#[macro_export]
macro_rules! some {
    ($e:expr) => {
        match $e {
            Some(x) => x,
            None => return,
        }
    };
}

pub use num::*;
mod num;

pub trait OptionExt {
    type Item;
    fn expect_or_else<F: FnOnce() -> S, S: Into<String>>(self, f: F) -> Self::Item;
}

impl<T> OptionExt for Option<T> {
    type Item = T;

    #[inline(always)]
    fn expect_or_else<F: FnOnce() -> S, S: Into<String>>(self, f: F) -> Self::Item {
        match self {
            Some(x) => x,
            None => panic!("{}", f().into()),
        }
    }
}

pub trait TypeExt {
    fn type_name() -> &'static str;
}

impl<T: ?Sized> TypeExt for T {
    #[inline(always)]
    fn type_name() -> &'static str {
        core::any::type_name::<T>()
    }
}

pub trait TypeIdExt {
    fn type_id() -> core::any::TypeId;
}

impl<T: ?Sized + 'static> TypeIdExt for T {
    #[inline(always)]
    fn type_id() -> core::any::TypeId {
        core::any::TypeId::of::<T>()
    }
}

pub trait BoolExt {
    fn to_f32(self) -> f32;
    fn to_f64(self) -> f64;
}

impl BoolExt for bool {
    #[inline(always)]
    fn to_f32(self) -> f32 {
        self as u8 as f32
    }

    #[inline(always)]
    fn to_f64(self) -> f64 {
        self as u8 as f64
    }
}

pub trait FloatExt {
    fn positive(self) -> Self;
    fn negative(self) -> Self;
}

impl FloatExt for f32 {
    #[inline(always)]
    fn positive(self) -> Self {
        (self > 0.) as u8 as Self
    }

    #[inline(always)]
    fn negative(self) -> Self {
        (self < 0.) as u8 as Self
    }
}

impl FloatExt for f64 {
    #[inline(always)]
    fn positive(self) -> Self {
        (self > 0.) as u8 as Self
    }

    #[inline(always)]
    fn negative(self) -> Self {
        (self < 0.) as u8 as Self
    }
}
