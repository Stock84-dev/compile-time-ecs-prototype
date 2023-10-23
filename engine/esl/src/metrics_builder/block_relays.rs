//! Used to decide wether a metric should be added to the `PostBlockX` stage that executed at the
//! end of the loop or the `PostTradeX` stage that executes inside the loop.
use super::*;

/// Puts a system on a stage (`PostTradeX`) that is called within the loop.
pub struct UpdateRelay;
/// Puts a system on a stage (`PostBlockX`) that is called after the loop.
pub struct BlockRelay;

macro_rules! impl_update_relay {
    ($order:ident, $metric:ident, $tracker:ident) => {
        impl MetricsBuilderRelay<execution_order::$order> for UpdateRelay {
            type AddMetric<B: MetricsBuilder, C: Condition, M: MetricTrait> = impl MetricsBuilder;
            type AddTracker<B: MetricsBuilder, C: Condition, M: MetricTrait> = impl MetricsBuilder;

            #[inline(always)]
            fn add_metric<B: MetricsBuilder, C: Condition, M: MetricTrait>(
                builder: B,
            ) -> Self::AddMetric<B, C, M> {
                builder.$metric::<C, M>()
            }

            #[inline(always)]
            fn add_tracker<B: MetricsBuilder, C: Condition, M: MetricTrait>(
                builder: B,
            ) -> Self::AddTracker<B, C, M> {
                builder.$tracker::<C, M>()
            }
        }
    };
}

macro_rules! impl_block_relay {
    ($order:ident, $metric:ident, $tracker:ident) => {
        impl MetricsBuilderRelay<execution_order::$order> for BlockRelay {
            type AddMetric<B: MetricsBuilder, C: Condition, M: MetricTrait> = impl MetricsBuilder;
            type AddTracker<B: MetricsBuilder, C: Condition, M: MetricTrait> = impl MetricsBuilder;

            #[inline(always)]
            fn add_metric<B: MetricsBuilder, C: Condition, M: MetricTrait>(
                builder: B,
            ) -> Self::AddMetric<B, C, M> {
                builder.$metric::<C, M>()
            }

            #[inline(always)]
            fn add_tracker<B: MetricsBuilder, C: Condition, M: MetricTrait>(
                builder: B,
            ) -> Self::AddTracker<B, C, M> {
                builder.$tracker::<C, M>()
            }
        }
    };
}

repeat!(
    impl_update_relay,
    0,
    9,
    Order,
    add_update_metric,
    add_update_tracker
);
repeat!(
    impl_block_relay,
    0,
    9,
    Order,
    add_block_metric,
    add_block_tracker
);
