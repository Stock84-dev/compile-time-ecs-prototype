use std::prelude::v1::*;

use either::Either;

use crate::{
    ecs::EcsStruct,
    entities::{DefaultEntitiesBuilder1, Entity, EntityRelay, WorldQuery},
    nest_module::{Nest, Nested, StackedNest},
    plugin::{sealed::IntoEcsBuilder, Plugin, PluginLoader, Plugins, SystemParamPlugin},
    schedule::ScheduleBuilderTrait,
    stage::{StageBuilder, StageLabel},
    system::SystemBuilder,
    world::{BasicWorld, World, WorldStruct},
    Ecs, Entities, EntitiesBuilder, FromWorld, ParamLabel,
};

macro_rules! def_stage_items {
    ($f:ident, $ret:ident) => {
        type $ret<System: SystemBuilder<'static, 'static> + 'static>: EcsBuilder;
        #[must_use]
        fn $f<System>(self, system: System) -> Self::$ret<System>
        where
            System: SystemBuilder<'static, 'static> + 'static;
    };
}

pub trait EcsBuilder: PluginLoader {
    type AddConfig<Param: ParamLabel, Entity: EntityRelay, T: 'static>: EcsBuilder;
    type AddGenericConfig<System: 'static, Param: 'static, Entity: EntityRelay, T: 'static>: EcsBuilder;
    type ExtendConfig<Param: ParamLabel, T: Clone + 'static>: EcsBuilder;
    type ExtendGenericConfig<System: 'static, Param: 'static, T: Clone + 'static>: EcsBuilder;
    type ForEachOwned<I: EcsBuilderIter, O: EcsBuilderOperation>: EcsBuilder;
    type SetMaxEvents<const N_MAX_EVENTS: usize>: EcsBuilder;
    type AddSystemToStageWithoutPlugin<System: SystemBuilder<'static, 'static> + 'static, Stage: StageLabel>: EcsBuilder;
    type AddSystemToStage<System: SystemBuilder<'static, 'static> + 'static, Stage: StageLabel>: EcsBuilder;
    type AddResource<T: 'static>: EcsBuilder;
    type InitResource<T: 'static>: EcsBuilder;
    type Spawn: EcsBuilder;
    type AddComponent<ER: EntityRelay, T: 'static>: EcsBuilder;
    type ExtendEntities<Component: Clone + 'static>: EcsBuilder;
    type AddPluginOnce<Plug: Plugin + 'static>: EcsBuilder;
    type AddPlugin<Plug: Plugin + 'static>: EcsBuilder;
    type SetPluginLoaded<Plugin: 'static>: EcsBuilder;
    type ScheduleBuilder: StageBuilder;
    type Build: Ecs;

    all_tuples::repeat!(
        def_stage_items,
        0,
        32,
        add_system_to_stage,
        AddSystemToStage
    );

    #[must_use]
    fn add_config<Param: ParamLabel, Entity: EntityRelay, T: 'static>(
        self,
        config: T,
    ) -> Self::AddConfig<Param, Entity, T>;
    #[must_use]
    fn add_generic_config<System: 'static, Param: 'static, Entity: EntityRelay, T: 'static>(
        self,
        config: T,
    ) -> Self::AddGenericConfig<System, Param, Entity, T>;
    #[must_use]
    fn extend_config<Param: ParamLabel, T: Clone + 'static>(
        self,
        config: T,
    ) -> Self::ExtendConfig<Param, T>;
    #[must_use]
    fn extend_generic_config<System: 'static, Param: 'static, T: Clone + 'static>(
        self,
        config: T,
    ) -> Self::ExtendGenericConfig<System, Param, T>;
    #[must_use]
    fn for_each_owned<I, O>(self, iter: I, operation: O) -> Self::ForEachOwned<I, O>
    where
        I: EcsBuilderIter,
        O: EcsBuilderOperation;
    #[must_use]
    fn set_max_events<const N_MAX_EVENTS: usize>(self) -> Self::SetMaxEvents<N_MAX_EVENTS>;
    #[must_use]
    fn add_system<Stage, System>(
        self,
        system: System,
        stage: Stage,
    ) -> Self::AddSystemToStage<System, Stage>
    where
        System: SystemBuilder<'static, 'static> + 'static,
        Stage: StageLabel;
    #[must_use]
    fn add_system_without_plugin<Stage, System>(
        self,
        system: System,
        stage: Stage,
    ) -> Self::AddSystemToStageWithoutPlugin<System, Stage>
    where
        System: SystemBuilder<'static, 'static> + 'static,
        Stage: StageLabel;
    #[must_use]
    fn add_resource<T: 'static>(self, resource: T) -> Self::AddResource<T>;
    #[must_use]
    fn init_resource<T: FromWorld>(self) -> Self::InitResource<T>;
    #[must_use]
    fn spawn(self, entity: &mut Entity) -> Self::Spawn;
    #[must_use]
    fn add_component<ER: EntityRelay, T: 'static>(
        self,
        component: T,
        entity_relay: ER,
    ) -> Self::AddComponent<ER, T>;
    #[must_use]
    fn extend_entities<Component>(self, component: Component) -> Self::ExtendEntities<Component>
    where
        Component: Clone + 'static;
    #[must_use]
    fn add_plugin_once<Plug: Plugin + 'static>(self, plugin: Plug) -> Self::AddPluginOnce<Plug>;
    #[must_use]
    fn add_plugin<Plug: Plugin + 'static>(self, plugin: Plug) -> Self::AddPlugin<Plug>;
    #[must_use]
    fn contains_plugin<Plugin: 'static>(&self) -> bool;
    #[must_use]
    fn __set_plugin_loaded<Plugin: 'static>(self) -> Self::SetPluginLoaded<Plugin>;
    #[must_use]
    // Taking Sched by value even, though we don't need it. This is a workaround of an ICE.
    // https://github.com/rust-lang/rust/issues/99945
    fn build(self) -> Self::Build;

    #[must_use]
    fn get_resource<T: 'static>(&self) -> Option<&T>;
    #[must_use]
    fn resource<T: 'static>(&self) -> &T;
    #[must_use]
    fn get_resource_mut<T: 'static>(&mut self) -> Option<&mut T>;
    #[must_use]
    fn resource_mut<T: 'static>(&mut self) -> &mut T;

    #[must_use]
    fn get_component<T: 'static>(&self, entity: Entity) -> Option<&T>;
    #[must_use]
    fn component<T: 'static>(&self, entity: Entity) -> &T;
    #[must_use]
    fn get_component_mut<T: 'static>(&mut self, entity: Entity) -> Option<&mut T>;
    #[must_use]
    fn component_mut<T: 'static>(&mut self, entity: Entity) -> &mut T;
    fn query<'w, F, Q>(&'w mut self, f: F)
    where
        F: FnMut(<Q as WorldQuery>::Item<'w>),
        Q: WorldQuery;
}

pub trait EcsBuilderOperation {
    type Apply<B: EcsBuilder>: EcsBuilder;
    fn apply<B: EcsBuilder>(&mut self, builder: B) -> Self::Apply<B>;
}

impl<S> EcsBuilderStruct<StackedNest, S, BasicWorld, StackedNest, StackedNest, 0>
where
    S: StageBuilder + ScheduleBuilderTrait,
{
    pub fn new<EB: EntitiesBuilder + Entities, const N_EVENTS: usize>(
        schedule_builder: S,
        entities_builder: EB,
    ) -> EcsBuilderStruct<
        StackedNest,
        S,
        WorldStruct<StackedNest, EB>,
        StackedNest,
        StackedNest,
        N_EVENTS,
    > {
        EcsBuilderStruct {
            added_plugins: StackedNest,
            world: WorldStruct {
                resources: StackedNest,
                entities: entities_builder,
            },
            entities: StackedNest,
            stages: StackedNest,
            entity_count: 0,
            schedule_builder,
        }
    }
}

pub struct EcsBuilderStruct<P, SchedBuilder, W, E, Stages, const N_EVENTS: usize> {
    added_plugins: P,
    schedule_builder: SchedBuilder,
    world: W,
    entities: E,
    entity_count: usize,
    stages: Stages,
}

macro_rules! impl_stage_items_for_builder {
    ($f:ident, $ret:ident) => {
        type $ret<System: SystemBuilder<'static, 'static> + 'static> = EcsBuilderStruct<
            P,
            <SchedBuilder as ScheduleBuilderTrait>::$ret<System>,
            W,
            E,
            S,
            N_EVENTS,
        >;
        #[inline(always)]
        fn $f<System>(self, system: System) -> Self::$ret<System>
        where
            System: SystemBuilder<'static, 'static> + 'static,
        {
            EcsBuilderStruct {
                added_plugins: self.added_plugins,
                world: self.world,
                entities: self.entities,
                entity_count: self.entity_count,
                stages: self.stages,
                schedule_builder: self.schedule_builder.$f(system),
            }
        }
    };
}

impl<P, SchedBuilder, W, E, S, const N_EVENTS: usize> EcsBuilder
    for EcsBuilderStruct<P, SchedBuilder, W, E, S, N_EVENTS>
where
    P: Plugins,
    SchedBuilder: StageBuilder + ScheduleBuilderTrait,
    W: World,
    E: Nest,
    S: Nest + StageBuilder,
{
    type AddComponent<ER: EntityRelay, T: 'static> =
        EcsBuilderStruct<P, SchedBuilder, W::AddComponent<T, ER>, E, S, N_EVENTS>;
    type AddConfig<Param: ParamLabel, Entity: EntityRelay, T: 'static> =
        EcsBuilderStruct<P, SchedBuilder, W::AddConfig<Param, Entity, T>, E, S, N_EVENTS>;
    type AddGenericConfig<System: 'static, Param: 'static, Entity: EntityRelay, T: 'static> =
        EcsBuilderStruct<
            P,
            SchedBuilder,
            W::AddGenericConfig<System, Param, Entity, T>,
            E,
            S,
            N_EVENTS,
        >;
    type AddPlugin<Plug: Plugin + 'static> = <<Plug::Build<
        <Plug::Deps<Self> as IntoEcsBuilder>::Builder,
    > as EcsBuilder>::SetPluginLoaded<Plug> as IntoEcsBuilder>::Builder;
    type AddPluginOnce<Plug: Plugin + 'static> =
        <<Self as PluginLoader>::Load<Plug> as IntoEcsBuilder>::Builder;
    type AddResource<T: 'static> =
        EcsBuilderStruct<P, SchedBuilder, W::AddResource<T>, E, S, N_EVENTS>;
    type AddSystemToStage<System: SystemBuilder<'static, 'static> + 'static, Stage: StageLabel> =
        <<System as SystemBuilder<'static, 'static>>::System<
            WorldStruct<StackedNest, DefaultEntitiesBuilder1>,
            N_EVENTS,
        > as SystemParamPlugin>::Build<
            Stage::AddSystem<System, EcsBuilderStruct<P, SchedBuilder, W, E, S, N_EVENTS>>,
        >;
    type AddSystemToStageWithoutPlugin<
        System: SystemBuilder<'static, 'static> + 'static,
        Stage: StageLabel,
    > = Stage::AddSystem<System, EcsBuilderStruct<P, SchedBuilder, W, E, S, N_EVENTS>>;
    type Build = EcsStruct<W, <SchedBuilder as StageBuilder>::BuildStage<W, N_EVENTS>>;
    type ExtendConfig<Param: ParamLabel, T: Clone + 'static> =
        EcsBuilderStruct<P, SchedBuilder, W::ExtendConfig<Param, T>, E, S, N_EVENTS>;
    type ExtendEntities<Component: Clone + 'static> =
        EcsBuilderStruct<P, SchedBuilder, W::ExtendEntities<Component>, E, S, N_EVENTS>;
    type ExtendGenericConfig<System: 'static, Param: 'static, T: Clone + 'static> =
        EcsBuilderStruct<P, SchedBuilder, W::ExtendGenericConfig<System, Param, T>, E, S, N_EVENTS>;
    type ForEachOwned<I: EcsBuilderIter, O: EcsBuilderOperation> = I::ForEachOwned<O, Self>;
    type InitResource<T: 'static> =
        EcsBuilderStruct<P, SchedBuilder, W::AddResource<T>, E, S, N_EVENTS>;
    type ScheduleBuilder = SchedBuilder;
    type SetMaxEvents<const N_MAX_EVENTS: usize> =
        EcsBuilderStruct<P, SchedBuilder, W, E, S, N_MAX_EVENTS>;
    type SetPluginLoaded<Plugin: 'static> =
        EcsBuilderStruct<P::Push<Plugin>, SchedBuilder, W, E, S, N_EVENTS>;
    type Spawn = EcsBuilderStruct<P, SchedBuilder, W, Nested<E, Entity>, S, N_EVENTS>;

    all_tuples::repeat!(
        impl_stage_items_for_builder,
        0,
        32,
        add_system_to_stage,
        AddSystemToStage
    );

    #[inline(always)]
    fn add_config<Param: ParamLabel, Entity: EntityRelay, T: 'static>(
        self,
        config: T,
    ) -> Self::AddConfig<Param, Entity, T> {
        EcsBuilderStruct {
            added_plugins: self.added_plugins,
            world: self.world.add_config::<Param, Entity, T>(config),
            entities: self.entities,
            entity_count: self.entity_count,
            stages: self.stages,
            schedule_builder: self.schedule_builder,
        }
    }

    #[inline(always)]
    fn add_generic_config<System: 'static, Param: 'static, Entity: EntityRelay, T: 'static>(
        self,
        config: T,
    ) -> Self::AddGenericConfig<System, Param, Entity, T> {
        EcsBuilderStruct {
            added_plugins: self.added_plugins,
            world: self
                .world
                .add_generic_config::<System, Param, Entity, T>(config),
            entities: self.entities,
            entity_count: self.entity_count,
            stages: self.stages,
            schedule_builder: self.schedule_builder,
        }
    }

    #[inline(always)]
    fn extend_config<Param: ParamLabel, T: Clone + 'static>(
        self,
        config: T,
    ) -> Self::ExtendConfig<Param, T> {
        EcsBuilderStruct {
            added_plugins: self.added_plugins,
            world: self.world.extend_config::<Param, T>(config),
            entities: self.entities,
            entity_count: self.entity_count,
            stages: self.stages,
            schedule_builder: self.schedule_builder,
        }
    }

    #[inline(always)]
    fn extend_generic_config<System: 'static, Param: 'static, T: Clone + 'static>(
        self,
        config: T,
    ) -> Self::ExtendGenericConfig<System, Param, T> {
        EcsBuilderStruct {
            added_plugins: self.added_plugins,
            world: self.world.extend_generic_config::<System, Param, T>(config),
            entities: self.entities,
            entity_count: self.entity_count,
            stages: self.stages,
            schedule_builder: self.schedule_builder,
        }
    }

    #[inline(always)]
    fn for_each_owned<I, O>(self, iter: I, operation: O) -> Self::ForEachOwned<I, O>
    where
        I: EcsBuilderIter,
        O: EcsBuilderOperation,
    {
        iter.for_each_owned(self, operation)
    }

    #[inline(always)]
    fn set_max_events<const N_MAX_EVENTS: usize>(self) -> Self::SetMaxEvents<N_MAX_EVENTS> {
        EcsBuilderStruct {
            added_plugins: self.added_plugins,
            world: self.world,
            entities: self.entities,
            entity_count: self.entity_count,
            stages: self.stages,
            schedule_builder: self.schedule_builder,
        }
    }

    #[inline(always)]
    fn add_system<Stage, System>(
        self,
        system: System,
        stage: Stage,
    ) -> Self::AddSystemToStage<System, Stage>
    where
        System: SystemBuilder<'static, 'static> + 'static,
        Stage: StageLabel,
    {
        let builder = self.add_system_without_plugin(system, stage);
        System::load_plugins(builder)
    }

    #[inline(always)]
    fn add_system_without_plugin<Stage, System>(
        self,
        system: System,
        _stage: Stage,
    ) -> Self::AddSystemToStageWithoutPlugin<System, Stage>
    where
        System: SystemBuilder<'static, 'static> + 'static,
        Stage: StageLabel,
    {
        Stage::add_system(system, self)
    }

    #[inline(always)]
    fn add_resource<T: 'static>(self, resource: T) -> Self::AddResource<T> {
        EcsBuilderStruct {
            added_plugins: self.added_plugins,
            world: self.world.add_resource(resource),
            entities: self.entities,
            entity_count: self.entity_count,
            stages: self.stages,
            schedule_builder: self.schedule_builder,
        }
    }

    #[inline(always)]
    fn init_resource<T: FromWorld>(mut self) -> Self::InitResource<T> {
        let resource = T::from_world(&mut self.world);

        EcsBuilderStruct {
            added_plugins: self.added_plugins,
            world: self.world.add_resource(resource),
            entities: self.entities,
            entity_count: self.entity_count,
            stages: self.stages,
            schedule_builder: self.schedule_builder,
        }
    }

    #[inline(always)]
    fn spawn(mut self, entity: &mut Entity) -> Self::Spawn {
        *entity = Entity(self.entity_count);
        self.entity_count += 1;
        EcsBuilderStruct {
            added_plugins: self.added_plugins,
            world: self.world,
            entities: self.entities.push(*entity),
            entity_count: self.entity_count,
            stages: self.stages,
            schedule_builder: self.schedule_builder,
        }
    }

    #[inline(always)]
    fn add_component<ER: EntityRelay, T: 'static>(
        self,
        component: T,
        entity: ER,
    ) -> Self::AddComponent<ER, T> {
        EcsBuilderStruct {
            added_plugins: self.added_plugins,
            world: self.world.add_component(component, entity),
            entities: self.entities,
            entity_count: self.entity_count,
            stages: self.stages,
            schedule_builder: self.schedule_builder,
        }
    }

    #[inline(always)]
    fn extend_entities<Component>(self, component: Component) -> Self::ExtendEntities<Component>
    where
        Component: Clone + 'static,
    {
        EcsBuilderStruct {
            added_plugins: self.added_plugins,
            world: self.world.extend_entities(component),
            entities: self.entities,
            entity_count: self.entity_count,
            stages: self.stages,
            schedule_builder: self.schedule_builder,
        }
    }

    #[inline(always)]
    fn add_plugin_once<Plug: Plugin + 'static>(self, plugin: Plug) -> Self::AddPluginOnce<Plug> {
        PluginLoader::load_once(self, plugin).__into_builder()
    }

    #[inline(always)]
    fn add_plugin<Plug: Plugin + 'static>(self, plugin: Plug) -> Self::AddPlugin<Plug> {
        PluginLoader::load(self, plugin).__into_builder()
    }

    #[inline(always)]
    fn contains_plugin<Plugin: 'static>(&self) -> bool {
        self.added_plugins.contains_plugin::<Plugin>()
    }

    #[inline(always)]
    fn __set_plugin_loaded<Plugin: 'static>(self) -> Self::SetPluginLoaded<Plugin> {
        EcsBuilderStruct {
            added_plugins: self.added_plugins.push::<Plugin>(),
            world: self.world,
            entities: self.entities,
            entity_count: self.entity_count,
            stages: self.stages,
            schedule_builder: self.schedule_builder,
        }
    }

    #[inline(always)]
    fn build(self) -> Self::Build {
        let mut world = self.world;
        let stages = self.schedule_builder.build_stage(&mut world);

        crate::ecs::EcsStruct { world, stages }
    }

    #[inline(always)]
    fn get_resource<T: 'static>(&self) -> Option<&T> {
        self.world.get_resource::<T>()
    }

    #[inline(always)]
    fn resource<T: 'static>(&self) -> &T {
        self.world.resource::<T>()
    }

    #[inline(always)]
    fn get_resource_mut<T: 'static>(&mut self) -> Option<&mut T> {
        self.world.get_resource_mut::<T>()
    }

    #[inline(always)]
    fn resource_mut<T: 'static>(&mut self) -> &mut T {
        self.world.resource_mut::<T>()
    }

    #[inline(always)]
    fn get_component<T: 'static>(&self, entity: Entity) -> Option<&T> {
        self.world.get_component::<T>(entity)
    }

    #[inline(always)]
    fn component<T: 'static>(&self, entity: Entity) -> &T {
        self.world.component::<T>(entity)
    }

    #[inline(always)]
    fn get_component_mut<T: 'static>(&mut self, entity: Entity) -> Option<&mut T> {
        self.world.get_component_mut::<T>(entity)
    }

    #[inline(always)]
    fn component_mut<T: 'static>(&mut self, entity: Entity) -> &mut T {
        self.world.component_mut::<T>(entity)
    }

    #[inline(always)]
    fn query<'w, F, Q>(&'w mut self, f: F)
    where
        F: FnMut(<Q as WorldQuery>::Item<'w>),
        Q: WorldQuery,
    {
        self.world.query::<F, Q>(f)
    }
}

impl<P, SchedBuilder, W, E, S, const N_EVENTS: usize> PluginLoader
    for EcsBuilderStruct<P, SchedBuilder, W, E, S, N_EVENTS>
where
    P: Plugins,
    SchedBuilder: StageBuilder + ScheduleBuilderTrait,
    W: World,
    E: Nest,
    S: Nest + StageBuilder,
{
    type Finish = Self;
    type LoadMultiple<Plug: Plugin + 'static> = <Plug::Build<
        <Plug::Deps<Self> as IntoEcsBuilder>::Builder,
    > as EcsBuilder>::SetPluginLoaded<Plug>;

    type Load<Plug: Plugin + 'static> = impl PluginLoader;

    #[inline(always)]
    fn load_once<Plug: Plugin + 'static>(self, plugin: Plug) -> Self::Load<Plug> {
        if self.added_plugins.contains_plugin::<Plug>() {
            Either::Left(self)
        } else {
            Either::Right(self.load(plugin))
        }
    }

    #[inline(always)]
    fn load<Plug: Plugin + 'static>(self, mut plugin: Plug) -> Self::LoadMultiple<Plug> {
        let builder = plugin.deps(self).__into_builder();
        if builder.contains_plugin::<Plug>() {
            panic!(
                "Plugin `{}` has a circular dependency",
                core::any::type_name::<Plug>()
            );
        }
        plugin.build(builder).__set_plugin_loaded::<Plug>()
    }
}

impl<O: PluginLoader + EcsBuilder, N: PluginLoader + EcsBuilder> PluginLoader for Either<O, N> {
    type Finish = Self;

    type Load<P: Plugin + 'static> = impl PluginLoader;
    type LoadMultiple<P: Plugin + 'static> = impl PluginLoader;

    #[inline(always)]
    fn load_once<Plug: Plugin + 'static>(self, plugin: Plug) -> Self::Load<Plug> {
        match self {
            Either::Left(original) => Either::Left(original.load_once(plugin).__into_builder()),
            Either::Right(new) => Either::Right(new.load_once(plugin).__into_builder()),
        }
    }

    #[inline(always)]
    fn load<Plug: Plugin + 'static>(self, plugin: Plug) -> Self::LoadMultiple<Plug> {
        match self {
            Either::Left(original) => Either::Left(original.load(plugin).__into_builder()),
            Either::Right(new) => Either::Right(new.load(plugin).__into_builder()),
        }
    }
}

macro_rules! impl_stage_items_for_either {
    ($f:ident, $f_plugin:ident, $ret:ident, $ret_plugin:ident) => {
        type $ret<System: SystemBuilder<'static, 'static> + 'static> = impl EcsBuilder;
        #[inline(always)]
        fn $f<System>(self, system: System) -> Self::$ret<System>
        where
            System: SystemBuilder<'static, 'static> + 'static,
        {
            match self {
                Either::Left(original) => Either::Left(original.$f(system)),
                Either::Right(new) => Either::Right(new.$f(system)),
            }
        }
    };
}

impl<L: EcsBuilder, R: EcsBuilder> EcsBuilder for Either<L, R> {
    type AddPlugin<Plug: Plugin + 'static> =
        Either<<L as EcsBuilder>::AddPlugin<Plug>, <R as EcsBuilder>::AddPlugin<Plug>>;
    type AddPluginOnce<Plug: Plugin + 'static> =
        Either<L::AddPluginOnce<Plug>, R::AddPluginOnce<Plug>>;
    type AddSystemToStage<System: SystemBuilder<'static, 'static> + 'static, Stage: StageLabel> =
        Either<L::AddSystemToStage<System, Stage>, R::AddSystemToStage<System, Stage>>;
    type AddSystemToStageWithoutPlugin<
        System: SystemBuilder<'static, 'static> + 'static,
        Stage: StageLabel,
    > = Either<
        L::AddSystemToStageWithoutPlugin<System, Stage>,
        R::AddSystemToStageWithoutPlugin<System, Stage>,
    >;
    type ExtendConfig<Param: ParamLabel, T: Clone + 'static> =
        Either<L::ExtendConfig<Param, T>, R::ExtendConfig<Param, T>>;
    type ExtendEntities<Component: Clone + 'static> =
        Either<L::ExtendEntities<Component>, R::ExtendEntities<Component>>;
    type ExtendGenericConfig<System: 'static, Param: 'static, T: Clone + 'static> =
        Either<L::ExtendGenericConfig<System, Param, T>, R::ExtendGenericConfig<System, Param, T>>;
    type ForEachOwned<I: EcsBuilderIter, O: EcsBuilderOperation> = I::ForEachOwned<O, Self>;
    type ScheduleBuilder = Either<L::ScheduleBuilder, R::ScheduleBuilder>;

    type AddComponent<ER: EntityRelay, T: 'static> = impl EcsBuilder;
    type AddConfig<Param: ParamLabel, Entity: EntityRelay, T: 'static> = impl EcsBuilder;
    type AddGenericConfig<System: 'static, Param: 'static, Entity: EntityRelay, T: 'static> =
        impl EcsBuilder;
    type AddResource<T: 'static> = impl EcsBuilder;
    type Build = impl Ecs;
    type InitResource<T: 'static> = impl EcsBuilder;
    type SetMaxEvents<const N_MAX_EVENTS: usize> = impl EcsBuilder;
    type SetPluginLoaded<P: 'static> = impl EcsBuilder;
    type Spawn = impl EcsBuilder;

    all_tuples::repeat!(
        impl_stage_items_for_either,
        0,
        32,
        add_system_to_stage,
        add_system_without_plugin_to_stage,
        AddSystemToStage,
        AddSystemWithoutPluginToStage
    );

    #[inline(always)]
    fn add_config<Param: ParamLabel, Entity: EntityRelay, T: 'static>(
        self,
        config: T,
    ) -> Self::AddConfig<Param, Entity, T> {
        match self {
            Either::Left(x) => Either::Left(x.add_config::<Param, Entity, T>(config)),
            Either::Right(x) => Either::Right(x.add_config::<Param, Entity, T>(config)),
        }
    }

    #[inline(always)]
    fn add_generic_config<System: 'static, Param: 'static, Entity: EntityRelay, T: 'static>(
        self,
        config: T,
    ) -> Self::AddGenericConfig<System, Param, Entity, T> {
        match self {
            Either::Left(x) => {
                Either::Left(x.add_generic_config::<System, Param, Entity, T>(config))
            },
            Either::Right(x) => {
                Either::Right(x.add_generic_config::<System, Param, Entity, T>(config))
            },
        }
    }

    #[inline(always)]
    fn for_each_owned<I, O>(self, iter: I, operation: O) -> Self::ForEachOwned<I, O>
    where
        I: EcsBuilderIter,
        O: EcsBuilderOperation,
    {
        iter.for_each_owned(self, operation)
    }

    #[inline(always)]
    fn set_max_events<const N_MAX_EVENTS: usize>(self) -> Self::SetMaxEvents<N_MAX_EVENTS> {
        match self {
            Either::Left(x) => Either::Left(x.set_max_events::<N_MAX_EVENTS>()),
            Either::Right(x) => Either::Right(x.set_max_events::<N_MAX_EVENTS>()),
        }
    }

    #[inline(always)]
    fn add_system<Stage, System>(
        self,
        system: System,
        stage: Stage,
    ) -> Self::AddSystemToStage<System, Stage>
    where
        System: SystemBuilder<'static, 'static> + 'static,
        Stage: StageLabel,
    {
        match self {
            Either::Left(x) => Either::Left(x.add_system(system, stage)),
            Either::Right(x) => Either::Right(x.add_system(system, stage)),
        }
    }

    #[inline(always)]
    fn add_system_without_plugin<Stage, System>(
        self,
        system: System,
        stage: Stage,
    ) -> Self::AddSystemToStageWithoutPlugin<System, Stage>
    where
        System: SystemBuilder<'static, 'static> + 'static,
        Stage: StageLabel,
    {
        match self {
            Either::Left(x) => Either::Left(x.add_system_without_plugin(system, stage)),
            Either::Right(x) => Either::Right(x.add_system_without_plugin(system, stage)),
        }
    }

    #[inline(always)]
    fn add_resource<T: 'static>(self, resource: T) -> Self::AddResource<T> {
        match self {
            Either::Left(x) => Either::Left(x.add_resource(resource)),
            Either::Right(x) => Either::Right(x.add_resource(resource)),
        }
    }

    #[inline(always)]
    fn init_resource<T: FromWorld>(self) -> Self::InitResource<T> {
        match self {
            Either::Left(x) => Either::Left(x.init_resource::<T>()),
            Either::Right(x) => Either::Right(x.init_resource::<T>()),
        }
    }

    #[inline(always)]
    fn spawn(self, entity: &mut Entity) -> Self::Spawn {
        match self {
            Either::Left(x) => Either::Left(x.spawn(entity)),
            Either::Right(x) => Either::Right(x.spawn(entity)),
        }
    }

    #[inline(always)]
    fn add_component<ER: EntityRelay, T: 'static>(
        self,
        component: T,
        entity: ER,
    ) -> Self::AddComponent<ER, T> {
        match self {
            Either::Left(x) => Either::Left(x.add_component(component, entity)),
            Either::Right(x) => Either::Right(x.add_component(component, entity)),
        }
    }

    #[inline(always)]
    fn extend_entities<Component>(self, component: Component) -> Self::ExtendEntities<Component>
    where
        Component: Clone + 'static,
    {
        match self {
            Either::Left(x) => Either::Left(x.extend_entities(component)),
            Either::Right(x) => Either::Right(x.extend_entities(component)),
        }
    }

    #[inline(always)]
    fn add_plugin_once<Plug: Plugin + 'static>(self, plugin: Plug) -> Self::AddPluginOnce<Plug> {
        match self {
            Either::Left(x) => Either::Left(x.add_plugin_once(plugin)),
            Either::Right(x) => Either::Right(x.add_plugin_once(plugin)),
        }
    }

    #[inline(always)]
    fn add_plugin<Plug: Plugin + 'static>(self, plugin: Plug) -> Self::AddPlugin<Plug> {
        match self {
            Either::Left(x) => Either::Left(x.add_plugin(plugin)),
            Either::Right(x) => Either::Right(x.add_plugin(plugin)),
        }
    }

    #[inline(always)]
    fn contains_plugin<Plugin: 'static>(&self) -> bool {
        match self {
            Either::Left(x) => x.contains_plugin::<Plugin>(),
            Either::Right(x) => x.contains_plugin::<Plugin>(),
        }
    }

    #[inline(always)]
    fn __set_plugin_loaded<P: 'static>(self) -> Self::SetPluginLoaded<P> {
        match self {
            Either::Left(x) => Either::Left(x.__set_plugin_loaded::<P>()),
            Either::Right(x) => Either::Right(x.__set_plugin_loaded::<P>()),
        }
    }

    #[inline(always)]
    fn build(self) -> Self::Build {
        match self {
            Either::Left(x) => Either::Left(x.build()),
            Either::Right(x) => Either::Right(x.build()),
        }
    }

    #[inline(always)]
    fn get_resource<T: 'static>(&self) -> Option<&T> {
        match self {
            Either::Left(x) => x.get_resource(),
            Either::Right(x) => x.get_resource(),
        }
    }

    #[inline(always)]
    fn resource<T: 'static>(&self) -> &T {
        match self {
            Either::Left(x) => x.resource(),
            Either::Right(x) => x.resource(),
        }
    }

    #[inline(always)]
    fn get_resource_mut<T: 'static>(&mut self) -> Option<&mut T> {
        match self {
            Either::Left(x) => x.get_resource_mut(),
            Either::Right(x) => x.get_resource_mut(),
        }
    }

    #[inline(always)]
    fn resource_mut<T: 'static>(&mut self) -> &mut T {
        match self {
            Either::Left(x) => x.resource_mut(),
            Either::Right(x) => x.resource_mut(),
        }
    }

    #[inline(always)]
    fn get_component<T: 'static>(&self, entity: Entity) -> Option<&T> {
        match self {
            Either::Left(x) => x.get_component(entity),
            Either::Right(x) => x.get_component(entity),
        }
    }

    #[inline(always)]
    fn component<T: 'static>(&self, entity: Entity) -> &T {
        match self {
            Either::Left(x) => x.component(entity),
            Either::Right(x) => x.component(entity),
        }
    }

    #[inline(always)]
    fn get_component_mut<T: 'static>(&mut self, entity: Entity) -> Option<&mut T> {
        match self {
            Either::Left(x) => x.get_component_mut(entity),
            Either::Right(x) => x.get_component_mut(entity),
        }
    }

    #[inline(always)]
    fn component_mut<T: 'static>(&mut self, entity: Entity) -> &mut T {
        match self {
            Either::Left(x) => x.component_mut(entity),
            Either::Right(x) => x.component_mut(entity),
        }
    }

    #[inline(always)]
    fn query<'w, F, Q>(&'w mut self, f: F)
    where
        F: FnMut(<Q as WorldQuery>::Item<'w>),
        Q: WorldQuery,
    {
        match self {
            Either::Left(x) => x.query::<F, Q>(f),
            Either::Right(x) => x.query::<F, Q>(f),
        }
    }

    #[inline(always)]
    fn extend_config<Param: ParamLabel, T: Clone + 'static>(
        self,
        config: T,
    ) -> Self::ExtendConfig<Param, T> {
        match self {
            Either::Left(x) => Either::Left(x.extend_config::<Param, T>(config)),
            Either::Right(x) => Either::Right(x.extend_config::<Param, T>(config)),
        }
    }

    #[inline(always)]
    fn extend_generic_config<System: 'static, Param: 'static, T: Clone + 'static>(
        self,
        config: T,
    ) -> Self::ExtendGenericConfig<System, Param, T> {
        match self {
            Either::Left(x) => Either::Left(x.extend_generic_config::<System, Param, T>(config)),
            Either::Right(x) => Either::Right(x.extend_generic_config::<System, Param, T>(config)),
        }
    }
}

pub trait EcsBuilderIter {
    type ForEachOwned<O: EcsBuilderOperation, B: EcsBuilder>: EcsBuilder;
    fn for_each_owned<O, B>(self, builder: B, operation: O) -> Self::ForEachOwned<O, B>
    where
        O: EcsBuilderOperation,
        B: EcsBuilder;
}

impl<N: EcsBuilderIter, T> EcsBuilderIter for Nested<N, T> {
    type ForEachOwned<O: EcsBuilderOperation, B: EcsBuilder> = N::ForEachOwned<O, O::Apply<B>>;

    #[inline(always)]
    fn for_each_owned<O: EcsBuilderOperation, B: EcsBuilder>(
        self,
        builder: B,
        mut operation: O,
    ) -> Self::ForEachOwned<O, B> {
        self.inner
            .for_each_owned(operation.apply(builder), operation)
    }
}

impl EcsBuilderIter for StackedNest {
    type ForEachOwned<O: EcsBuilderOperation, B: EcsBuilder> = B;

    #[inline(always)]
    fn for_each_owned<O: EcsBuilderOperation, B: EcsBuilder>(
        self,
        builder: B,
        _operation: O,
    ) -> Self::ForEachOwned<O, B> {
        builder
    }
}
