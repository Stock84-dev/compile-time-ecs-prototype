use std::marker::PhantomData;

use bytemuck::{Pod, Zeroable};
use inception::*;

use crate::{
    schema::Readable,
    stages::{CatchUp, IndicatorCompute, Init},
    *,
};

pub trait Indicator: 'static {
    type Config: Pod;
    type Input: Readable;
    type Output: Pod + Zeroable;
    fn new(config: Self::Config) -> Self;
    fn init<R: Reader<Self::Input>>(&mut self, reader: &mut R) -> usize;
    fn update<R: Reader<Self::Input>>(&mut self, reader: &mut R, offset: usize) -> Self::Output;
}

#[system]
pub fn init_compute<S: 'static, P: 'static, I: Indicator>(
    mut series: EntityConfig<S, P, Series<I::Input>>,
    mut state: EntityConfig<S, P, I>,
    mut start: LoopIndex,
) {
    start.max_mut(state.init(&mut *series));
}

#[system]
pub fn compute<S: 'static, P: 'static, I: Indicator>(
    index: LoopIndex,
    mut series: EntityConfig<S, P, Series<I::Input>>,
    mut state: EntityConfig<S, P, I>,
    mut output: EntityConfig<S, P, I::Output>,
) {
    *output = state.update(&mut *series, *index);
}

pub trait IndicatorValue: SystemParamPlugin {
    type Indicator: Indicator<Output = Self>;
}

pub trait IndicatorConfigTrait {
    type Indicator: Indicator<Config = Self>;
}

pub struct ComputeIndicatorPlugin<Param, Entity, I: Indicator> {
    config: I::Config,
    input: Series<I::Input>,
    _context: PhantomData<(Entity, Param)>,
}

impl<Entity, Param, I: Indicator> ComputeIndicatorPlugin<Param, Entity, I> {
    #[inline(always)]
    pub fn new(config: I::Config, input: Series<I::Input>) -> Self {
        Self {
            config,
            input,
            _context: PhantomData,
        }
    }
}

impl<Entity: EntityRelay, Param: ParamLabel, I: Indicator> Plugin
    for ComputeIndicatorPlugin<Param, Entity, I>
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
            .add_config::<Param, Entity, _>(I::new(self.config))
            .add_config::<Param, Entity, _>(self.input)
            .add_config::<Param, Entity, _>(I::Output::zeroed())
    }
}

pub struct IndicatorPlugin<SystemName, ParamName, I>(PhantomData<(SystemName, ParamName, I)>);

impl<S: 'static, P: 'static, I: Indicator> SystemParamPlugin for IndicatorPlugin<S, P, I> {
    type Build<B: EcsBuilder> =
    <<<B as EcsBuilder>::AddSystemToStage<init_compute::System<S, P, I>, Init> as EcsBuilder>::AddSystemToStage<compute::System<S, P, I>, CatchUp> as EcsBuilder>::AddSystemToStage<compute::System<S, P, I>, IndicatorCompute>;

    #[inline(always)]
    fn build<B: EcsBuilder>(builder: B) -> Self::Build<B> {
        builder
            .add_system(init_compute::new::<S, P, I>(), Init::new())
            .add_system(compute::new::<S, P, I>(), CatchUp::new())
            .add_system(compute::new::<S, P, I>(), IndicatorCompute::new())
    }
}
