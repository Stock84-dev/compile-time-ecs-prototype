use ::core::{fmt::Debug, ops::Deref};
use bytemuck::{Pod, Zeroable};
use inception::{
    EcsBuilder, SystemParam, SystemParamNameMapper, SystemParamPlugin, World,
};
use yata::prelude::*;
pub use yata::{methods::*, *};

use crate::{indicator::IndicatorPlugin, schema::Readable, Indicator, Reader, Value};

impl<T> Indicator for T
where
    T: Method + 'static,
    T::Params: Pod,
    T::Input: Readable + Sized,
    T::Output: Pod + Debug,
{
    type Config = T::Params;
    type Input = T::Input;
    type Output = T::Output;

    fn new(config: Self::Config) -> Self {
        // TODO: this hould be a proper value
        let input = unsafe { ::core::mem::zeroed::<T::Input>() };
        T::new(config, &input).unwrap()
    }

    fn init<R: Reader<Self::Input>>(&mut self, _reader: &mut R) -> usize {
        0
        // for i in 1..self.period + 1 {
        //     let price = reader.read(i);
        //     let prev_price = reader.read(i - 1);
        //     let diff = price - prev_price;
        //     self.avg_gain += diff.positive() * diff / self.period_f32;
        //     self.avg_loss -= diff.negative() * diff / self.period_f32;
        // }
        // self.period + 1
    }

    fn update<R: Reader<Self::Input>>(&mut self, reader: &mut R, offset: usize) -> Self::Output {
        let price = reader.read(offset);
        self.next(&price)
    }
}

/// A system parameter to acces indicator values from `yata` crate. The indicators will be
/// inaccurate until one full window length has passed.
pub struct Ind<'w, 's, T: Indicator, const N: usize> {
    output: T::Output,
    _marker: inception::PhantomSystemParam<'w, 's, N>,
}

impl<'w, 's, T: Indicator, const N: usize> ::core::ops::Deref for Ind<'w, 's, T, N>
where
    T::Output: Deref,
{
    type Target = <T::Output as ::core::ops::Deref>::Target;

    fn deref(&self) -> &Self::Target {
        self.output.deref()
    }
}
impl<'w, 's, T: Indicator, const N: usize> esl::Value for Ind<'w, 's, T, N>
where
    T::Output: Value,
{
    type Value = <T::Output as esl::Value>::Value;

    fn get(&self) -> Self::Value {
        self.output.get()
    }
}

impl<'w, 's, T: Indicator, const N: usize> SystemParam for Ind<'w, 's, T, N>
where
    T::Output: Zeroable,
{
    type Build<B: EcsBuilder, SB: SystemParamNameMapper + 'static, ParamName: 'static> =
        <IndicatorPlugin<SB, ParamName, T> as SystemParamPlugin>::Build<
            B::ExtendGenericConfig<SB, ParamName, T::Output>,
        >;
    type Item<'world, 'state, Wrld: World> = Ind<'world, 'state, T, N>;
    type State = ();

    inception::unimpl_get_param!();

    #[inline(always)]
    fn get_param_for_entity<'world, 'state, Wrld, SB, ParamName, E>(
        entity: &'world mut E,
        _state: &'state mut Self::State,
        world: &'world mut Wrld,
    ) -> Option<Self::Item<'world, 'state, Wrld>>
    where
        Wrld: inception::World,
        SB: inception::SystemParamNameMapper,
        E: inception::EntityFetch,
        ParamName: 'static,
    {
        Some(Ind {
            output: *entity.config::<SB, ParamName, T::Output>(),
            _marker: inception::PhantomSystemParam::default(),
        })
    }

    #[inline(always)]
    fn build<
        B: inception::EcsBuilder,
        SB: inception::SystemParamNameMapper + 'static,
        ParamName: 'static,
    >(
        builder: B,
    ) -> Self::Build<B, SB, ParamName> {
        let builder = builder.extend_generic_config::<SB, ParamName, _>(T::Output::zeroed());
        crate::indicator::IndicatorPlugin::<SB, ParamName, T>::build(builder)
    }
}
