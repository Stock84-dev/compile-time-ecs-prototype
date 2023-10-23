use core::marker::PhantomData;
use std::prelude::v1::*;

use either::Either;
use ergnomics::*;

use crate::{
    config::ConfigValue,
    entities::{
        DefaultEntitiesBuilder1, Entities, EntitiesBuilder, EntityFnMut, EntityRelay, WorldQuery,
    },
    nest_module::{Nest, Nested, StackedNest},
    Entity, ParamLabel,
};

pub trait World: Sized + 'static {
    type AddConfig<Param: ParamLabel, Entity: EntityRelay, T: 'static>: World;
    type AddGenericConfig<System: 'static, Param: 'static, Entity: EntityRelay, T: 'static>: World;
    type ExtendConfig<Param: ParamLabel, T: Clone + 'static>: World;
    type ExtendGenericConfig<System: 'static, Param: 'static, T: Clone + 'static>: World;
    type AddResource<T: 'static>: World;
    type InitResource<T: 'static>: World;
    type AddComponent<C: 'static, ER: EntityRelay>: World;
    type ExtendEntities<C: Clone + 'static>: World;

    #[must_use]
    fn add_component<C: 'static, ER: EntityRelay>(
        self,
        component: C,
        entity: ER,
    ) -> Self::AddComponent<C, ER>;
    #[must_use]
    fn extend_entities<C: Clone + 'static>(self, component: C) -> Self::ExtendEntities<C>;
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
    fn config<System: 'static, Param: 'static, Config: 'static>(&self) -> &Config;
    #[must_use]
    fn config_mut<System: 'static, Param: 'static, Config: 'static>(&mut self) -> &mut Config;

    #[must_use]
    fn get_resource<T: 'static>(&self) -> Option<&T>;
    #[must_use]
    fn resource<T: 'static>(&self) -> &T;
    #[must_use]
    fn get_resource_mut<T: 'static>(&mut self) -> Option<&mut T>;
    #[must_use]
    fn resource_mut<T: 'static>(&mut self) -> &mut T;
    #[must_use]
    fn init_resource<T: FromWorld>(self) -> Self::InitResource<T>;
    #[must_use]
    fn add_resource<T: 'static>(self, resource: T) -> Self::AddResource<T>;

    #[must_use]
    fn get_component<T: 'static>(&self, entity: Entity) -> Option<&T>;
    #[must_use]
    fn component<T: 'static>(&self, entity: Entity) -> &T;
    #[must_use]
    fn get_component_mut<T: 'static>(&mut self, entity: Entity) -> Option<&mut T>;
    #[must_use]
    fn component_mut<T: 'static>(&mut self, entity: Entity) -> &mut T;
    #[must_use]
    fn get_components<'w, Q: WorldQuery>(&'w mut self, entity: Entity) -> Option<Q::Item<'w>>;
    #[must_use]
    fn components<'w, Q: WorldQuery>(&'w mut self, entity: Entity) -> Q::Item<'w>;
    fn for_each<F>(&mut self, f: F)
    where
        F: EntityFnMut;
    fn query<'w, F, Q>(&'w mut self, f: F)
    where
        F: FnMut(<Q as WorldQuery>::Item<'w>),
        Q: WorldQuery;
    fn query_entity<'w, F, Q>(&'w mut self, entity: Entity, f: F)
    where
        F: FnMut(<Q as WorldQuery>::Item<'w>),
        Q: WorldQuery;
}

pub trait FromWorld: 'static {
    fn from_world<W>(world: &mut W) -> Self;
}

impl<T: Default + 'static> FromWorld for T {
    #[inline(always)]
    fn from_world<W>(_world: &mut W) -> Self {
        Self::default()
    }
}

pub type BasicWorld = WorldStruct<StackedNest, DefaultEntitiesBuilder1>;

pub struct WorldStruct<R, E> {
    pub(crate) resources: R,
    pub(crate) entities: E,
}

impl<R, Ents> World for WorldStruct<R, Ents>
where
    R: Nest + 'static,
    Ents: Entities + EntitiesBuilder + 'static,
{
    type AddComponent<Comp: 'static, ER: EntityRelay> = WorldStruct<R, Ents::Add<Comp, ER>>;
    type AddConfig<Param: ParamLabel, Entity: EntityRelay, T: 'static> =
        WorldStruct<R, Entity::Add<Ents, ConfigValue<Param::System, Param, T>>>;
    type AddGenericConfig<System: 'static, Param: 'static, Entity: EntityRelay, T: 'static> =
        WorldStruct<R, Entity::Add<Ents, ConfigValue<System, Param, T>>>;
    type AddResource<T: 'static> = WorldStruct<Nested<R, T>, Ents>;
    type ExtendEntities<Comp: Clone + 'static> = WorldStruct<R, Ents::ExtendEntities<Comp>>;
    type ExtendGenericConfig<System: 'static, Param: 'static, T: Clone + 'static> =
        Self::ExtendEntities<ConfigValue<System, Param, T>>;

    type ExtendConfig<Param: ParamLabel, T: Clone + 'static> = impl World;
    type InitResource<T: 'static> = impl World;

    #[inline(always)]
    fn add_component<Comp: 'static, ER: EntityRelay>(
        self,
        component: Comp,
        entity: ER,
    ) -> Self::AddComponent<Comp, ER> {
        WorldStruct {
            resources: self.resources,
            entities: self.entities.add(component, entity),
        }
    }

    #[inline(always)]
    fn extend_entities<Comp: Clone + 'static>(self, component: Comp) -> Self::ExtendEntities<Comp> {
        WorldStruct {
            resources: self.resources,
            entities: self.entities.extend_entities(component),
        }
    }

    #[inline(always)]
    fn add_config<Param: ParamLabel, Entity: EntityRelay, T: 'static>(
        self,
        config: T,
    ) -> Self::AddConfig<Param, Entity, T> {
        self.add_generic_config::<Param::System, Param, Entity, T>(config)
    }

    #[inline(always)]
    fn add_generic_config<System: 'static, Param: 'static, Entity: EntityRelay, T: 'static>(
        self,
        config: T,
    ) -> Self::AddGenericConfig<System, Param, Entity, T> {
        WorldStruct {
            resources: self.resources,
            entities: Entity::add(
                self.entities,
                ConfigValue(config, PhantomData::<(System, Param)>),
            ),
        }
    }

    #[inline(always)]
    fn extend_config<Param: ParamLabel, T: Clone + 'static>(
        self,
        config: T,
    ) -> Self::ExtendConfig<Param, T> {
        self.extend_generic_config::<Param::System, Param, T>(config)
    }

    #[inline(always)]
    fn extend_generic_config<System: 'static, Param: 'static, T: Clone + 'static>(
        self,
        config: T,
    ) -> Self::ExtendGenericConfig<System, Param, T> {
        self.extend_entities(ConfigValue(config, PhantomData::<(System, Param)>))
    }

    #[inline(always)]
    fn config<System: 'static, Param: 'static, Config: 'static>(&self) -> &Config {
        &self
            .resources
            .get::<ConfigValue<System, Param, Config>>()
            .expect_or_else(|| {
                format!(
                    "Config resource `{}` not found",
                    ConfigValue::<System, Param, Config>::type_name()
                )
            })
            .0
    }

    #[inline(always)]
    fn config_mut<System: 'static, Param: 'static, Config: 'static>(&mut self) -> &mut Config {
        &mut self
            .resources
            .get_mut::<ConfigValue<System, Param, Config>>()
            .expect_or_else(|| {
                format!(
                    "Config resource `{}` not found",
                    ConfigValue::<System, Param, Config>::type_name()
                )
            })
            .0
    }

    #[inline(always)]
    fn get_resource<T: 'static>(&self) -> Option<&T> {
        self.resources.get::<T>()
    }

    #[inline(always)]
    fn resource<T: 'static>(&self) -> &T {
        self.resources
            .get::<T>()
            .expect_or_else(|| format!("Resource `{}` not found", T::type_name()))
    }

    #[inline(always)]
    fn get_resource_mut<T: 'static>(&mut self) -> Option<&mut T> {
        self.resources.get_mut::<T>()
    }

    #[inline(always)]
    fn resource_mut<T: 'static>(&mut self) -> &mut T {
        self.resources
            .get_mut::<T>()
            .expect_or_else(|| format!("Resource `{}` not found", T::type_name()))
    }

    #[inline(always)]
    fn init_resource<T: FromWorld>(mut self) -> Self::InitResource<T> {
        let resource = T::from_world(&mut self);
        WorldStruct {
            resources: self.resources.push(resource),
            entities: self.entities,
        }
    }

    #[inline(always)]
    fn add_resource<T: 'static>(self, resource: T) -> Self::AddResource<T> {
        WorldStruct {
            resources: self.resources.push(resource),
            entities: self.entities,
        }
    }

    #[inline(always)]
    fn get_component<T: 'static>(&self, entity: Entity) -> Option<&T> {
        self.entities.get_component::<T>(entity)
    }

    #[inline(always)]
    fn component<T: 'static>(&self, entity: Entity) -> &T {
        self.entities.get_component::<T>(entity).expect_or_else(|| {
            format!(
                "Component `{}` not found in entity `{}`",
                T::type_name(),
                entity.0
            )
        })
    }

    #[inline(always)]
    fn get_component_mut<T: 'static>(&mut self, entity: Entity) -> Option<&mut T> {
        self.entities.get_component_mut::<T>(entity)
    }

    #[inline(always)]
    fn component_mut<T: 'static>(&mut self, entity: Entity) -> &mut T {
        self.entities
            .get_component_mut::<T>(entity)
            .expect_or_else(|| {
                format!(
                    "Component `{}` not found in entity `{}`",
                    T::type_name(),
                    entity.0
                )
            })
    }

    #[inline(always)]
    fn get_components<'w, Q: WorldQuery>(&'w mut self, entity: Entity) -> Option<Q::Item<'w>> {
        self.entities.get_components::<Q>(entity)
    }

    #[inline(always)]
    fn components<'w, Q: WorldQuery>(&'w mut self, entity: Entity) -> Q::Item<'w> {
        self.entities.components::<Q>(entity)
    }

    #[inline(always)]
    fn for_each<F>(&mut self, f: F)
    where
        F: EntityFnMut,
    {
        self.entities.for_each(f);
    }

    #[inline(always)]
    fn query<'w, F, Q>(&'w mut self, f: F)
    where
        F: FnMut(<Q as WorldQuery>::Item<'w>),
        Q: WorldQuery,
    {
        self.entities.query::<F, Q>(f);
    }

    #[inline(always)]
    fn query_entity<'w, F, Q>(&'w mut self, entity: Entity, f: F)
    where
        F: FnMut(<Q as WorldQuery>::Item<'w>),
        Q: WorldQuery,
    {
        self.entities.query_entity::<F, Q>(entity, f);
    }
}
