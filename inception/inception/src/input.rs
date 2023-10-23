use core::marker::PhantomData;

use ergnomics::*;

use crate::{system_param::SystemParamNameMapper, *};

pub trait Input {
    fn take<T: 'static, SB: SystemParamNameMapper, ParamName: 'static>(&mut self) -> T;
}

pub struct InputItem<I, PN> {
    // required to be public by `system` proc macro
    pub data: Option<I>,
    pub param_name: PhantomData<PN>,
}

impl<N: Input, I: 'static, PN: 'static> Input for Nested<N, InputItem<I, PN>> {
    fn take<T: 'static, SB: SystemParamNameMapper, ParamName: 'static>(&mut self) -> T {
        if T::type_id() == I::type_id() && PN::type_id() == ParamName::type_id() {
            unsafe {
                let value = self.item.data.take().expect("Input already taken");
                let casted = core::mem::transmute_copy::<I, T>(&value);
                core::mem::forget(value);
                return casted;
            }
        } else {
            self.inner.take::<T, SB, ParamName>()
        }
    }
}

impl Input for StackedNest {
    fn take<T: 'static, SB: SystemParamNameMapper, ParamName: 'static>(&mut self) -> T {
        panic!(
            "Input `{}` doesn't exist for {}(... {} ...)",
            core::any::type_name::<T>(),
            core::any::type_name::<SB>(),
            core::any::type_name::<ParamName>(),
        );
    }
}

/// System parameter that allows passing arguments to systems.
/// # Example
/// ```
/// use inception::*;
/// #[system]
/// fn my_system(input: In<i32>) {
///     println!("input: {}", *input);
/// }
/// pub type Update0 = Stage0;
/// schedule! {
///     struct Schedule,
///     Stage0 as Update0,
/// }
/// let ecs = EcsBuilderStruct::new::<_, 0>(ConfigBuilder::new(), Schedule::builder())
///     .add_system_to_stage(my_system::new(42), Update0::new())
///     .build()
///     .run();
/// ```
pub struct In<'w, 's, T, const N: usize> {
    data: &'s mut T,
    _marker: PhantomSystemParam<'w, 's, N>,
}

/// Same thing as `In` but doesn't hold any data. Useful when other parameters need to access
/// system input. This adds some confusion so it's not recommended to use this.
pub struct PhantomIn<'w, 's, T, const N: usize> {
    _data: PhantomData<T>,
    _marker: PhantomSystemParam<'w, 's, N>,
}

impl<'w, 's, T, const N: usize> core::ops::Deref for In<'w, 's, T, N> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.data
    }
}

impl<'w, 's, T, const N: usize> core::ops::DerefMut for In<'w, 's, T, N> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.data
    }
}

impl<'w, 's, T: 'static, const N: usize> SystemParam for PhantomIn<'w, 's, T, N> {
    type Item<'world, 'state, Wrld: World> = PhantomIn<'world, 'state, T, N>;
    type State = ();

    impl_no_plugin!();

    fn get_param<'world, 'state, Wrld: World, SB: SystemParamNameMapper, ParamName>(
        _state: &'state mut Self::State,
        _world: &'world mut Wrld,
    ) -> Self::Item<'world, 'state, Wrld> {
        PhantomIn {
            _data: Default::default(),
            _marker: PhantomSystemParam::default(),
        }
    }
}

pub struct InState<T> {
    data: T,
}

impl<T: 'static> SystemParamState for InState<T> {
    fn init<W: crate::World, SB: crate::SystemParamNameMapper, ParamName: 'static, I: Input>(
        inputs: &mut I,
        _world: &mut W,
    ) -> Self {
        Self {
            data: inputs.take::<T, SB, ParamName>(),
        }
    }
}

impl<'w, 's, T: 'static, const N: usize> SystemParam for In<'w, 's, T, N> {
    type Item<'world, 'state, Wrld: World> = In<'world, 'state, T, N>;
    type State = InState<T>;

    impl_no_plugin!();

    fn get_param<'world, 'state, Wrld: World, SB: SystemParamNameMapper, ParamName>(
        state: &'state mut Self::State,
        _world: &'world mut Wrld,
    ) -> Self::Item<'world, 'state, Wrld> {
        In {
            data: &mut state.data,
            _marker: PhantomSystemParam::default(),
        }
    }
}
