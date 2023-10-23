#![feature(type_name_of_val)]
#![deny(unused_must_use)]
use core::marker::PhantomData;
use std::time::Instant;

use ::config::CONFIG;
use esl::{
    block_relays::UpdateRelay,
    inputs::orderflow::*,
    orderflow::TypeMaskResource,
    simulation_relays::SimulationEnding,
    stages::{End, Signal},
    *,
};
use esl_utils::{mssql::load_orderflow, plot_tracks, print_metric};
use mouse::{
    prelude::*,
    time::{DateTime, NaiveDateTime, Utc},
};

pub struct ExampleOrderflowStrategyState {
    prev_amount: f32,
    close_timestamp_ns: u64,
}

#[strategy]
/// Opens a position if the size of a trade is `amount_multiplier` bigger than previous trade in
/// the same direction as this trade. Then holds the position for `hold_ns` nanoseconds.
fn example_orderflow_strategy(
    // Some state, `In` allows to pass the value while constructing the system.
    mut state: In<ExampleOrderflowStrategyState>,
    mask: TypeMask,
    amount: Amount,
    timestamp: TimestampNs,
    mut orders: Orders,
    amount_multiplier: Param![1..100, 1],
    hold_ns: Param![1..60_000_000_000, 1_000_000],
    position: Metric<Position>,
) {
    let position = position.metric();
    if *timestamp as u64 >= state.close_timestamp_ns {
        if position.is_long() {
            orders.send(MarketCloseLong::full());
        } else {
            orders.send(MarketCloseShort::full());
        }
        state.close_timestamp_ns = 0;
    }
    let condition = *amount > state.prev_amount * amount_multiplier.get() && position.is_closed();
    let buy = mask.buy && condition;
    let sell = mask.sell && condition;
    orders.on(buy, MarketOpenLong::full());
    orders.on(sell, MarketOpenShort::full());
    if buy || sell {
        state.close_timestamp_ns = *timestamp as u64 + hold_ns.get() as u64;
    }
    state.prev_amount = *amount;
}

#[tokio::main]
async fn main() -> Result<()> {
    unsafe {
        // IMPORTANT: Change `cache_dir` in config.yaml to a valid path.
        ::config::load("config.yaml")?;
    }
    warn!("Logging works!");
    // Loads data from the filesystem. If it isn't present then it downloads data from SQL server.
    // If the start time is different from the time of the first tick then a query will be run to
    // check if there is more data. That query takes around 5 seconds to execute. Same happens for
    // the end time.
    let start =
        NaiveDateTime::parse_from_str("2022-12-12 14:31:14.0000000", "%Y-%m-%d %H:%M:%S%.f")?;
    let end = NaiveDateTime::parse_from_str("2022-12-15 14:31:14.0000000", "%Y-%m-%d %H:%M:%S%.f")?;
    let start = DateTime::from_utc(start, Utc);
    let end = DateTime::from_utc(end, Utc);
    // Credentials are stored in config file. Config example is in the root of the project.
    let map = load_orderflow(
        CONFIG
            .mssql
            .as_ref()
            .expect("missing `mssql` entry inside config"),
        "[TradingApp-Algoseek-ES].dbo.ESZ2",
        start..end,
        512 * (1 << 20),
    )
    .await?;
    let orderflow = map
        .slice()
        .iter()
        .filter(|x| {
            use packed_struct::PackedStruct;
            let mask = TypeMaskResource::unpack(&[x.type_mask as u8]).unwrap();
            // There are a lot of Quote updates, type mask 97 and 161
            // mask.ty == orderflow::MessageType::Quote
            mask.ty == orderflow::MessageType::Trade
        })
        .cloned()
        .collect::<Vec<_>>();
    dbg!(orderflow.first(), orderflow.last());
    let input_len = orderflow.len();
    let mut metrics = vec![0u8; 1024 * 1024];
    let n_trackers = 20;
    let to_alloc = 4 * n_trackers * input_len;
    println!(
        "Allocating {} MiB for tracks",
        to_alloc as f32 / 1024. / 1024.
    );
    let mut tracks = vec![0u8; to_alloc];
    let now = Instant::now();
    let state = ExampleOrderflowStrategyState {
        prev_amount: orderflow[0].amount,
        close_timestamp_ns: 0,
    };
    let builder = EcsBuilderStruct::new::<_, 6>(
        esl::stages::BacktestSchedule::builder(),
        EntitiesBuilderStruct1::new(),
    );
    MetricsBuilderStruct::new(builder)
        .add_tracker::<SimulationEnding, UpdateRelay, OnPositionClosed, Balance>()
        .add_tracker::<SimulationEnding, UpdateRelay, OnPositionClosed, MaxBalance>()
        .add_tracker::<SimulationEnding, UpdateRelay, OnPositionClosed, Drawdown>()
        .add_metric::<SimulationEnding, UpdateRelay, OnPositionClosed, NTrades>()
        //
        .add_metric::<SimulationEnding, UpdateRelay, OnPositionClosed, CagrOverMeanDd>()
        .add_metric::<SimulationEnding, UpdateRelay, OnPositionClosed, Count<Drawdown>>()
        .add_metric::<SimulationEnding, UpdateRelay, OnPositionClosed, Sum<Drawdown>>()
        .add_metric::<SimulationEnding, UpdateRelay, OnPositionClosed, Mean<Drawdown>>()
        .add_metric::<SimulationEnding, UpdateRelay, OnPositionClosed, Cagr>()
        .finish()
        .add_config::<example_orderflow_strategy::amount_multiplier, Entity0, _>(ParamConfig(32.))
        .add_config::<example_orderflow_strategy::hold_ns, Entity0, _>(ParamConfig(60. * 1e9 * 2.))
        .add_system(example_orderflow_strategy::new(state), Signal::new())
        // Using without plugin to shave off 3 seconds of compile time.
        .add_system_without_plugin(plot_tracks::new::<Balance>(), End::new())
        .add_system_without_plugin(plot_tracks::new::<MaxBalance>(), End::new())
        .add_system_without_plugin(plot_tracks::new::<Drawdown>(), End::new())
        .add_system_without_plugin(print_metric::new::<CagrOverMeanDd>(), End::new())
        .add_resource(MetricsPtr(metrics.as_mut_ptr()))
        .add_resource(TracksPtr(tracks.as_mut_ptr()))
        // Used to compute elapsed time for CAGR
        .add_resource(StartTimestampNs(orderflow[0].timestamp_ns as u64))
        .add_resource(AccountsPerThread(1))
        .add_resource(ThreadsPerDevice(1))
        .add_resource(ThreadId(0))
        .add_resource(NSamples(input_len))
        .add_resource(RiskFreeRate(0.))
        .add_resource(TradingDaysPerYear(252.))
        .add_plugin(CorePlugin {
            loop_end_bound_excluded: input_len,
        })
        .add_plugin(OrderflowBacktestPlugin {
            backtest_plugin: BacktestPlugin {
                inputs: unsafe {
                    Series::<orderflow::OrderflowInputNest>::new(orderflow.as_ptr() as *const u8)
                },
                starting_balance: 1.0,
                slippage: types::Slippage::Absolute(0.25),
                fee: types::Fee::RelativeToVolume(0.),
                inputs_marker: PhantomData::<orderflow::OrderflowInput>,
            },
        })
        .build()
        .run();
    let elapsed_ns = now.elapsed().as_nanos();
    println!(
        "Elapsed: {} ms, {} candles/s",
        elapsed_ns / 1_000_000,
        input_len as f32 * 1_000_000_000. / elapsed_ns as f32,
    );
    Ok(())
}
