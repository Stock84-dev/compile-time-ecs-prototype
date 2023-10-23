use bytemuck::{Pod, Zeroable};
use ergnomics::*;
use esl_macros::Value;
use inception::*;

use crate::*;

#[repr(C)]
#[derive(Default, Clone, Copy, PartialEq, Debug, Zeroable, Pod, Value)]
pub struct RsiOutput {
    rsi: f32,
}

#[repr(C)]
#[derive(Default, Clone, Copy, PartialEq, Debug, Zeroable, Pod)]
pub struct RsiConfig {
    pub len: usize,
}

#[repr(C)]
#[derive(Default, Clone, Copy, PartialEq, Debug, Zeroable, Pod)]
pub struct RsiState {
    period: usize,
    period_f32: f32,
    avg_gain: f32,
    avg_loss: f32,
    _padding: u32,
}

#[indicator]
impl Indicator for RsiState {
    type Config = RsiConfig;
    type Input = f32;
    type Output = RsiOutput;

    #[inline(always)]
    fn new(config: Self::Config) -> Self {
        Self {
            period: config.len,
            avg_gain: 0.0,
            avg_loss: 0.0,
            period_f32: config.len as f32,
            _padding: 0,
        }
    }

    #[inline(always)]
    fn init<R: Reader<Self::Input>>(&mut self, reader: &mut R) -> usize {
        for i in 1..self.period + 1 {
            let price = reader.read(i);
            let prev_price = reader.read(i - 1);
            let diff = price - prev_price;
            self.avg_gain += diff.positive() * diff / self.period_f32;
            self.avg_loss -= diff.negative() * diff / self.period_f32;
        }
        self.period + 1
    }

    #[inline(always)]
    fn update<R: Reader<Self::Input>>(&mut self, reader: &mut R, offset: usize) -> Self::Output {
        let price = reader.read(offset);
        let prev_price = reader.read(offset - 1);
        let diff = price - prev_price;
        let last_price = reader.read(offset - self.period);
        let last_prev_price = reader.read(offset - self.period - 1);
        let last_diff = last_price - last_prev_price;
        // Using rolling average because it is faster, but it is prone to prcision errors
        // First remove from average to minimize floating point precision errors
        self.avg_gain -= last_diff.positive() * last_diff / self.period_f32;
        self.avg_loss += last_diff.negative() * last_diff / self.period_f32;

        self.avg_gain += diff.positive() * diff / self.period_f32;
        self.avg_loss -= diff.negative() * diff / self.period_f32;

        let mut rs = self.avg_gain / self.avg_loss;
        // faster than branchless version, branchless uses aditional mul instruction with cmov
        rs = if rs.is_nan() { 1. } else { rs };
        let rsi = 100. - (100. / (1. + rs));
        // we could clamp the value between 0 and 100, no need to bother, happens rarely
        //        assert!(rsi >= 0.);
        //        assert!(rsi <= 100.);

        // println!("{}", rsi);
        RsiOutput { rsi }
    }
}

// impl Indicator for RsiState {
//     type Config = RsiConfig;
//     type Input = f32;
//     type Output = RsiOutput;
//
//     fn new(config: Self::Config) -> Self {
//         Self {
//             period: config.len,
//             avg_gain: 0.0,
//             avg_loss: 0.0,
//             period_f32: config.len as f32,
//             _padding: 0,
//         }
//     }
//
//     fn init<R: Reader<Self::Input>>(&mut self, reader: &mut R) -> usize {
//         for i in 1..self.period + 1 {
//             let price = reader.read(i);
//             let prev_price = reader.read(i - 1);
//             let diff = price - prev_price;
//             self.avg_gain += diff.positive() * diff / self.period_f32;
//             self.avg_loss -= diff.negative() * diff / self.period_f32;
//         }
//         self.period + 1
//     }
//
//     fn update<R: Reader<Self::Input>>(&mut self, reader: &mut R, offset: usize) -> Self::Output {
//         let price = reader.read(offset);
//         let prev_price = reader.read(offset - 1);
//         let diff = price - prev_price;
//         let last_price = reader.read(offset - self.period);
//         let last_prev_price = reader.read(offset - self.period - 1);
//         let last_diff = last_price - last_prev_price;
//         self.avg_gain -= last_diff.positive() * last_diff / self.period_f32;
//         self.avg_loss += last_diff.negative() * last_diff / self.period_f32;
//         self.avg_gain += diff.positive() * diff / self.period_f32;
//         self.avg_loss -= diff.negative() * diff / self.period_f32;
//         let mut rs = self.avg_gain / self.avg_loss;
//         rs = if rs.is_nan() { 1. } else { rs };
//         let rsi = 100. - (100. / (1. + rs));
//         RsiOutput { rsi }
//     }
// }
// pub struct Rsi<'w, 's, const N: usize> {
//     output: RsiOutput,
//     _marker: inception::PhantomSystemParam<'w, 's, N>,
// }
// impl<'w, 's, const N: usize> core::ops::Deref for Rsi<'w, 's, N> {
//     type Target = <RsiOutput as core::ops::Deref>::Target;
//
//     fn deref(&self) -> &Self::Target {
//         self.output.deref()
//     }
// }
// impl<'w, 's, const N: usize> esl::Value for Rsi<'w, 's, N> {
//     type Value = <RsiOutput as esl::Value>::Value;
//
//     fn get(&self) -> Self::Value {
//         self.output.get()
//     }
// }
// impl<'w, 's, const N: usize> SystemParam for Rsi<'w, 's, N> {
//     type Build<B:inception::EcsBuilder,SB:inception::SystemParamNameMapper+'static,ParamName:'
// static>  =  <crate::indicator::IndicatorPlugin<SB,ParamName,RsiState>as
// inception::SystemParamPlugin> ::Build<B::AddConfig<RsiOutput> , > ;     type Config = ();
//     type Item<'world, 'state, Wrld: World> = Rsi<'world, 'state, N>;
//     type PluginConfig = ();
//     type State = ();
//
//     inception::unimpl_get_param!();
//
//     #[inline(always)]
//     fn get_param_for_entity<'world, 'state, Wrld, SB, ParamName, E>(
//         entity: &'world mut E,
//         config: &mut Self::Config,
//         context: &SystemParamContext,
//         state: &'state mut Self::State,
//         world: &'world mut Wrld,
//     ) -> Option<Self::Item<'world, 'state, Wrld>>
//     where
//         Wrld: inception::World,
//         SB: inception::SystemParamNameMapper,
//         E: inception::EntityFetch,
//     {
//         entity.get_component::<AccountTag>()?;
//         Some(Rsi {
//             output: *world.config::<RsiOutput>(context),
//             _marker: inception::PhantomSystemParam::default(),
//         })
//     }
//
//     #[inline(always)]
//     fn build<
//         B: inception::EcsBuilder,
//         SB: inception::SystemParamNameMapper + 'static,
//         ParamName: 'static,
//     >(
//         config: Self::PluginConfig,
//         context: &SystemParamContext,
//         builder: B,
//     ) -> Self::Build<B, SB, ParamName> {
//         let builder = builder.add_config(*context, RsiOutput::zeroed());
//         crate::indicator::IndicatorPlugin::<SB, ParamName, RsiState>::build(
//             config, context, builder,
//         )
//     }
// }
