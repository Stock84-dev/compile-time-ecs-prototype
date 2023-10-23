use core::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

use all_tuples::repeat;
pub use conditions::*;
use esl::*;

use crate::stages::*;

pub mod block_relays;
pub mod conditions;
pub mod simulation_relays;

pub mod execution_order {
    //! When to execute a system based on its dependencies. System with no dependencies has
    //! `Order0`, system with one dependency in a chain has `Order1`, etc. Handled automatically
    //! by `impl_metric` macro.
    use super::*;

    pub struct Order0;
    pub struct Order1;
    pub struct Order2;
    pub struct Order3;
    pub struct Order4;
    pub struct Order5;
    pub struct Order6;
    pub struct Order7;
    pub struct Order8;
    pub struct Order9;
    pub enum InvalidOrder {}

    impl ExecutionOrder for Order0 {
        type Next = Order1;
    }

    impl ExecutionOrder for Order1 {
        type Next = Order2;
    }

    impl ExecutionOrder for Order2 {
        type Next = Order3;
    }

    impl ExecutionOrder for Order3 {
        type Next = Order4;
    }

    impl ExecutionOrder for Order4 {
        type Next = Order5;
    }

    impl ExecutionOrder for Order5 {
        type Next = Order6;
    }

    impl ExecutionOrder for Order6 {
        type Next = Order7;
    }

    impl ExecutionOrder for Order7 {
        type Next = Order8;
    }

    impl ExecutionOrder for Order8 {
        type Next = Order9;
    }

    impl ExecutionOrder for Order9 {
        type Next = InvalidOrder;
    }

    impl ExecutionOrder for InvalidOrder {
        type Next = InvalidOrder;
    }

    pub struct MaxExecutionOrder<T>(PhantomData<T>);

    esl_macros::impl_max_execution_order!();
}

pub struct Metric<'w, 's, M, const N: usize> {
    component: &'w mut MetricComponent<M>,
    _marker: PhantomSystemParam<'s, 'w, N>,
}

impl<'w, 's, M, const N: usize> Metric<'w, 's, M, N> {
    #[inline(always)]
    pub fn metric(&self) -> &M {
        &self.component.metric
    }
}

impl<'w, 's, M: 'static, const N: usize> SystemParam for Metric<'w, 's, M, N> {
    // cast lifetimes
    type Item<'world, 'state, Wrld: World> = Metric<'world, 'state, M, N>;
    type State = ();

    type Build<B: EcsBuilder, SB: SystemParamNameMapper + 'static, ParamName: 'static> =
        impl EcsBuilder;

    unimpl_get_param!();

    #[inline(always)]
    fn get_param_for_entity<'world, 'state, Wrld, SB, ParamName, E>(
        entity: &'world mut E,
        _state: &'state mut Self::State,
        _world: &'world mut Wrld,
    ) -> Option<Self::Item<'world, 'state, Wrld>>
    where
        Wrld: World,
        SB: SystemParamNameMapper,
        E: EntityFetch,
    {
        Some(Metric {
            component: entity.get_component_mut()?,
            _marker: PhantomSystemParam::default(),
        })
    }

    #[inline(always)]
    fn build<B: EcsBuilder, SB: SystemParamNameMapper + 'static, ParamName: 'static>(
        builder: B,
    ) -> Self::Build<B, SB, ParamName> {
        builder
    }
}

impl<'w, 's, M: Value, const N: usize> Value for Metric<'w, 's, M, N> {
    type Value = M::Value;

    #[inline(always)]
    fn get(&self) -> Self::Value {
        self.component.get()
    }
}

impl<'w, 's, M: Deref, const N: usize> Deref for Metric<'w, 's, M, N> {
    type Target = M::Target;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        self.component.deref()
    }
}

impl<'w, 's, M: DerefMut, const N: usize> DerefMut for Metric<'w, 's, M, N> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.component.deref_mut()
    }
}

#[derive(Clone, Default)]
pub struct MetricComponent<M> {
    pub metric: M,
    tracker_id: usize,
}

impl<M: Default> MetricComponent<M> {
    #[inline(always)]
    pub fn new(tracker_id: usize) -> Self {
        Self {
            metric: M::default(),
            tracker_id,
        }
    }
}

impl<M: Value> Value for MetricComponent<M> {
    type Value = M::Value;

    #[inline(always)]
    fn get(&self) -> Self::Value {
        self.metric.get()
    }
}

impl<M: Deref> Deref for MetricComponent<M> {
    type Target = M::Target;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        self.metric.deref()
    }
}

impl<M: DerefMut> DerefMut for MetricComponent<M> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.metric.deref_mut()
    }
}

pub trait ExecutionOrder {
    type Next: ExecutionOrder;
}

#[system_param]
/// Records the value of the metric and stores it in memory for later access.
pub struct Tracks<M: MetricTrait> {
    n_samples: Res<NSamples>,
    n_trackers: Res<NTrackers>,
    tracks_ptr: Res<TracksPtr>,
    accounts_per_thread: Res<AccountsPerThread>,
    thread_id: Res<ThreadId>,
    threads_per_device: Res<ThreadsPerDevice>,
    sample_id: &mut SampleId,
    tracker: Metric<M>,
    sample_recorded: &mut SampleRecorded,
    entity: EntityParam,
}

impl<'w, 's, M: MetricTrait, const N: usize> Tracks<'w, 's, M, N> {
    #[inline(always)]
    pub fn push(&mut self) {
        let offset = self.track_offset() + self.sample_id.0 * core::mem::size_of::<M>();
        unsafe {
            let dest = self.tracks_ptr.0.add(offset);
            let dest = dest as *mut M;
            *dest = self.tracker.component.metric;
        }
        self.sample_recorded.0 = true;
    }

    #[inline(always)]
    pub fn tracks(&self) -> &[M] {
        let offset = self.track_offset();
        unsafe {
            let dest = self.tracks_ptr.0.add(offset);
            let dest = dest as *const M;
            core::slice::from_raw_parts(dest, self.n_samples.0)
        }
    }

    #[inline(always)]
    fn track_offset(&self) -> usize {
        let n_accounts = self.threads_per_device.0 * self.accounts_per_thread.0;
        let track_stride = self.n_samples.0 * core::mem::size_of::<M>();
        let thread_stride = self.n_trackers.0 * track_stride;
        let account_stride = n_accounts * thread_stride;
        let account_id = self.entity.0;
        let offset = account_id * account_stride
            + self.thread_id.0 * thread_stride
            + self.tracker.component.tracker_id * track_stride;
        // OPTME: have a pointer for each track condition to reduce wasted space, also have a
        // pointer for each track size
        debug_assert_eq!(
            core::mem::size_of::<M>(),
            4,
            "WIP: metric size must be 4 bytes"
        );
        offset
    }
}

// pub trait MetricPlugin<SimRelay, PosRelay> {
//     type MetricDeps: MetricPlugin<SimRelay, PosRelay>;
//     type TrackerDeps: MetricPlugin<SimRelay, PosRelay>;
//
//     type AddMetrics<C: Condition, B: MetricsBuilder>: MetricsBuilder;
//     type AddTrackers<C: Condition, B: MetricsBuilder>: MetricsBuilder;
//
//     fn add_metrics<C: Condition, B: MetricsBuilder>(
//         builder: B,
//     ) -> Self::Add<SimRelay, MetricRelay, PosRelay, C, B>;
//
//     fn add_trackers<
//         C: Condition,
//         B: MetricsBuilder,
//     >(
//         builder: B,
//     ) -> Self::AddTrackers<SimRelay, MetricRelay, PosRelay, C, B>;
// }
//
// impl<T0: MetricTrait> MetricPlugin for (T0,) {
//     type ExecutionOrder = execution_order::Order0;
//     type MetricDeps = (T0::MetricDeps,);
//     type SimulationRelay = simulation_relays::SimulationEnding;
//     type TrackerDeps = (T0::TrackerDeps,);
//
//     type AddMetrics<
//         SimRelay: SimulationRelay<Self::SimulationRelay>,
//         PosRelay: MetricsBuilderRelay<Self::ExecutionOrder>,
//         C: Condition,
//         B: MetricsBuilder,
//     > = impl MetricsBuilder;
//     type AddTrackers<
//         SimRelay: SimulationRelay<Self::SimulationRelay>,
//         PosRelay: MetricsBuilderRelay<Self::ExecutionOrder>,
//         C: Condition,
//         B: MetricsBuilder,
//     > = impl MetricsBuilder;
//
//     fn add_metrics<
//         SimRelay: SimulationRelay<Self::SimulationRelay>,
//         PosRelay: MetricsBuilderRelay<Self::ExecutionOrder>,
//         C: Condition,
//         B: MetricsBuilder,
//     >(
//         builder: B,
//     ) -> Self::Add<SimRelay, MetricRelay, PosRelay, C, B> {
//         Self::MetricDeps::add_metrics::<SimRelay, PosRelay, C, _>(builder)
//             .add_metric_without_deps::<SimRelay, PosRelay, C, T0>()
//     }
//
//     fn add_trackers<
//         SimRelay: SimulationRelay<Self::SimulationRelay>,
//         PosRelay: MetricsBuilderRelay<Self::ExecutionOrder>,
//         C: Condition,
//         B: MetricsBuilder,
//     >(
//         builder: B,
//     ) -> Self::AddTrackers<SimRelay, MetricRelay, PosRelay, C, B> {
//         Self::TrackerDeps::add_trackers::<SimRelay, PosRelay, C, _>(builder)
//             .add_tracker_without_deps::<SimRelay, PosRelay, C, T0>()
//     }
// }

pub trait SimulationRelayMarker {}
pub trait MetricTrait: Value + Default + Copy + 'static {
    type UpdateParams<'w, 's, W: World, const N: usize>: SystemParam;
    type ExecutionOrder: ExecutionOrder;
    // type MetricDeps: MetricPlugin;
    // type TrackerDeps: MetricPlugin;
    type SimulationRelay: SimulationRelayMarker;

    fn update<'w, 's, W: World, const N: usize>(params: Self::UpdateParams<'w, 's, W, N>);
}

pub trait MetricKindRelay {
    type Add<
        SimRelay: SimulationRelay<M::SimulationRelay>,
        PosRelay: MetricsBuilderRelay<M::ExecutionOrder>,
        C: Condition,
        M: MetricTrait,
        B: MetricsBuilder,
    >: MetricsBuilder;

    fn add<
        SimRelay: SimulationRelay<M::SimulationRelay>,
        PosRelay: MetricsBuilderRelay<M::ExecutionOrder>,
        C: Condition,
        M: MetricTrait,
        B: MetricsBuilder,
    >(
        builder: B,
    ) -> Self::Add<SimRelay, PosRelay, C, M, B>;
}
pub struct MetricRelay;

impl MetricKindRelay for MetricRelay {
    type Add<
        SimRelay: SimulationRelay<M::SimulationRelay>,
        PosRelay: MetricsBuilderRelay<M::ExecutionOrder>,
        C: Condition,
        M: MetricTrait,
        B: MetricsBuilder,
    > = B::AddMetric<SimRelay, PosRelay, C, M>;

    #[inline(always)]
    fn add<
        SimRelay: SimulationRelay<M::SimulationRelay>,
        PosRelay: MetricsBuilderRelay<M::ExecutionOrder>,
        C: Condition,
        M: MetricTrait,
        B: MetricsBuilder,
    >(
        builder: B,
    ) -> Self::Add<SimRelay, PosRelay, C, M, B> {
        builder.add_metric::<SimRelay, PosRelay, C, M>()
    }
}

pub struct TrackerRelay;

impl MetricKindRelay for TrackerRelay {
    type Add<
        SimRelay: SimulationRelay<M::SimulationRelay>,
        PosRelay: MetricsBuilderRelay<M::ExecutionOrder>,
        C: Condition,
        M: MetricTrait,
        B: MetricsBuilder,
    > = B::AddTracker<SimRelay, PosRelay, C, M>;

    #[inline(always)]
    fn add<
        SimRelay: SimulationRelay<M::SimulationRelay>,
        PosRelay: MetricsBuilderRelay<M::ExecutionOrder>,
        C: Condition,
        M: MetricTrait,
        B: MetricsBuilder,
    >(
        builder: B,
    ) -> Self::Add<SimRelay, PosRelay, C, M, B> {
        builder.add_tracker::<SimRelay, PosRelay, C, M>()
    }
}

pub trait SimulationRelay<Sim> {
    type AddMetric<
        B: MetricsBuilder,
        Relay: MetricsBuilderRelay<M::ExecutionOrder>,
        C: Condition,
        M: MetricTrait,
    >: MetricsBuilder;
    type AddTracker<
        B: MetricsBuilder,
        Relay: MetricsBuilderRelay<M::ExecutionOrder>,
        C: Condition,
        M: MetricTrait,
    >: MetricsBuilder;
    fn add_metric<B, Relay, C, M>(builder: B) -> Self::AddMetric<B, Relay, C, M>
    where
        B: MetricsBuilder,
        Relay: MetricsBuilderRelay<M::ExecutionOrder>,
        C: Condition,
        M: MetricTrait;
    fn add_tracker<B, Relay, C, M>(builder: B) -> Self::AddTracker<B, Relay, C, M>
    where
        B: MetricsBuilder,
        Relay: MetricsBuilderRelay<M::ExecutionOrder>,
        C: Condition,
        M: MetricTrait;
}

pub trait Condition: 'static {
    type Params<'w, 's, W: World, const N: usize>: SystemParam;
    fn run<'w, 's, W: World, const N: usize>(params: Self::Params<'w, 's, W, N>) -> Skip;
}

#[derive(PartialEq)]
pub enum Skip {
    True,
    False,
}

pub struct MetricsBuilderStruct<
    B: EcsBuilder,
    Mems,
    UM0,
    UM1,
    UM2,
    UM3,
    UM4,
    UM5,
    UM6,
    UM7,
    UM8,
    EM0,
    EM1,
    EM2,
    EM3,
    EM4,
    EM5,
    EM6,
    EM7,
    EM8,
> {
    builder: B,
    field_offset: usize,
    n_trackers: usize,
    mems: Mems,
    update_metrics0: UM0,
    update_metrics1: UM1,
    update_metrics2: UM2,
    update_metrics3: UM3,
    update_metrics4: UM4,
    update_metrics5: UM5,
    update_metrics6: UM6,
    update_metrics7: UM7,
    update_metrics8: UM8,
    block_metrics0: EM0,
    block_metrics1: EM1,
    block_metrics2: EM2,
    block_metrics3: EM3,
    block_metrics4: EM4,
    block_metrics5: EM5,
    block_metrics6: EM6,
    block_metrics7: EM7,
    block_metrics8: EM8,
}

macro_rules! def_metrics_and_trackers {
    (
        $update_metric:ident,
        $block_metric:ident,
        $update_tracker:ident,
        $block_tracker:ident,
        $update_metric_fn:ident,
        $block_metric_fn:ident,
        $update_tracker_fn:ident,
        $block_tracker_fn:ident
    ) => {
        type $update_metric<C: Condition, M: MetricTrait>: MetricsBuilder;
        type $block_metric<C: Condition, M: MetricTrait>: MetricsBuilder;
        type $update_tracker<C: Condition, M: MetricTrait>: MetricsBuilder;
        type $block_tracker<C: Condition, M: MetricTrait>: MetricsBuilder;

        fn $update_metric_fn<C: Condition, M: MetricTrait>(self) -> Self::$update_metric<C, M>;
        fn $block_metric_fn<C: Condition, M: MetricTrait>(self) -> Self::$block_metric<C, M>;
        fn $update_tracker_fn<C: Condition, M: MetricTrait>(self) -> Self::$update_tracker<C, M>;
        fn $block_tracker_fn<C: Condition, M: MetricTrait>(self) -> Self::$block_tracker<C, M>;
    };
}

pub trait MetricsBuilder {
    type AddMetric<
        SimRelay: SimulationRelay<M::SimulationRelay>,
        Relay: MetricsBuilderRelay<M::ExecutionOrder>,
        C: Condition,
        M: MetricTrait,
    >: MetricsBuilder;
    type AddTracker<
        SimRelay: SimulationRelay<M::SimulationRelay>,
        Relay: MetricsBuilderRelay<M::ExecutionOrder>,
        C: Condition,
        M: MetricTrait,
    >: MetricsBuilder;
    repeat!(
        def_metrics_and_trackers,
        0,
        9,
        AddUpdateMetric,
        AddBlockMetric,
        AddUpdateTracker,
        AddBlockTracker,
        add_update_metric,
        add_block_metric,
        add_update_tracker,
        add_block_tracker
    );
    type Finish: EcsBuilder;
    fn add_metric<SimRelay, Relay, C, M>(self) -> Self::AddMetric<SimRelay, Relay, C, M>
    where
        SimRelay: SimulationRelay<M::SimulationRelay>,
        Relay: MetricsBuilderRelay<M::ExecutionOrder>,
        C: Condition,
        M: MetricTrait;
    fn add_tracker<SimRelay, Relay, C, M>(self) -> Self::AddTracker<SimRelay, Relay, C, M>
    where
        SimRelay: SimulationRelay<M::SimulationRelay>,
        Relay: MetricsBuilderRelay<M::ExecutionOrder>,
        C: Condition,
        M: MetricTrait;
    fn skip<M: MetricTrait>(&mut self);
    fn finish(self) -> Self::Finish;
}

pub trait MetricsBuilderRelay<ExecutionOrder> {
    type AddMetric<B: MetricsBuilder, C: Condition, M: MetricTrait>: MetricsBuilder;
    type AddTracker<B: MetricsBuilder, C: Condition, M: MetricTrait>: MetricsBuilder;
    fn add_metric<B: MetricsBuilder, C: Condition, M: MetricTrait>(
        builder: B,
    ) -> Self::AddMetric<B, C, M>;
    fn add_tracker<B: MetricsBuilder, C: Condition, M: MetricTrait>(
        builder: B,
    ) -> Self::AddTracker<B, C, M>;
    fn skip<B: MetricsBuilder, M: MetricTrait>(mut builder: B) -> B {
        builder.skip::<M>();
        builder
    }
}
impl<B: EcsBuilder>
    MetricsBuilderStruct<
        B,
        StackedNest,
        StackedNest,
        StackedNest,
        StackedNest,
        StackedNest,
        StackedNest,
        StackedNest,
        StackedNest,
        StackedNest,
        StackedNest,
        StackedNest,
        StackedNest,
        StackedNest,
        StackedNest,
        StackedNest,
        StackedNest,
        StackedNest,
        StackedNest,
        StackedNest,
    >
{
    pub fn new(builder: B) -> Self {
        Self {
            builder,
            mems: StackedNest,
            update_metrics0: StackedNest,
            update_metrics1: StackedNest,
            update_metrics2: StackedNest,
            update_metrics3: StackedNest,
            update_metrics4: StackedNest,
            update_metrics5: StackedNest,
            update_metrics6: StackedNest,
            update_metrics7: StackedNest,
            update_metrics8: StackedNest,
            block_metrics0: StackedNest,
            block_metrics1: StackedNest,
            block_metrics2: StackedNest,
            block_metrics3: StackedNest,
            block_metrics4: StackedNest,
            block_metrics5: StackedNest,
            block_metrics6: StackedNest,
            block_metrics7: StackedNest,
            block_metrics8: StackedNest,
            field_offset: 0,
            n_trackers: 0,
        }
    }
}

macro_rules! impl_metrics_and_trackers {
    ($update_metric:ident, $block_metric:ident, $update_tracker:ident, $block_tracker:ident) => {
        type $update_metric<C: Condition, M: MetricTrait> = impl MetricsBuilder;
        type $block_metric<C: Condition, M: MetricTrait> = impl MetricsBuilder;
        type $update_tracker<C: Condition, M: MetricTrait> = impl MetricsBuilder;
        type $block_tracker<C: Condition, M: MetricTrait> = impl MetricsBuilder;
    };
}

impl<
    B,
    Mems,
    UM0,
    UM1,
    UM2,
    UM3,
    UM4,
    UM5,
    UM6,
    UM7,
    UM8,
    EM0,
    EM1,
    EM2,
    EM3,
    EM4,
    EM5,
    EM6,
    EM7,
    EM8,
> MetricsBuilder
    for MetricsBuilderStruct<
        B,
        Mems,
        UM0,
        UM1,
        UM2,
        UM3,
        UM4,
        UM5,
        UM6,
        UM7,
        UM8,
        EM0,
        EM1,
        EM2,
        EM3,
        EM4,
        EM5,
        EM6,
        EM7,
        EM8,
    >
where
    B: EcsBuilder,
    Mems: MetricMem + 'static,
    UM0: MetricUpdateBuilder + 'static,
    UM1: MetricUpdateBuilder + 'static,
    UM2: MetricUpdateBuilder + 'static,
    UM3: MetricUpdateBuilder + 'static,
    UM4: MetricUpdateBuilder + 'static,
    UM5: MetricUpdateBuilder + 'static,
    UM6: MetricUpdateBuilder + 'static,
    UM7: MetricUpdateBuilder + 'static,
    UM8: MetricUpdateBuilder + 'static,
    EM0: MetricUpdateBuilder + 'static,
    EM1: MetricUpdateBuilder + 'static,
    EM2: MetricUpdateBuilder + 'static,
    EM3: MetricUpdateBuilder + 'static,
    EM4: MetricUpdateBuilder + 'static,
    EM5: MetricUpdateBuilder + 'static,
    EM6: MetricUpdateBuilder + 'static,
    EM7: MetricUpdateBuilder + 'static,
    EM8: MetricUpdateBuilder + 'static,
{
    type AddMetric<
        SimRelay: SimulationRelay<M::SimulationRelay>,
        Relay: MetricsBuilderRelay<M::ExecutionOrder>,
        C: Condition,
        M: MetricTrait,
    > = impl MetricsBuilder;
    type AddTracker<
        SimRelay: SimulationRelay<M::SimulationRelay>,
        Relay: MetricsBuilderRelay<M::ExecutionOrder>,
        C: Condition,
        M: MetricTrait,
    > = impl MetricsBuilder;
    type Finish = impl EcsBuilder;

    repeat!(
        impl_metrics_and_trackers,
        0,
        9,
        AddUpdateMetric,
        AddBlockMetric,
        AddUpdateTracker,
        AddBlockTracker
    );

    esl_macros::impl_add_metrics_and_trackers!();

    #[inline(always)]
    fn add_metric<SimRelay, Relay, C, M>(self) -> Self::AddMetric<SimRelay, Relay, C, M>
    where
        SimRelay: SimulationRelay<M::SimulationRelay>,
        Relay: MetricsBuilderRelay<M::ExecutionOrder>,
        C: Condition,
        M: MetricTrait,
    {
        SimRelay::add_metric::<Self, Relay, C, M>(self)
    }

    #[inline(always)]
    fn add_tracker<SimRelay, Relay, C, M>(self) -> Self::AddTracker<SimRelay, Relay, C, M>
    where
        SimRelay: SimulationRelay<M::SimulationRelay>,
        Relay: MetricsBuilderRelay<M::ExecutionOrder>,
        C: Condition,
        M: MetricTrait,
    {
        SimRelay::add_tracker::<Self, Relay, C, M>(self)
    }

    #[inline(always)]
    fn skip<M: MetricTrait>(&mut self) {
        self.field_offset += core::mem::size_of::<M>();
    }

    #[inline(always)]
    fn finish(self) -> Self::Finish {
        let builder = self
            .builder
            .add_resource(NTrackers(self.n_trackers))
            .extend_entities(SampleRecorded::default())
            .extend_entities(SampleId::default());
        // Manually expanding macro because `builder` is not accessible outside the macro
        // macro_rules! add_systems {
        //     (
        //         $update_metrics:ident,
        //         $update_order:ident,
        //         $block_metrics:ident,
        //         $block_order:ident
        //     ) => {
        //         let builder = builder
        //             .add_system_to_stage_without_plugin(
        //                 MetricSystemBuilder::new(self.$update_metrics),
        //                 $update_order::new(),
        //             )
        //             .add_system_to_stage_without_plugin(
        //                 MetricSystemBuilder::new(self.$block_metrics),
        //                 $block_order::new(),
        //             );
        //     };
        // }
        let builder = builder
            .add_system_without_plugin(
                MetricSystemBuilder::new(self.update_metrics0),
                PostTrade0::new(),
            )
            .add_system_without_plugin(
                MetricSystemBuilder::new(self.block_metrics0),
                PostBlock0::new(),
            );
        let builder = builder
            .add_system_without_plugin(
                MetricSystemBuilder::new(self.update_metrics1),
                PostTrade1::new(),
            )
            .add_system_without_plugin(
                MetricSystemBuilder::new(self.block_metrics1),
                PostBlock1::new(),
            );
        let builder = builder
            .add_system_without_plugin(
                MetricSystemBuilder::new(self.update_metrics2),
                PostTrade2::new(),
            )
            .add_system_without_plugin(
                MetricSystemBuilder::new(self.block_metrics2),
                PostBlock2::new(),
            );
        let builder = builder
            .add_system_without_plugin(
                MetricSystemBuilder::new(self.update_metrics3),
                PostTrade3::new(),
            )
            .add_system_without_plugin(
                MetricSystemBuilder::new(self.block_metrics3),
                PostBlock3::new(),
            );
        let builder = builder
            .add_system_without_plugin(
                MetricSystemBuilder::new(self.update_metrics4),
                PostTrade4::new(),
            )
            .add_system_without_plugin(
                MetricSystemBuilder::new(self.block_metrics4),
                PostBlock4::new(),
            );
        let builder = builder
            .add_system_without_plugin(
                MetricSystemBuilder::new(self.update_metrics5),
                PostTrade5::new(),
            )
            .add_system_without_plugin(
                MetricSystemBuilder::new(self.block_metrics5),
                PostBlock5::new(),
            );
        let builder = builder
            .add_system_without_plugin(
                MetricSystemBuilder::new(self.update_metrics6),
                PostTrade6::new(),
            )
            .add_system_without_plugin(
                MetricSystemBuilder::new(self.block_metrics6),
                PostBlock6::new(),
            );
        let builder = builder
            .add_system_without_plugin(
                MetricSystemBuilder::new(self.update_metrics7),
                PostTrade7::new(),
            )
            .add_system_without_plugin(
                MetricSystemBuilder::new(self.block_metrics7),
                PostBlock7::new(),
            );
        let builder = builder
            .add_system_without_plugin(
                MetricSystemBuilder::new(self.update_metrics8),
                PostTrade8::new(),
            )
            .add_system_without_plugin(
                MetricSystemBuilder::new(self.block_metrics8),
                PostBlock8::new(),
            );
        builder
            .add_system_without_plugin(inc_sample_id::new(), IndicatorCompute::new())
            .add_system_without_plugin(
                MetricReaderSystemBuilder {
                    readers: self.mems.clone(),
                },
                Init::new(),
            )
            .add_system_without_plugin(MetricWriterSystemBuilder { writers: self.mems }, End::new())
    }
}

#[system]
fn inc_sample_id(updated: &mut SampleRecorded, sample_id: &mut SampleId) {
    if **updated {
        **sample_id += 1;
        **updated = false;
    }
}

pub trait MetricMem: Clone {
    fn read<E: EntityFetch>(
        &mut self,
        metrics_ptr: *const u8,
        accounts_per_thread: usize,
        threads_per_device: usize,
        thread_id: usize,
        entity: &mut E,
    );
    fn write<E: EntityFetch>(
        &mut self,
        metrics_ptr: *mut u8,
        accounts_per_thread: usize,
        threads_per_device: usize,
        thread_id: usize,
        entity: &mut E,
    );
}

pub trait MetricUpdate<W: World> {
    fn update<E>(&mut self, entity: &mut E, world: &mut W) -> Skip
    where
        E: EntityFetch;
}

impl<W: World> MetricUpdate<W> for StackedNest {
    #[inline(always)]
    fn update<E>(&mut self, _entity: &mut E, _world: &mut W) -> Skip
    where
        E: EntityFetch,
    {
        Skip::False
    }
}

impl<W: World, A: MetricUpdate<W>, B: MetricUpdate<W>> MetricUpdate<W> for Nested<A, B> {
    #[inline(always)]
    fn update<E>(&mut self, entity: &mut E, world: &mut W) -> Skip
    where
        E: EntityFetch,
        W: World,
    {
        self.inner.update(entity, world);
        self.item.update(entity, world);
        Skip::False
    }
}

struct TrackerUpdate<M: MetricTrait, C: Condition, W: World, const N: usize> {
    metric: MetricStruct<M, C, W, N>,
    tracks_state: TracksState<M, N>,
}

impl<M: MetricTrait, C: Condition, W: World, const N: usize> MetricUpdate<W>
    for TrackerUpdate<M, C, W, N>
{
    #[inline(always)]
    fn update<E>(&mut self, entity: &mut E, world: &mut W) -> Skip
    where
        E: EntityFetch,
    {
        if let Skip::True = self.metric.update(entity, world) {
            return Skip::True;
        }
        let mut tracks = Tracks::<'static, 'static, M, N>::get_param_for_entity::<
            W,
            MetricSystem,
            (),
            E,
        >(entity, &mut self.tracks_state, world)
        .unwrap();
        tracks.push();
        Skip::False
    }
}

struct TrackerUpdateBuilder<M, C> {
    metric_builder: MetricUpdateBuilderStruct<M, C>,
}

impl<M: MetricTrait, C: Condition> MetricUpdateBuilder for TrackerUpdateBuilder<M, C> {
    type Build<W: World, const N: usize> = TrackerUpdate<M, C, W, N>;

    #[inline(always)]
    fn build<W: World, const N: usize>(self, world: &mut W) -> Self::Build<W, N> {
        let state =
            <<Tracks<'static, 'static, M, N> as SystemParam>::State as SystemParamState>::init::<
                W,
                MetricSystem,
                (),
                StackedNest,
            >(&mut StackedNest, world);
        TrackerUpdate {
            metric: self.metric_builder.build(world),
            tracks_state: state,
        }
    }
}

struct MetricUpdateBuilderStruct<M, C> {
    _m: PhantomData<M>,
    _condition: PhantomData<C>,
}

impl<M: MetricTrait, C: Condition> MetricUpdateBuilder for MetricUpdateBuilderStruct<M, C> {
    type Build<W: World, const N: usize> = MetricStruct<M, C, W, N>;

    #[inline(always)]
    fn build<W: World, const N: usize>(self, world: &mut W) -> Self::Build<W, N> {
        let states = <<M::UpdateParams<'static, 'static, W, N> as SystemParam>::State as SystemParamState>::init::<
                W,
                MetricSystem,
                (),
                StackedNest,
            >(&mut StackedNest, world);
        let condition_states =
            <<C::Params<'static, 'static, W, N> as SystemParam>::State as SystemParamState>::init::<
                W,
                MetricSystem,
                (),
                StackedNest,
            >(&mut StackedNest, world);
        MetricStruct {
            states,
            condition_states,
        }
    }
}

pub trait MetricUpdateBuilder {
    type Build<W: World, const N: usize>: MetricUpdate<W>;
    fn build<W: World, const N: usize>(self, world: &mut W) -> Self::Build<W, N>;
}

impl MetricUpdateBuilder for StackedNest {
    type Build<W: World, const N: usize> = StackedNest;

    #[inline(always)]
    fn build<W: World, const N: usize>(self, _world: &mut W) -> Self::Build<W, N> {
        StackedNest
    }
}

impl<A: MetricUpdateBuilder, B: MetricUpdateBuilder> MetricUpdateBuilder for Nested<A, B> {
    type Build<W: World, const N: usize> = Nested<A::Build<W, N>, B::Build<W, N>>;

    #[inline(always)]
    fn build<W: World, const N: usize>(self, world: &mut W) -> Self::Build<W, N> {
        Nested::new(self.item.build(world), self.inner.build(world))
    }
}

struct MetricStruct<M: MetricTrait, C: Condition, W: World, const N: usize> {
    states: <M::UpdateParams<'static, 'static, W, N> as SystemParam>::State,
    condition_states: <C::Params<'static, 'static, W, N> as SystemParam>::State,
}

impl<M: MetricTrait, C: Condition, W: World, const N: usize> MetricUpdate<W>
    for MetricStruct<M, C, W, N>
{
    #[inline(always)]
    fn update<E>(&mut self, entity: &mut E, world: &mut W) -> Skip
    where
        E: EntityFetch,
    {
        let world_ptr = world as *mut W;
        unsafe {
            let lifetime_params = match C::Params::<'static, 'static>::get_param_for_entity::<_, MetricSystem, (), E>(
                entity,
                &mut self.condition_states,
                &mut *world_ptr,
            ) {
                Some(x) => x,
                None => return Skip::True,
            };
            let params: C::Params<'static, 'static, W, N> =
                core::mem::transmute_copy(&lifetime_params);
            core::mem::forget(lifetime_params);
            if C::run(params) == Skip::True {
                return Skip::True;
            }
        }
        unsafe {
            let lifetime_params =
                match M::UpdateParams::<'static, 'static>::get_param_for_entity::<_, MetricSystem, (), E>(
                    entity,
                    &mut self.states,
                    &mut *world_ptr,
                ) {
                    Some(x) => x,
                    None => return Skip::True,
                };
            let params: <M as MetricTrait>::UpdateParams<'static, 'static, W, N> =
                core::mem::transmute_copy(&lifetime_params);
            core::mem::forget(lifetime_params);
            M::update(params);
        }
        Skip::False
    }
}

struct MetricReaderSystemBuilder<T> {
    readers: T,
}

#[derive(SystemParamPlugin)]
struct MetricReaderSystem<T> {
    readers: T,
}

struct MetricReaderForEach<'a, T> {
    metrics_ptr: *const u8,
    accounts_per_thread: usize,
    threads_per_device: usize,
    thread_id: usize,
    readers: &'a mut T,
}

impl<'a, T: MetricMem> EntityFnMut for MetricReaderForEach<'a, T> {
    #[inline(always)]
    fn call_mut<E: EntityFetch>(&mut self, entity: &mut E) {
        self.readers.read(
            self.metrics_ptr,
            self.accounts_per_thread,
            self.threads_per_device,
            self.thread_id,
            entity,
        );
    }
}

impl<'w, 's, W: World, T: MetricMem> System<'w, 's, W> for MetricReaderSystem<T> {
    #[inline(always)]
    fn call(&'s mut self, world: &'w mut W) {
        let metrics_ptr = world.resource::<MetricsPtr>().0;
        let accounts_per_thread = world.resource::<AccountsPerThread>().0;
        let threads_per_device = world.resource::<ThreadsPerDevice>().0;
        let thread_id = world.resource::<ThreadId>().0;
        let for_each = MetricReaderForEach {
            metrics_ptr,
            accounts_per_thread,
            threads_per_device,
            thread_id,
            readers: &mut self.readers,
        };
        world.for_each(for_each);
    }
}

impl<'w, 's, T: MetricMem> SystemBuilder<'w, 's> for MetricReaderSystemBuilder<T> {
    type System<W: World, const N: usize> = MetricReaderSystem<T>;

    #[inline(always)]
    fn build<W: World, const N: usize>(self, _world: &mut W) -> Self::System<W, N> {
        MetricReaderSystem {
            readers: self.readers,
        }
    }
}

struct MetricWriterSystemBuilder<T> {
    writers: T,
}

#[derive(SystemParamPlugin)]
struct MetricWriterSystem<T> {
    writers: T,
}

struct MetricWriterForEach<'a, T> {
    metrics_ptr: *mut u8,
    accounts_per_thread: usize,
    threads_per_device: usize,
    thread_id: usize,
    writers: &'a mut T,
}

impl<'a, T: MetricMem> EntityFnMut for MetricWriterForEach<'a, T> {
    #[inline(always)]
    fn call_mut<E: EntityFetch>(&mut self, entity: &mut E) {
        self.writers.write(
            self.metrics_ptr,
            self.accounts_per_thread,
            self.threads_per_device,
            self.thread_id,
            entity,
        );
    }
}

impl<'w, 's, W: World, T: MetricMem> System<'w, 's, W> for MetricWriterSystem<T> {
    #[inline(always)]
    fn call(&'s mut self, world: &'w mut W) {
        let metrics_ptr = world.resource::<MetricsPtr>().0;
        let accounts_per_thread = world.resource::<AccountsPerThread>().0;
        let threads_per_device = world.resource::<ThreadsPerDevice>().0;
        let thread_id = world.resource::<ThreadId>().0;
        let for_each = MetricWriterForEach {
            metrics_ptr,
            accounts_per_thread,
            threads_per_device,
            thread_id,
            writers: &mut self.writers,
        };
        world.for_each(for_each);
    }
}

impl<'w, 's, T: MetricMem> SystemBuilder<'w, 's> for MetricWriterSystemBuilder<T> {
    type System<W: World, const N: usize> = MetricWriterSystem<T>;

    #[inline(always)]
    fn build<W: World, const N: usize>(self, _world: &mut W) -> Self::System<W, N> {
        MetricWriterSystem {
            writers: self.writers,
        }
    }
}

struct MetricMemStruct<M> {
    field_offset: usize,
    _m: PhantomData<M>,
}

impl<M> Clone for MetricMemStruct<M> {
    #[inline(always)]
    fn clone(&self) -> Self {
        Self {
            _m: self._m.clone(),
            field_offset: self.field_offset,
        }
    }
}

impl<M: MetricTrait> MetricMem for MetricMemStruct<M> {
    #[inline(always)]
    fn read<E: EntityFetch>(
        &mut self,
        metrics_ptr: *const u8,
        accounts_per_thread: usize,
        threads_per_device: usize,
        thread_id: usize,
        entity: &mut E,
    ) {
        let account_id = entity.entity().0;
        let mut metric = entity.component_mut::<MetricComponent<M>>();
        let offset = accounts_per_thread * threads_per_device * self.field_offset
            + account_id * threads_per_device * core::mem::size_of::<M>()
            + thread_id * core::mem::size_of::<M>();
        unsafe {
            metric.metric = *(metrics_ptr.add(offset) as *const M);
        }
    }

    #[inline(always)]
    fn write<E: EntityFetch>(
        &mut self,
        metrics_ptr: *mut u8,
        accounts_per_thread: usize,
        threads_per_device: usize,
        thread_id: usize,
        entity: &mut E,
    ) {
        let account_id = entity.entity().0;
        let metric = entity.component_mut::<MetricComponent<M>>();
        let offset = accounts_per_thread * threads_per_device * self.field_offset
            + account_id * threads_per_device * core::mem::size_of::<M>()
            + thread_id * core::mem::size_of::<M>();
        unsafe {
            *(metrics_ptr.add(offset) as *mut M) = metric.metric;
        }
    }
}

struct MetricMemWriteStruct<M> {
    field_offset: usize,
    _m: PhantomData<M>,
}

impl<M> Clone for MetricMemWriteStruct<M> {
    #[inline(always)]
    fn clone(&self) -> Self {
        Self {
            field_offset: self.field_offset,
            _m: self._m.clone(),
        }
    }
}

impl<M: MetricTrait> MetricMem for MetricMemWriteStruct<M> {
    #[inline(always)]
    fn read<E: EntityFetch>(
        &mut self,
        _metrics_ptr: *const u8,
        _accounts_per_thread: usize,
        _threads_per_device: usize,
        _thread_id: usize,
        _entity: &mut E,
    ) {
        // Empty, uses Default::default()
    }

    #[inline(always)]
    fn write<E: EntityFetch>(
        &mut self,
        metrics_ptr: *mut u8,
        accounts_per_thread: usize,
        threads_per_device: usize,
        thread_id: usize,
        entity: &mut E,
    ) {
        let account_id = entity.entity().0;
        let metric = entity.component_mut::<MetricComponent<M>>();
        let offset = accounts_per_thread * threads_per_device * self.field_offset
            + account_id * threads_per_device * core::mem::size_of::<M>()
            + thread_id * core::mem::size_of::<M>();
        unsafe {
            *(metrics_ptr.add(offset) as *mut M) = metric.metric;
        }
    }
}

impl MetricMem for StackedNest {
    #[inline(always)]
    fn read<E: EntityFetch>(
        &mut self,
        _metrics_ptr: *const u8,
        _accounts_per_thread: usize,
        _threads_per_device: usize,
        _thread_id: usize,
        _entity: &mut E,
    ) {
    }

    #[inline(always)]
    fn write<E: EntityFetch>(
        &mut self,
        _metrics_ptr: *mut u8,
        _accounts_per_thread: usize,
        _threads_per_device: usize,
        _thread_id: usize,
        _entity: &mut E,
    ) {
    }
}

impl<A: MetricMem, B: MetricMem> MetricMem for Nested<A, B> {
    #[inline(always)]
    fn read<E: EntityFetch>(
        &mut self,
        metrics_ptr: *const u8,
        accounts_per_thread: usize,
        threads_per_device: usize,
        thread_id: usize,
        entity: &mut E,
    ) {
        self.inner.read(
            metrics_ptr,
            accounts_per_thread,
            threads_per_device,
            thread_id,
            entity,
        );
        self.item.read(
            metrics_ptr,
            accounts_per_thread,
            threads_per_device,
            thread_id,
            entity,
        );
    }

    #[inline(always)]
    fn write<E: EntityFetch>(
        &mut self,
        metrics_ptr: *mut u8,
        accounts_per_thread: usize,
        threads_per_device: usize,
        thread_id: usize,
        entity: &mut E,
    ) {
        self.inner.write(
            metrics_ptr,
            accounts_per_thread,
            threads_per_device,
            thread_id,
            entity,
        );
        self.item.write(
            metrics_ptr,
            accounts_per_thread,
            threads_per_device,
            thread_id,
            entity,
        );
    }
}

struct MetricSystemBuilder<M> {
    systems: M,
}
impl<M> MetricSystemBuilder<M> {
    #[inline(always)]
    fn new(systems: M) -> Self {
        Self { systems }
    }
}

impl<'w, 's, M: MetricUpdateBuilder> SystemBuilder<'w, 's> for MetricSystemBuilder<M> {
    type System<W: World, const N: usize> = MetricUpdateSystem<M::Build<W, N>>;

    #[inline(always)]
    fn build<W: World, const N: usize>(self, world: &mut W) -> Self::System<W, N> {
        MetricUpdateSystem {
            systems: self.systems.build(world),
        }
    }
}

#[derive(SystemParamPlugin)]
pub struct MetricUpdateSystem<M> {
    systems: M,
}

impl<'w, 's, W: World, T: MetricUpdate<W>> System<'w, 's, W> for MetricUpdateSystem<T> {
    #[inline(always)]
    fn call(&'s mut self, world: &'w mut W) {
        let for_each = MetricSystemForEach {
            world: world as *mut W,
            metrics: &mut self.systems,
        };
        world.for_each(for_each);
    }
}

struct MetricSystem;
struct Unknown;

macro_rules! impl_mapper {
    ($($param:expr),*) => {
        $(
            impl Mapper<$param> for MetricSystem {
                type Name = Unknown;
            }
        )*
    };
}

impl_mapper!(0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15);
impl SystemParamNameMapper for MetricSystem {
    #[inline(always)]
    fn get_param_name<const PARAM_ID: usize>() -> Option<&'static str> {
        if PARAM_ID < 16 {
            Some(core::any::type_name::<Unknown>())
        } else {
            None
        }
    }
}

struct MetricSystemForEach<'a, W, M> {
    world: *mut W,
    metrics: &'a mut M,
}
impl<'a, W, M> EntityFnMut for MetricSystemForEach<'a, W, M>
where
    W: World,
    M: MetricUpdate<W>,
{
    #[inline(always)]
    fn call_mut<E: EntityFetch>(&mut self, entity: &mut E) {
        unsafe {
            self.metrics.update(entity, &mut *self.world);
        }
    }
}
