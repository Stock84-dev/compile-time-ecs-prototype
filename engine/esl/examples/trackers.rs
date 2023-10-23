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
use esl_utils::plot_tracks;
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
    let count = 1633869265 - 1443184345;
    let hlcvs_mapped = load_hlcv("BitMEX", "XBTUSD", start_ts, count).await?;
    let timeframe_s = 60 * 60;
    let hlcvs = esl_utils::change_timeframe(&hlcvs_mapped, timeframe_s);
    // let close = hlcvs.iter().map(|x| x.close).collect::<Vec<_>>();
    let input_len = hlcvs.len();
    let mut metrics = vec![0u8; 1024 * 1024];
    // How many trackers will be added. Make sure that this is bigger than the number of methods
    // below, otherwise the program will crash.
    let n_trackers = 20;
    let to_alloc = 4 * n_trackers * hlcvs.len();
    println!(
        "Allocating {} MiB for tracks",
        to_alloc as f32 / 1024. / 1024.
    );
    let mut tracks = vec![0u8; to_alloc];
    let now = Instant::now();
    let schedule = esl::stages::BacktestSchedule::builder();
    let builder = EcsBuilderStruct::new::<_, 6>(schedule, EntitiesBuilderStruct1::new());

    let mut ecs = MetricsBuilderStruct::new(builder)
        // Trackers store a performance metric which can later be used to plot them.
        .add_tracker::<SimulationEnding, UpdateRelay, OnPositionClosed, Balance>()
        .add_tracker::<SimulationEnding, UpdateRelay, OnPositionClosed, MaxBalance>()
        .add_tracker::<SimulationEnding, UpdateRelay, OnPositionClosed, Sum<MaxBalance>>()
        .add_tracker::<SimulationEnding, UpdateRelay, OnPositionClosed, Drawdown>()
        .add_tracker::<SimulationEnding, UpdateRelay, OnPositionClosed, MaxDrawdown>()
        // This is required every time a tracker is used.
        .add_tracker::<SimulationEnding, UpdateRelay, OnPositionClosed, NTrades>()
        .add_tracker::<SimulationEnding, UpdateRelay, OnPositionClosed, BalanceDelta>()
        .add_tracker::<SimulationEnding, UpdateRelay, OnPositionClosed, BalanceDeltaRel>()
        .add_tracker::<SimulationEnding, UpdateRelay, OnPositionClosed, NWinPositions>()
        .add_tracker::<SimulationEnding, UpdateRelay, OnPositionClosed, NLossPositions>()
        .add_tracker::<SimulationEnding, UpdateRelay, OnPositionClosed, WinRate>()
        // //
        .add_tracker::<SimulationEnding, UpdateRelay, OnPositionClosed, Cagr>()
        // //
        .add_tracker::<SimulationEnding, UpdateRelay, OnPositionClosed, CagrOverMaxDd>()
        .add_metric::<SimulationEnding, UpdateRelay, OnPositionClosed, Max<Drawdown>>()
        //
        .add_tracker::<SimulationEnding, UpdateRelay, OnPositionClosed, CagrOverMeanDd>()
        .add_metric::<SimulationEnding, UpdateRelay, OnPositionClosed, Sum<Drawdown>>()
        .add_metric::<SimulationEnding, UpdateRelay, OnPositionClosed, Count<Drawdown>>()
        .add_metric::<SimulationEnding, UpdateRelay, OnPositionClosed, Mean<Drawdown>>()
        //
        .add_tracker::<SimulationEnding, UpdateRelay, OnPositionClosed, ProfitFactor>()
        .add_metric::<SimulationEnding, UpdateRelay, OnPositionClosed, Sum<Profit>>()
        .add_metric::<SimulationEnding, UpdateRelay, OnPositionClosed, Sum<Loss>>()
        .add_metric::<SimulationEnding, UpdateRelay, OnPositionClosed, Profit>()
        .add_metric::<SimulationEnding, UpdateRelay, OnPositionClosed, Loss>()
        //
        .add_tracker::<SimulationEnding, UpdateRelay, OnPositionClosed, NormalizedProfitFactor>()
        .add_metric::<SimulationEnding, UpdateRelay, OnPositionClosed, Sum<ProfitRel>>()
        .add_metric::<SimulationEnding, UpdateRelay, OnPositionClosed, Sum<LossRel>>()
        .add_metric::<SimulationEnding, UpdateRelay, OnPositionClosed, ProfitRel>()
        .add_metric::<SimulationEnding, UpdateRelay, OnPositionClosed, LossRel>()
        //
        .add_tracker::<SimulationEnding, UpdateRelay, OnPositionClosed, ExpectedPayoff>()
        //
        .add_tracker::<SimulationEnding, UpdateRelay, OnPositionClosed, ReturnY>()
        //
        .add_tracker::<SimulationEnding, UpdateRelay, OnPositionClosed, SharpeRatio>()
        .add_metric::<SimulationEnding, UpdateRelay, OnPositionClosed, Stddev<BalanceDeltaRel>>()
        .add_metric::<SimulationEnding, UpdateRelay, OnPositionClosed, Count<BalanceDeltaRel>>()
        .add_metric::<SimulationEnding, UpdateRelay, OnPositionClosed, Mean<BalanceDeltaRel>>()
        //
        .add_tracker::<SimulationEnding, UpdateRelay, OnPositionClosed, SortinoRatio>()
        .add_metric::<SimulationEnding, UpdateRelay, OnPositionClosed, Stddev<LossRel>>()
        .add_metric::<SimulationEnding, UpdateRelay, OnPositionClosed, Count<LossRel>>()
        .add_metric::<SimulationEnding, UpdateRelay, OnPositionClosed, Mean<LossRel>>()
        .finish()
        .add_resource(MetricsPtr(metrics.as_mut_ptr()))
        .add_resource(TracksPtr(tracks.as_mut_ptr()))
        .add_resource(AccountsPerThread(1))
        .add_resource(ThreadsPerDevice(1))
        .add_resource(ThreadId(0))
        // How many samples will be generated from this backtest. Not known in advance, so it's set
        // to max possible value.
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
        .add_config::<engine_strategies::rsi::hline, Entity0, _>(ParamConfig(6.))
        .add_system(engine_strategies::rsi::new(), Signal::new())
        // Using without plugin to shave off 3 seconds of compile time.
        .add_system_without_plugin(plot_tracks::new::<Balance>(), End::new())
        .add_system_without_plugin(plot_tracks::new::<MaxBalance>(), End::new())
        .add_system_without_plugin(plot_tracks::new::<Drawdown>(), End::new())
        .add_system_without_plugin(print::new(), End::new())
        .add_system_without_plugin(plot_tracks::new::<MaxDrawdown>(), End::new())
        .add_system_without_plugin(plot_tracks::new::<NTrades>(), End::new())
        .add_system_without_plugin(plot_tracks::new::<BalanceDelta>(), End::new())
        .add_system_without_plugin(plot_tracks::new::<BalanceDeltaRel>(), End::new())
        .add_system_without_plugin(plot_tracks::new::<NWinPositions>(), End::new())
        .add_system_without_plugin(plot_tracks::new::<NLossPositions>(), End::new())
        .add_system_without_plugin(plot_tracks::new::<WinRate>(), End::new())
        .add_system_without_plugin(plot_tracks::new::<Cagr>(), End::new())
        .add_system_without_plugin(plot_tracks::new::<CagrOverMaxDd>(), End::new())
        .add_system_without_plugin(plot_tracks::new::<ProfitFactor>(), End::new())
        .add_system_without_plugin(plot_tracks::new::<NormalizedProfitFactor>(), End::new())
        .add_system_without_plugin(plot_tracks::new::<ExpectedPayoff>(), End::new())
        .add_system_without_plugin(plot_tracks::new::<ReturnY>(), End::new())
        .add_system_without_plugin(plot_tracks::new::<CagrOverMeanDd>(), End::new())
        .add_system_without_plugin(plot_tracks::new::<SharpeRatio>(), End::new())
        .add_system_without_plugin(plot_tracks::new::<SortinoRatio>(), End::new())
        .build();
    ecs.run();
    let elapsed_ns = now.elapsed().as_nanos();
    println!(
        "Elapsed: {} ms, {} candles/s",
        elapsed_ns / 1_000_000,
        input_len as f32 * 1_000_000_000. / elapsed_ns as f32,
    );
    println!("{}", metrics[0]);
    // Elapsed: 70 ms, 754304.75 candles/s
    // With all the metrics and trackers
    // Elapsed: 18317 ms, 173495.55 candles/s
    Ok(())
}

#[system]
fn print(tracks: Tracks<Drawdown>) {
    println!("{:?}", tracks.tracks()[0]);
}
