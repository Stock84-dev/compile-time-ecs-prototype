#![feature(type_name_of_val)]
#![deny(unused_must_use)]
// This example backtests a strategy and prints the RSI for every candle.

use core::marker::PhantomData;
use std::time::Instant;

use bytemuck::offset_of;
use esl::{
    block_relays::UpdateRelay,
    simulation_relays::SimulationEnding,
    stages::{PostTrade0, Signal},
    ta::rsi::{Rsi, RsiConfig, RsiState},
    *,
};
use hlcv_loader::load_hlcv;
use merovingian::hlcv::Hlcv;
use mouse::prelude::*;

mod bench;

#[tokio::main]
async fn main() -> Result<()> {
    unsafe {
        // IMPORTANT: Change `cache_dir` in config.yaml to a valid path.
        ::config::load("config.yaml")?;
    }
    let start_ts = 1443184345;
    // Total amount of 1 second candles in the dataset
    let count = 1633869265 - 1443184345;
    // let count = 10000 * 20 * 60;
    let hlcvs_mapped = load_hlcv("BitMEX", "XBTUSD", start_ts, count).await?;
    let timeframe_s = 60 * 20;
    // let timeframe_s = 60 * 60;
    let hlcvs = esl_utils::change_timeframe(&hlcvs_mapped, timeframe_s);
    run(&hlcvs, timeframe_s);
    Ok(())
}

#[inline(never)]
pub fn run(hlcvs: &[Hlcv], timeframe_s: u32) -> f32 {
    let input_len = hlcvs.len();
    let mut metrics = vec![0u8; 1024 * 1024];
    let now = Instant::now();
    let schedule = esl::stages::BacktestSchedule::builder();
    let builder = EcsBuilderStruct::new::<_, 6>(schedule, EntitiesBuilderStruct1::new())
        // Resources are like singletons in ECS. These ones may be needed. If a resource isn't
        // present then the system will panic. If an entity doesn't have a component then a system
        // will just skip it.
        .add_resource(MetricsPtr(metrics.as_mut_ptr()))
        .add_resource(AccountsPerThread(1))
        .add_resource(ThreadsPerDevice(1))
        .add_resource(ThreadId(0))
        .add_resource(NSamples(input_len))
        .add_resource(RiskFreeRate(0.))
        .add_resource(TradingDaysPerYear(252.))
        .add_plugin(CorePlugin {
            loop_end_bound_excluded: input_len,
        })
        .add_plugin(HlcvBacktestPlugin {
            timeframe_s,
            backtest_plugin: BacktestPlugin {
                inputs: unsafe { Series::<hlcv::HlcvInputNest>::new(hlcvs.as_ptr() as *const u8) },
                starting_balance: 1.0,
                slippage: types::Slippage::Relative(0.),
                fee: types::Fee::RelativeToVolume(0.00075 * 2.),
                inputs_marker: PhantomData::<hlcv::HlcvInput>,
            },
        })
        .add_plugin(ComputeIndicatorPlugin::<
            engine_strategies::rsi::rsi,
            Entity0,
            RsiState,
        >::new(RsiConfig { len: 11 }, unsafe {
            Series::with_stride(
                (hlcvs.as_ptr() as *const u8).add(offset_of!(Hlcv, close)),
                Hlcv::size(),
            )
        }))
        .add_config::<engine_strategies::rsi::lline, Entity0, _>(ParamConfig(19.))
        .add_config::<engine_strategies::rsi::hline, Entity0, _>(ParamConfig(6.));
    let mut ecs = MetricsBuilderStruct::new(builder)
        .add_metric::<SimulationEnding, UpdateRelay, OnPositionClosed, Balance>()
        .finish()
        .add_system(engine_strategies::rsi::new(), Signal::new())
        .add_system(finish::new(), esl::stages::End::new())
        .build();
    ecs.run();
    let elapsed_ns = now.elapsed().as_nanos();
    println!(
        "Elapsed: {} ms, {} candles/s",
        elapsed_ns / 1_000_000,
        input_len as f32 * 1_000_000_000. / elapsed_ns as f32,
    );
    let h = hlcvs.iter().map(|h| h.high).collect::<Vec<f32>>();
    let l = hlcvs.iter().map(|h| h.low).collect::<Vec<f32>>();
    let c = hlcvs.iter().map(|h| h.close).collect::<Vec<f32>>();
    bench::bench(&h, &l, &c);
    let balance = ecs
        .component::<MetricComponent<Balance>>(Entity(0))
        .metric
        .get();
    return balance;
}

#[system]
fn finish(balance: Metric<Balance>) {
    println!("Balance: {}", balance.get());
}
