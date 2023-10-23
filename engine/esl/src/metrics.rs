//! To access a metric from a system use `Metric<T>`.
//! All performance metrics aren't in percentages for performance reasons. Multiply them by 100 to
//! get the percentage.
//! `Balance` must always be included.
//! If the type name doesn't include `Rel` then it is in absolute terms, otherwise it is in
//! relative terms. To get percentages multiply by 100.
use core::marker::PhantomData;
use std::prelude::v1::*;

use ergnomics::*;
use inception::*;
use num_traits::{Float, FromPrimitive};

use crate::{execution_order::*, simulation_relays::*, types::Direction, value::Value, *};

macro_rules! impl_blanket_metric {
    ($metric:ident, $relay:ident, $inner:ident) => {
        #[metric]
        #[derive(Default)]
        pub struct $metric($inner);
        #[impl_metric]
        impl MetricTrait for $metric {
            type ExecutionOrder = Order0;
            type SimulationRelay = $relay;

            #[inline(always)]
            fn update() {
                // handled by plugins
            }
        }
    };
}

impl_blanket_metric!(Balance, SimulationEnding, f32);
impl_blanket_metric!(Position, BacktestEnding, f32);
impl_blanket_metric!(NOrders, SimulationEnding, u32);
impl_blanket_metric!(EntryPrice, BacktestEnding, f32);
impl_blanket_metric!(ExitPrice, BacktestEnding, f32);

impl Position {
    #[inline(always)]
    pub fn is_closed(&self) -> bool {
        self.get() == 0.0
    }

    #[inline(always)]
    pub fn is_opened(&self) -> bool {
        !self.is_closed()
    }

    #[inline(always)]
    pub fn direction(&self) -> Option<Direction> {
        let position = self.get();
        if position > 0.0 {
            Some(Direction::Long)
        } else if position < 0.0 {
            Some(Direction::Short)
        } else {
            None
        }
    }

    #[inline(always)]
    pub fn is_long(&self) -> bool {
        self.get() > 0.
    }

    #[inline(always)]
    pub fn is_short(&self) -> bool {
        self.get() > 0.
    }
}

macro_rules! max_metric {
    ($max:ident, $cur:tt, $order:ty, $simulation:ty) => {
        #[metric]
        #[derive(Default)]
        pub struct $max(f32);
        #[impl_metric]
        impl MetricTrait for $max {
            type ExecutionOrder = $order;
            type SimulationRelay = $simulation;

            #[inline(always)]
            fn update(mut max_balance: Metric<$max>, balance: Metric<$cur>) {
                (*max_balance).max_mut(*balance);
            }
        }
    };
}

max_metric!(MaxBalance, Balance, Order1, SimulationEnding);

#[metric]
#[derive(Default, Debug)]
pub struct Drawdown(f32);
#[impl_metric]
impl MetricTrait for Drawdown {
    type SimulationRelay = SimulationEnding;

    #[inline(always)]
    fn update(
        mut drawdown: Metric<Drawdown>,
        max_balance: Metric<MaxBalance>,
        balance: Metric<Balance>,
    ) {
        *drawdown = (*max_balance - *balance) / *max_balance;
    }
}

max_metric!(MaxDrawdown, Drawdown, Order2, SimulationEnding);

#[metric]
#[derive(Default)]
pub struct NTrades(u32);
#[impl_metric]
impl MetricTrait for NTrades {
    type SimulationRelay = SimulationEnding;

    #[inline(always)]
    fn update(mut n_trades: Metric<NTrades>, events: EntityEvents<OrderExecuted>) {
        *n_trades += events.len() as u32;
    }
}

#[metric]
#[derive(Default)]
pub struct PrevBalance(f32);
#[impl_metric]
impl MetricTrait for PrevBalance {
    type SimulationRelay = SimulationEnding;

    #[inline(always)]
    fn update(mut prev_balance: Metric<PrevBalance>, balance: Metric<Balance>) {
        *prev_balance = *balance;
    }
}

#[metric]
#[derive(Default)]
pub struct BalanceDelta(f32);
#[impl_metric]
impl MetricTrait for BalanceDelta {
    type SimulationRelay = BacktestEnding;

    #[inline(always)]
    fn update(
        mut profit: Metric<BalanceDelta>,
        prev_balance: Metric<PrevBalance>,
        balance: Metric<Balance>,
    ) {
        *profit = *balance - *prev_balance;
    }
}

#[metric]
#[derive(Default)]
pub struct BalanceDeltaRel(f32);
#[impl_metric]
impl MetricTrait for BalanceDeltaRel {
    type SimulationRelay = BacktestEnding;

    #[inline(always)]
    fn update(
        mut delta_rel: Metric<BalanceDeltaRel>,
        delta: Metric<BalanceDelta>,
        prev_balance: Metric<PrevBalance>,
    ) {
        *delta_rel = *delta / *prev_balance;
    }
}

#[metric]
#[derive(Default)]
pub struct Profit(f32);
#[impl_metric]
impl MetricTrait for Profit {
    type SimulationRelay = BacktestEnding;

    #[inline(always)]
    fn update(mut profit: Metric<Profit>, delta: Metric<BalanceDelta>) {
        if *delta > 0.0 {
            *profit = *delta;
        } else {
            *profit = 0.;
        }
    }
}

#[metric]
#[derive(Default)]
pub struct Loss(f32);
#[impl_metric]
impl MetricTrait for Loss {
    type SimulationRelay = BacktestEnding;

    #[inline(always)]
    fn update(mut loss: Metric<Loss>, delta: Metric<BalanceDelta>) {
        if *delta < 0.0 {
            *loss = delta.abs();
        } else {
            *loss = 0.;
        }
    }
}

#[metric]
#[derive(Default)]
pub struct ProfitRel(f32);
#[impl_metric]
impl MetricTrait for ProfitRel {
    type SimulationRelay = BacktestEnding;

    #[inline(always)]
    fn update(mut profit: Metric<ProfitRel>, delta: Metric<BalanceDeltaRel>) {
        if *delta > 0.0 {
            *profit = *delta;
        } else {
            *profit = 0.;
        }
    }
}

#[metric]
#[derive(Default)]
pub struct LossRel(f32);
#[impl_metric]
impl MetricTrait for LossRel {
    type SimulationRelay = BacktestEnding;

    #[inline(always)]
    fn update(mut loss: Metric<LossRel>, delta: Metric<BalanceDeltaRel>) {
        if *delta < 0.0 {
            *loss = delta.abs();
        } else {
            *loss = 0.;
        }
    }
}

#[metric]
#[derive(Default)]
pub struct NWinPositions(u32);
#[impl_metric]
impl MetricTrait for NWinPositions {
    type SimulationRelay = SimulationEnding;

    #[inline(always)]
    fn update(
        mut n_win_positions: Metric<NWinPositions>,
        delta: Metric<BalanceDelta>,
        events: EntityEvents<PositionClosed>,
    ) {
        if *delta > 0.0 {
            *n_win_positions += events.len() as u32;
        }
    }
}

#[metric]
#[derive(Default)]
pub struct NLossPositions(u32);
#[impl_metric]
impl MetricTrait for NLossPositions {
    type SimulationRelay = SimulationEnding;

    #[inline(always)]
    fn update(
        mut n_loss_positions: Metric<NLossPositions>,
        delta: Metric<BalanceDelta>,
        events: EntityEvents<PositionClosed>,
    ) {
        if *delta < 0.0 {
            *n_loss_positions += events.len() as u32;
        }
    }
}

#[metric]
#[derive(Default)]
pub struct WinRate(f32);
#[impl_metric]
impl MetricTrait for WinRate {
    type SimulationRelay = SimulationEnding;

    #[inline(always)]
    fn update(
        mut win_rate: Metric<WinRate>,
        n_win_trades: Metric<NWinPositions>,
        n_trades: Metric<NTrades>,
    ) {
        *win_rate = *n_win_trades as f32 / *n_trades as f32;
    }
}

#[metric]
#[derive(Default)]
pub struct Cagr(f32);
#[impl_metric]
impl MetricTrait for Cagr {
    type SimulationRelay = SimulationEnding;

    #[inline(always)]
    fn update(
        mut cagr: Metric<Cagr>,
        starting_balance: Res<StartingBalance>,
        balance: Metric<Balance>,
        duration_ms: Res<Elapsed>,
    ) {
        *cagr = (*balance / starting_balance.0).powf(1. / duration_ms.years()) - 1.;
    }
}

#[metric]
#[derive(Default)]
pub struct Sum<M: MetricTrait>(M::Value);
#[impl_metric]
impl<M: MetricTrait> MetricTrait for Sum<M> {
    type ExecutionOrder = <M::ExecutionOrder as ExecutionOrder>::Next;
    type SimulationRelay = M::SimulationRelay;

    #[inline(always)]
    fn update(mut sum: Metric<Sum<M>>, metric: Metric<M>) {
        *sum = *sum + metric.get();
    }
}

#[metric]
#[derive(Default)]
pub struct Max<M: MetricTrait>(M::Value);
#[impl_metric]
impl<M: MetricTrait> MetricTrait for Max<M> {
    type ExecutionOrder = <M::ExecutionOrder as ExecutionOrder>::Next;
    type SimulationRelay = M::SimulationRelay;

    #[inline(always)]
    fn update(mut max: Metric<Max<M>>, metric: Metric<M>) {
        if max.get() < metric.get() {
            *max = metric.get();
        }
    }
}

#[metric]
#[derive(Default)]
pub struct Min<M: MetricTrait>(M::Value);
#[impl_metric]
impl<M: MetricTrait> MetricTrait for Min<M> {
    type ExecutionOrder = <M::ExecutionOrder as ExecutionOrder>::Next;
    type SimulationRelay = M::SimulationRelay;

    #[inline(always)]
    fn update(mut min: Metric<Min<M>>, metric: Metric<M>) {
        if min.get() > metric.get() {
            *min = metric.get();
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
#[repr(C)]
pub struct Count<M: MetricTrait>(u32, PhantomData<M>);
impl<M: MetricTrait> Value for Count<M> {
    type Value = u32;

    #[inline(always)]
    fn get(&self) -> Self::Value {
        self.0
    }
}
impl<M: MetricTrait> core::ops::Deref for Count<M> {
    type Target = u32;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<M: MetricTrait> core::ops::DerefMut for Count<M> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl<M: MetricTrait> Default for Count<M> {
    #[inline(always)]
    fn default() -> Self {
        Self(Default::default(), Default::default())
    }
}
#[impl_metric]
impl<M: MetricTrait> MetricTrait for Count<M> {
    type ExecutionOrder = <M::ExecutionOrder as ExecutionOrder>::Next;
    type SimulationRelay = M::SimulationRelay;

    #[inline(always)]
    fn update(mut count: Metric<Count<M>>) {
        *count += 1;
    }
}

#[metric]
#[derive(Default)]
pub struct Mean<M: MetricTrait>(M::Value);
#[impl_metric]
impl<M: MetricTrait> MetricTrait for Mean<M> {
    type ExecutionOrder = <<M::ExecutionOrder as ExecutionOrder>::Next as ExecutionOrder>::Next;
    type SimulationRelay = M::SimulationRelay;

    #[inline(always)]
    fn update(mut mean: Metric<Mean<M>>, sum: Metric<Sum<M>>, count: Metric<Count<M>>) {
        let count = <M::Value as FromPrimitive>::from_u32(count.get()).unwrap();
        *mean = *sum / count;
    }
}

#[metric]
#[derive(Default)]
pub struct Stddev<M: MetricTrait>(M::Value);
#[impl_metric]
impl<M: MetricTrait<Value = f32>> MetricTrait for Stddev<M> {
    type ExecutionOrder = <<Mean<M> as MetricTrait>::ExecutionOrder as ExecutionOrder>::Next;
    type SimulationRelay = M::SimulationRelay;

    #[inline(always)]
    fn update(
        mut stddev: Metric<Stddev<M>>,
        mean: Metric<Mean<M>>,
        tracks: Tracks<M>,
        count: Metric<Count<M>>,
    ) {
        let mut sum = M::Value::default();
        let tracks = tracks.tracks();
        for track in tracks.iter().take(count.get() as usize) {
            let diff = track.get() - mean.get();
            sum = sum + diff * diff;
        }
        let count = <M::Value as FromPrimitive>::from_u32(count.get()).unwrap();
        *stddev = (sum / count).sqrt();
    }
}

#[metric]
#[derive(Default)]
pub struct CoefficientOfCorrelation<M: MetricTrait>(M::Value);
#[impl_metric]
impl<M: MetricTrait<Value = f32>> MetricTrait for CoefficientOfCorrelation<M>
where
    MaxExecutionOrder<(Order0, <M as MetricTrait>::ExecutionOrder)>: ExecutionOrder,
{
    type ExecutionOrder =
        <<Sum<Squared<M>> as MetricTrait>::ExecutionOrder as ExecutionOrder>::Next;
    type SimulationRelay = M::SimulationRelay;

    #[inline(always)]
    fn update(
        mut r: Metric<CoefficientOfCorrelation<M>>,
        sum_xy: Metric<Sum<Mul<DurationS, M>>>,
        sum_x: Metric<Sum<DurationS>>,
        sum_y: Metric<Sum<M>>,
        sum_x2: Metric<Sum<Squared<DurationS>>>,
        sum_y2: Metric<Sum<Squared<M>>>,
        n: Metric<Count<M>>,
    ) {
        let n = M::Value::from_u32(*n).unwrap();
        *r = (n * *sum_xy - *sum_x * *sum_y)
            / ((n * *sum_x2 - *sum_x * *sum_x) * (n * *sum_y2 - *sum_y * *sum_y)).sqrt();
    }
}

#[derive(Clone, Copy, PartialEq)]
#[repr(C)]
pub struct Mul<M0: MetricTrait, M1: MetricTrait<Value = M0::Value>>(M0::Value, PhantomData<M1>);
impl<M0: MetricTrait, M1: MetricTrait<Value = M0::Value>> Value for Mul<M0, M1> {
    type Value = M0::Value;

    #[inline(always)]
    fn get(&self) -> Self::Value {
        self.0
    }
}
impl<M0: MetricTrait, M1: MetricTrait<Value = M0::Value>> core::ops::Deref for Mul<M0, M1> {
    type Target = M0::Value;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<M0: MetricTrait, M1: MetricTrait<Value = M0::Value>> core::ops::DerefMut for Mul<M0, M1> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl<M0: MetricTrait, M1: MetricTrait<Value = M0::Value>> Default for Mul<M0, M1> {
    #[inline(always)]
    fn default() -> Self {
        Self(Default::default(), Default::default())
    }
}
#[impl_metric]
impl<M0: MetricTrait, M1: MetricTrait<Value = M0::Value>> MetricTrait for Mul<M0, M1>
where
    MaxExecutionOrder<(M0::ExecutionOrder, M1::ExecutionOrder)>: ExecutionOrder,
{
    type ExecutionOrder =
        <MaxExecutionOrder<(M0::ExecutionOrder, M1::ExecutionOrder)> as ExecutionOrder>::Next;
    type SimulationRelay = M0::SimulationRelay;

    #[inline(always)]
    fn update(mut mul: Metric<Mul<M0, M1>>, a: Metric<M0>, b: Metric<M1>) {
        *mul = a.get() * b.get();
    }
}

#[metric]
#[derive(Default)]
pub struct Squared<M: MetricTrait>(M::Value);
#[impl_metric]
impl<M: MetricTrait> MetricTrait for Squared<M> {
    type ExecutionOrder = <M::ExecutionOrder as ExecutionOrder>::Next;
    type SimulationRelay = M::SimulationRelay;

    #[inline(always)]
    fn update(mut squared: Metric<Squared<M>>, metric: Metric<M>) {
        *squared = metric.get() * metric.get();
    }
}

#[metric]
#[derive(Default)]
pub struct CagrOverMeanDd(f32);
#[impl_metric]
impl MetricTrait for CagrOverMeanDd {
    type SimulationRelay = SimulationEnding;

    #[inline(always)]
    fn update(
        mut cagr_over_mean_dd: Metric<CagrOverMeanDd>,
        cagr: Metric<Cagr>,
        mean_dd: Metric<Mean<Drawdown>>,
    ) {
        *cagr_over_mean_dd = *cagr / *mean_dd;
    }
}

#[metric]
#[derive(Default)]
pub struct CagrOverMaxDd(f32);
#[impl_metric]
impl MetricTrait for CagrOverMaxDd {
    type SimulationRelay = SimulationEnding;

    #[inline(always)]
    fn update(
        mut cagr_over_max_dd: Metric<CagrOverMaxDd>,
        cagr: Metric<Cagr>,
        max_dd: Metric<Max<Drawdown>>,
    ) {
        *cagr_over_max_dd = *cagr / *max_dd;
    }
}

#[metric]
#[derive(Default)]
pub struct ProfitFactor(f32);
#[impl_metric]
impl MetricTrait for ProfitFactor {
    type SimulationRelay = SimulationEnding;

    // TODO: can this be optimized to use balance and starting balance
    #[inline(always)]
    fn update(
        mut profit_factor: Metric<ProfitFactor>,
        profits: Metric<Sum<Profit>>,
        losses: Metric<Sum<Loss>>,
    ) {
        *profit_factor = *profits / *losses;
    }
}

#[metric]
#[derive(Default)]
pub struct NormalizedProfitFactor(f32);
#[impl_metric]
impl MetricTrait for NormalizedProfitFactor {
    type SimulationRelay = SimulationEnding;

    // TODO: can this be optimized to use balance and starting balance
    #[inline(always)]
    fn update(
        mut profit_factor: Metric<NormalizedProfitFactor>,
        profits: Metric<Sum<ProfitRel>>,
        losses: Metric<Sum<LossRel>>,
    ) {
        *profit_factor = *profits / *losses;
    }
}

#[metric]
#[derive(Default)]
pub struct DurationS(f32);
#[impl_metric]
impl MetricTrait for DurationS {
    type SimulationRelay = SimulationEnding;

    #[inline(always)]
    fn update(mut duration_s: Metric<DurationS>, duration: Res<Elapsed>) {
        *duration_s = duration.seconds();
    }
}

#[metric]
#[derive(Default)]
pub struct ExpectedPayoff(f32);
#[impl_metric]
impl MetricTrait for ExpectedPayoff {
    type SimulationRelay = SimulationEnding;

    // TODO: can this be optimized to use balance and starting balance
    #[inline(always)]
    fn update(
        mut expected_payoff: Metric<ExpectedPayoff>,
        profits: Metric<Sum<Profit>>,
        losses: Metric<Sum<Loss>>,
        n_trades: Metric<NTrades>,
    ) {
        *expected_payoff = (*profits - *losses) / *n_trades as f32;
    }
}

#[metric]
#[derive(Default)]
pub struct ReturnY(f32);
#[impl_metric]
impl MetricTrait for ReturnY {
    type SimulationRelay = SimulationEnding;

    #[inline(always)]
    fn update(
        mut return_y: Metric<ReturnY>,
        balance: Metric<Balance>,
        duration: Res<Elapsed>,
        starting_balance: Res<StartingBalance>,
    ) {
        *return_y = (*balance - starting_balance.0) / starting_balance.0 / duration.years();
    }
}

#[metric]
#[derive(Default)]
pub struct SharpeRatio(f32);
#[impl_metric]
impl MetricTrait for SharpeRatio {
    type SimulationRelay = SimulationEnding;

    #[inline(always)]
    fn update(
        mut sharpe_ratio: Metric<SharpeRatio>,
        return_y: Metric<ReturnY>,
        risk_free_rate: Res<RiskFreeRate>,
        stddev: Metric<Stddev<BalanceDeltaRel>>,
        duration: Res<Elapsed>,
        trading_days_per_year: Res<TradingDaysPerYear>,
    ) {
        let year = trading_days_per_year.0 * 24. * 60. * 60.;
        let ratio = year / duration.seconds();
        *sharpe_ratio = (*return_y - risk_free_rate.0) / *stddev * ratio.sqrt();
    }
}

#[metric]
#[derive(Default)]
pub struct SortinoRatio(f32);
#[impl_metric]
impl MetricTrait for SortinoRatio {
    type SimulationRelay = SimulationEnding;

    #[inline(always)]
    fn update(
        mut sharpe_ratio: Metric<SortinoRatio>,
        return_y: Metric<ReturnY>,
        risk_free_rate: Res<RiskFreeRate>,
        stddev: Metric<Stddev<LossRel>>,
    ) {
        *sharpe_ratio = (*return_y - risk_free_rate.0) / *stddev;
    }
}
