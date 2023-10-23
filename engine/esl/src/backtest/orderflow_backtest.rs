use std::prelude::v1::*;

use inception::*;

use crate::{backtest::BacktestingParams, orderflow::*, stages::*, *};

pub struct OrderflowBacktestPlugin<I, S> {
    pub backtest_plugin: super::BacktestPlugin<I, S>,
}

impl<I: Inputs, S: Nest + 'static> Plugin for OrderflowBacktestPlugin<I, S> {
    type Deps<L: PluginLoader> = L;

    type Build<B: EcsBuilder> = impl EcsBuilder;

    fn deps<L: PluginLoader>(&mut self, loader: L) -> Self::Deps<L> {
        loader
    }

    fn build<B: EcsBuilder>(self, builder: B) -> Self::Build<B> {
        builder
            .add_resource(Elapsed(0))
            .init_resource::<TimestampNsResource>()
            .init_resource::<TypeMaskResource>()
            .init_resource::<AmountResource>()
            .init_resource::<NOrdersResource>()
            .add_plugin(self.backtest_plugin)
            .add_system(trade::new(), Trade::new())
            .add_system_without_plugin(input::new(), Input1::new())
    }
}

#[system]
fn input(
    timestamp: TimestampNs,
    start_timestamp: Res<StartTimestampNs>,
    mut elapsed: Res<Elapsed>,
) {
    elapsed.0 = *timestamp as u64 - start_timestamp.0;
}

#[system]
fn trade(
    mut params: BacktestingParams,
    price: Price,
    max_balance: Metric<MaxBalance>,
    elapsed: Res<Elapsed>,
) {
    // dbg!(*price);
    // dbg!(elapsed.seconds());
    // TODO: There are multiple event types. We shouldn't process them all.
    //if *max_balance > 1595. {
        // dbg!(elapsed.seconds());
        // panic!();
    //}
    // if *max_balance < 1595. {
    params.trade(*price, *price, *price);
    // }
}
