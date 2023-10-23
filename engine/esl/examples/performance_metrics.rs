#![recursion_limit = "256"]
#![feature(type_name_of_val)]
#![deny(unused_must_use)]
use core::marker::PhantomData;
use std::time::Instant;

use esl::{
    block_relays::UpdateRelay,
    simulation_relays::SimulationEnding,
    stages::{End, Signal},
    ta::rsi::{RsiConfig, RsiState},
    *,
};
use hlcv_loader::load_hlcv;
use memoffset::offset_of;
use merovingian::hlcv::Hlcv;
use mouse::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    unsafe {
        // IMPORTANT: Change `cache_dir` in config.yaml to a valid path.
        ::config::load("config.yaml")?;
    }
    warn!("Logging works!");
    let start_ts = 1443184345;
    let _count = 1633869265 - 1443184345;
    // let count = 1000 * 60 * 60;
    let count = 10000 * 20 * 60;
    let hlcvs_mapped = load_hlcv("BitMEX", "XBTUSD", start_ts, count).await?;
    let timeframe_s = 60 * 60;
    // let timeframe_s = 60;
    let hlcvs = esl_utils::change_timeframe(&hlcvs_mapped, timeframe_s);
    // At the start of the backtest, metrics are loaded from this vector. At the end of the
    // backtest, metrics are written back.
    // We provide a big enough vector to avoid out of bounds.
    let mut metrics = vec![0u8; 1024 * 1024];
    let input_len = hlcvs.len();
    let now = Instant::now();
    let schedule = esl::stages::BacktestSchedule::builder();
    let builder = EcsBuilderStruct::new::<_, 6>(schedule, EntitiesBuilderStruct1::new())
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
        // The first paramater indicates when a metric should be activated. Use this parameter for
        // now.
        // The second parameter indicates in which stage a metric should be updated. (`UpdateRelay`
        // or `BlockRelay`). Update executes in the loop, block executes after the loop.
        // The third parameter indicates when a metric should be updated. (`OnPositionClosed`
        // updates after a position is closed).
        // The fourth parameter indicates the type of the metric.
        // Metrics have other `Metric<T>` dependencies, you must add them manually. To find them go
        // to definition and look inside the `update` function.
        .add_metric::<SimulationEnding, UpdateRelay, OnPositionClosed, Balance>()
        .add_metric::<SimulationEnding, UpdateRelay, OnPositionClosed, MaxBalance>()
        .add_metric::<SimulationEnding, UpdateRelay, OnPositionClosed, Sum<MaxBalance>>()
        .add_metric::<SimulationEnding, UpdateRelay, OnPositionClosed, Drawdown>()
        .add_metric::<SimulationEnding, UpdateRelay, OnPositionClosed, MaxDrawdown>()
        .add_metric::<SimulationEnding, UpdateRelay, OnPositionClosed, NTrades>()
        .add_metric::<SimulationEnding, UpdateRelay, OnPositionClosed, BalanceDelta>()
        .add_metric::<SimulationEnding, UpdateRelay, OnPositionClosed, BalanceDeltaRel>()
        .add_metric::<SimulationEnding, UpdateRelay, OnPositionClosed, NWinPositions>()
        .add_metric::<SimulationEnding, UpdateRelay, OnPositionClosed, NLossPositions>()
        .add_metric::<SimulationEnding, UpdateRelay, OnPositionClosed, WinRate>()
        //
        .add_metric::<SimulationEnding, UpdateRelay, OnPositionClosed, Cagr>()
        //
        .add_metric::<SimulationEnding, UpdateRelay, OnPositionClosed, CagrOverMaxDd>()
        .add_metric::<SimulationEnding, UpdateRelay, OnPositionClosed, Max<Drawdown>>()
        //
        .add_metric::<SimulationEnding, UpdateRelay, OnPositionClosed, CagrOverMeanDd>()
        .add_metric::<SimulationEnding, UpdateRelay, OnPositionClosed, Sum<Drawdown>>()
        .add_metric::<SimulationEnding, UpdateRelay, OnPositionClosed, Count<Drawdown>>()
        .add_metric::<SimulationEnding, UpdateRelay, OnPositionClosed, Mean<Drawdown>>()
        .finish()
        .add_system(engine_strategies::rsi::new(), Signal::new())
        .add_system(esl_utils::print_metric::new::<Balance>(), End::new())
        .add_system(esl_utils::print_metric::new::<MaxBalance>(), End::new())
        .add_system(esl_utils::print_metric::new::<MaxDrawdown>(), End::new())
        .add_system(esl_utils::print_metric::new::<NTrades>(), End::new())
        .add_system(esl_utils::print_metric::new::<Cagr>(), End::new())
        .add_system(esl_utils::print_metric::new::<CagrOverMaxDd>(), End::new())
        .add_system(esl_utils::print_metric::new::<CagrOverMeanDd>(), End::new())
        .build();
    ecs.run();
    let elapsed_ns = now.elapsed().as_nanos();
    println!(
        "Elapsed: {} ms, {} candles/s",
        elapsed_ns / 1_000_000,
        input_len as f32 * 1_000_000_000. / elapsed_ns as f32,
    );
    Ok(())
}
