//! Used to turn on a performance metric based on the stage of a simulation.
use super::*;

/// Adds a performance metric if the simulation is ending.
pub struct SimulationEnding;
/// Not implemented.
pub struct BacktestEnding;
/// Not implemented.
pub struct BlockEnding;

macro_rules! impl_simulation_relay {
    ($required:ident, $actual:ident) => {
        impl SimulationRelay<$required> for $actual {
            type AddMetric<
                B: MetricsBuilder,
                Relay: MetricsBuilderRelay<M::ExecutionOrder>,
                C: Condition,
                M: MetricTrait,
            > = impl MetricsBuilder;
            type AddTracker<
                B: MetricsBuilder,
                Relay: MetricsBuilderRelay<M::ExecutionOrder>,
                C: Condition,
                M: MetricTrait,
            > = impl MetricsBuilder;

            #[inline(always)]
            fn add_metric<B, Relay, C, M>(builder: B) -> Self::AddMetric<B, Relay, C, M>
            where
                B: MetricsBuilder,
                Relay: MetricsBuilderRelay<M::ExecutionOrder>,
                C: Condition,
                M: MetricTrait,
            {
                Relay::add_metric::<B, C, M>(builder)
            }

            #[inline(always)]
            fn add_tracker<B, Relay, C, M>(builder: B) -> Self::AddTracker<B, Relay, C, M>
            where
                B: MetricsBuilder,
                Relay: MetricsBuilderRelay<M::ExecutionOrder>,
                C: Condition,
                M: MetricTrait,
            {
                Relay::add_tracker::<B, C, M>(builder)
            }
        }
    };
}

macro_rules! impl_simulation_relay_skip {
    ($required:ident, $actual:ident) => {
        impl SimulationRelay<$required> for $actual {
            type AddMetric<
                B: MetricsBuilder,
                Relay: MetricsBuilderRelay<M::ExecutionOrder>,
                C: Condition,
                M: MetricTrait,
            > = impl MetricsBuilder;
            type AddTracker<
                B: MetricsBuilder,
                Relay: MetricsBuilderRelay<M::ExecutionOrder>,
                C: Condition,
                M: MetricTrait,
            > = impl MetricsBuilder;

            #[inline(always)]
            fn add_metric<B, Relay, C, M>(builder: B) -> Self::AddMetric<B, Relay, C, M>
            where
                B: MetricsBuilder,
                Relay: MetricsBuilderRelay<M::ExecutionOrder>,
                C: Condition,
                M: MetricTrait,
            {
                Relay::skip::<B, M>(builder)
            }

            #[inline(always)]
            fn add_tracker<B, Relay, C, M>(builder: B) -> Self::AddTracker<B, Relay, C, M>
            where
                B: MetricsBuilder,
                Relay: MetricsBuilderRelay<M::ExecutionOrder>,
                C: Condition,
                M: MetricTrait,
            {
                Relay::skip::<B, M>(builder)
            }
        }
    };
}

impl_simulation_relay!(SimulationEnding, SimulationEnding);
impl_simulation_relay!(BacktestEnding, SimulationEnding);
impl_simulation_relay!(BlockEnding, SimulationEnding);

impl_simulation_relay_skip!(SimulationEnding, BacktestEnding);
impl_simulation_relay!(BacktestEnding, BacktestEnding);
impl_simulation_relay!(BlockEnding, BacktestEnding);

impl_simulation_relay_skip!(SimulationEnding, BlockEnding);
impl_simulation_relay_skip!(BacktestEnding, BlockEnding);
impl_simulation_relay!(BlockEnding, BlockEnding);

impl SimulationRelayMarker for SimulationEnding {}
impl SimulationRelayMarker for BacktestEnding {}
impl SimulationRelayMarker for BlockEnding {}
