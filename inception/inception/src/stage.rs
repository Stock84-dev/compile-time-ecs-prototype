use core::marker::PhantomData;

use either::Either;

use crate::{
    nest_module::{Nested, StackedNest},
    system::System,
    world::World,
    EcsBuilder, SystemBuilder,
};

pub struct AddSystemToStageCommand<System> {
    pub builder: System,
}

pub trait StageLabel {
    type AddSystem<System: SystemBuilder<'static, 'static> + 'static, Builder: EcsBuilder>: EcsBuilder;
    #[must_use]
    fn add_system<System, Builder>(
        system: System,
        builder: Builder,
    ) -> Self::AddSystem<System, Builder>
    where
        Builder: EcsBuilder,
        System: SystemBuilder<'static, 'static> + 'static;
}

pub trait Stage<W> {
    fn run(&mut self, world: &mut W);
}

pub trait StageBuilder {
    type BuildStage<W: World, const N_EVENTS: usize>: Stage<W> + 'static;
    fn build_stage<W: World, const N_EVENTS: usize>(
        self,
        world: &mut W,
    ) -> Self::BuildStage<W, N_EVENTS>;
}

impl StageBuilder for StackedNest {
    type BuildStage<W: World, const N_EVENTS: usize> = StackedNest;

    #[inline(always)]
    fn build_stage<W: World, const N_EVENTS: usize>(
        self,
        _world: &mut W,
    ) -> Self::BuildStage<W, N_EVENTS> {
        StackedNest
    }
}

impl<N: StageBuilder, S: SystemBuilder<'static, 'static> + 'static> StageBuilder
    for Nested<N, AddSystemToStageCommand<S>>
{
    type BuildStage<W: World, const N_EVENTS: usize> =
        Nested<N::BuildStage<W, N_EVENTS>, StageData<(), S::System<W, N_EVENTS>>>;

    #[inline(always)]
    fn build_stage<W: World, const N_EVENTS: usize>(
        self,
        world: &mut W,
    ) -> Self::BuildStage<W, N_EVENTS> {
        Nested {
            item: StageData {
                stage: PhantomData::<()>::default(),
                systems: self.item.builder.build(world),
            },
            inner: self.inner.build_stage(world),
        }
    }
}

impl<L: StageBuilder, R: StageBuilder> StageBuilder for Either<L, R> {
    type BuildStage<W: World, const N_EVENTS: usize> =
        Either<L::BuildStage<W, N_EVENTS>, R::BuildStage<W, N_EVENTS>>;

    #[inline(always)]
    fn build_stage<W: World, const N_EVENTS: usize>(
        self,
        world: &mut W,
    ) -> Self::BuildStage<W, N_EVENTS> {
        match self {
            Either::Left(l) => Either::Left(l.build_stage(world)),
            Either::Right(r) => Either::Right(r.build_stage(world)),
        }
    }
}

impl<W: World> Stage<W> for () {
    fn run(&mut self, _world: &mut W) {}
}

impl<L: Stage<W>, R: Stage<W>, W: World> Stage<W> for Either<L, R> {
    fn run(&mut self, world: &mut W) {
        match self {
            Either::Left(l) => l.run(world),
            Either::Right(r) => r.run(world),
        }
    }
}

pub struct StageData<S, F> {
    stage: PhantomData<S>,
    systems: F,
}

impl<W: World, S: 'static, F: System<'static, 'static, W> + 'static> Stage<W> for StageData<S, F> {
    #[inline(always)]
    fn run(&mut self, world: &mut W) {
        unsafe {
            let world = core::mem::transmute::<&mut W, &'static mut W>(world);
            let systems = core::mem::transmute::<&mut F, &'static mut F>(&mut self.systems);
            systems.call(world);
        };
    }
}

impl<W: World> Stage<W> for StackedNest {
    #[inline(always)]
    fn run(&mut self, _world: &mut W) {
        // Empty
    }
}

impl<W: World, N: Stage<W>, I: Stage<W>> Stage<W> for Nested<N, I> {
    #[inline(always)]
    fn run(&mut self, world: &mut W) {
        self.inner.run(world);
        self.item.run(world);
    }
}
