use inception::*;

use crate::{
    backtest::BacktestingParams,
    hlcv::*,
    stages::{IndicatorCompute, Trade},
    *,
};

pub struct HlcvBacktestPlugin<I, S> {
    pub timeframe_s: u32,
    pub backtest_plugin: super::BacktestPlugin<I, S>,
}

impl<I: Inputs, S: Nest + 'static> Plugin
    for HlcvBacktestPlugin<I, S>
{
    type Deps<L: PluginLoader> = L;

    type Build<B: EcsBuilder> = impl EcsBuilder;

    #[inline(always)]
    fn deps<L: PluginLoader>(&mut self, loader: L) -> Self::Deps<L> {
        loader
    }

    #[inline(always)]
    fn build<B: EcsBuilder>(self, builder: B) -> Self::Build<B> {
        builder
            .add_resource(TimeframeS(self.timeframe_s))
            .add_resource(Elapsed(0))
            .init_resource::<HighResource>()
            .init_resource::<LowResource>()
            // .init_resource::<CloseResource>()
            .init_resource::<VolumeResource>()
            .add_plugin(self.backtest_plugin)
            .add_system(input::new(), IndicatorCompute::new())
            .add_system(trade::new(), Trade::new())
    }
}
#[system]
fn input(index: LoopIndex, mut elapsed: Res<Elapsed>, timeframe: Res<TimeframeS>) {
    elapsed.0 = *index as u64 * timeframe.0 as u64 * 1_000_000_000;
}

#[system]
fn trade(mut params: BacktestingParams, price: Price, high: High, low: Low) {
    params.trade(*high, *low, *price);
}
