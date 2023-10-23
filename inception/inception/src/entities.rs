use all_tuples::{all_tuples, repeat};

use crate::{
    config::ConfigValue,
    nest_module::{Nested, StackedNest},
};

macro_rules! def_relays {
    ($insert_fn:ident, $relay:ident, $insert_ty:ident) => {
        pub struct $relay;
        impl EntityRelay for $relay {
            type Add<E: EntitiesBuilder + Entities, C: 'static> = E::$insert_ty<C>;

            #[inline(always)]
            fn add<E: EntitiesBuilder + Entities, C: 'static>(
                builder: E,
                component: C,
            ) -> Self::Add<E, C> {
                builder.$insert_fn(component)
            }
        }
    };
}

macro_rules! def_insertions_for_trait {
    ($t:ident) => {
        macro_rules! def_insertions {
            ($insert: ident,$insert_fn: ident) => {
                type $insert<C: 'static>: $t + Entities + 'static;
                fn $insert_fn<C: 'static>(self, component: C) -> Self::$insert<C>;
            };
        }
    };
}

def_insertions_for_trait!(EntitiesBuilder);

macro_rules! def_all {
    ($n:literal) => {
        macro_rules! def_entities_builder {
            ($name: ident) => {
                inception_macros::entities_builder!($name, $n);
            };
        }
        repeat!(def_relays, 0, $n, add, Entity, Add);
        repeat!(def_entities_builder, 1, $n, EntitiesBuilderStruct);
        pub trait EntitiesBuilder {
            type Add<C: 'static, ER: EntityRelay>: EntitiesBuilder + Entities + 'static;
            type ExtendEntities<C: Clone + 'static>: EntitiesBuilder + Entities;
            repeat!(def_insertions, 0, $n, Add, add);
            fn add<C: 'static, ER: EntityRelay>(self, component: C, entity: ER)
            -> Self::Add<C, ER>;
            fn extend_entities<C: Clone + 'static>(self, component: C) -> Self::ExtendEntities<C>;
        }
    };
}
def_all!(16);

pub trait EntityRelay {
    type Add<E: EntitiesBuilder + Entities, C: 'static>: EntitiesBuilder + Entities + 'static;
    fn add<E: EntitiesBuilder + Entities, C: 'static>(builder: E, component: C) -> Self::Add<E, C>;
}

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Entity(pub usize);

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ResourceEntity;

impl core::fmt::Display for Entity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<usize> for Entity {
    #[inline(always)]
    fn from(a: usize) -> Self {
        Self(a)
    }
}

impl From<ResourceEntity> for Entity {
    #[inline(always)]
    fn from(_value: ResourceEntity) -> Self {
        Self(usize::MAX)
    }
}

pub trait EntityFnMut {
    fn call_mut<E: EntityFetch>(&mut self, entity: &mut E);
}

pub trait EntityFetch {
    fn get_component<T: 'static>(&self) -> Option<&T>;
    fn get_component_mut<T: 'static>(&mut self) -> Option<&mut T>;
    fn component<T: 'static>(&self) -> &T;
    fn component_mut<T: 'static>(&mut self) -> &mut T;
    fn entity(&self) -> Entity;
    fn config<System: 'static, Param: 'static, Config: 'static>(&self) -> &Config;
    fn config_mut<System: 'static, Param: 'static, Config: 'static>(&mut self) -> &mut Config;
}

pub trait EntityComponent {
    fn get_component<T: 'static>(&self) -> Option<&T>;
    fn get_component_mut<T: 'static>(&mut self) -> Option<&mut T>;
}

impl<N, F> EntityComponent for Nested<N, F>
where
    N: EntityComponent,
    F: 'static,
{
    #[inline(always)]
    fn get_component<T: 'static>(&self) -> Option<&T> {
        if core::any::TypeId::of::<T>() == core::any::TypeId::of::<F>() {
            unsafe { Some(&*(&self.item as *const F as *const T)) }
        } else {
            self.inner.get_component::<T>()
        }
    }

    #[inline(always)]
    fn get_component_mut<T: 'static>(&mut self) -> Option<&mut T> {
        if core::any::TypeId::of::<T>() == core::any::TypeId::of::<F>() {
            unsafe { Some(&mut *(&mut self.item as *mut F as *mut T)) }
        } else {
            self.inner.get_component_mut::<T>()
        }
    }
}

impl EntityComponent for StackedNest {
    #[inline(always)]
    fn get_component<T: 'static>(&self) -> Option<&T> {
        None
    }

    #[inline(always)]
    fn get_component_mut<T: 'static>(&mut self) -> Option<&mut T> {
        None
    }
}

#[derive(Debug)]
pub struct EntityData<C> {
    entity: Entity,
    components: C,
}

impl<C: EntityComponent> EntityData<C> {
    #[inline(always)]
    fn query<'w, F, Q>(&'w mut self, f: &mut F)
    where
        F: FnMut(<Q as WorldQuery>::Item<'w>),
        Q: WorldQuery,
    {
        unsafe {
            if let Some(x) = Q::get_component(&self.components) {
                f(x);
            }
        }
    }

    #[inline(always)]
    fn query_entity<'w, F, Q>(&'w mut self, entity: Entity, f: &mut F)
    where
        F: FnMut(<Q as WorldQuery>::Item<'w>),
        Q: WorldQuery,
    {
        if entity == self.entity {
            self.query::<F, Q>(f);
        }
    }

    pub fn add<T: 'static>(self, component: T) -> EntityData<Nested<C, T>> {
        EntityData {
            entity: self.entity,
            components: Nested::new(component, self.components),
        }
    }
}

impl EntityData<StackedNest> {
    pub fn new(entity: Entity) -> Self {
        Self {
            entity,
            components: StackedNest,
        }
    }
}

impl<C: EntityComponent> EntityFetch for EntityData<C> {
    #[inline(always)]
    fn get_component<T: 'static>(&self) -> Option<&T> {
        self.components.get_component()
    }

    #[inline(always)]
    fn get_component_mut<T: 'static>(&mut self) -> Option<&mut T> {
        self.components.get_component_mut()
    }

    #[inline(always)]
    fn component<T: 'static>(&self) -> &T {
        match self.get_component() {
            Some(x) => x,
            None => panic!(
                "Entity `{}` doesn't have `{}` component",
                self.entity,
                std::any::type_name::<T>()
            ),
        }
    }

    #[inline(always)]
    fn component_mut<T: 'static>(&mut self) -> &mut T {
        let entity = self.entity;
        if let Some(x) = self.get_component_mut() {
            return x;
        }
        panic!(
            "Entity `{}` doesn't have `{}` component",
            entity,
            std::any::type_name::<T>()
        )
    }

    #[inline(always)]
    fn entity(&self) -> Entity {
        self.entity
    }

    #[inline(always)]
    fn config<System: 'static, Param: 'static, Config: 'static>(&self) -> &Config {
        &self.component::<ConfigValue<System, Param, Config>>().0
    }

    #[inline(always)]
    fn config_mut<System: 'static, Param: 'static, Config: 'static>(&mut self) -> &mut Config {
        &mut self.component_mut::<ConfigValue<System, Param, Config>>().0
    }
}

pub trait WorldQuery {
    type Component: 'static;
    type Item<'w>: WorldQuery<Component = Self::Component>;

    unsafe fn get_component<'w, E: EntityComponent>(entity: &'w E) -> Option<Self::Item<'w>>;
}

impl<T: 'static> WorldQuery for &T {
    type Component = T;
    type Item<'w> = &'w T;

    #[inline(always)]
    unsafe fn get_component<'w, E: EntityComponent>(entity: &'w E) -> Option<Self::Item<'w>> {
        entity.get_component::<T>()
    }
}

impl<T: 'static> WorldQuery for &mut T {
    type Component = T;
    type Item<'w> = &'w mut T;

    #[inline(always)]
    unsafe fn get_component<'w, E: EntityComponent>(entity: &'w E) -> Option<Self::Item<'w>> {
        // #[allow(clippy::cast_ref_to_mut)]
        // let entity = &mut *(entity as *const E as *mut E);
        #[allow(mutable_transmutes)]
        let entity = core::mem::transmute::<&'w E, &'w mut E>(entity);
        entity.get_component_mut::<T>()
    }
}

macro_rules! impl_component_ref {
    ($($param: ident),*) => {
        impl<$($param),*> WorldQuery for ($($param,)*)
        where
            $($param: WorldQuery),*
        {
            type Component = ($($param::Component,)*);
            type Item<'w> = ($($param::Item<'w>,)*);

            #[allow(unused_variables)]
            #[inline(always)]
            unsafe fn get_component<'w, E: EntityComponent>(entity: &'w E) -> Option<Self::Item<'w>> {
                Some(($($param::get_component(entity)?,)*))
            }
        }
    };
}

all_tuples!(impl_component_ref, 0, 16, P);

pub trait Entities {
    fn query<'w, F, Q>(&'w mut self, f: F)
    where
        F: FnMut(<Q as WorldQuery>::Item<'w>),
        Q: WorldQuery;
    fn query_entity<'w, F, Q>(&'w mut self, entity: Entity, f: F)
    where
        F: FnMut(<Q as WorldQuery>::Item<'w>),
        Q: WorldQuery;
    fn for_each<F>(&mut self, f: F)
    where
        F: EntityFnMut;
    fn get_components<'w, Q: WorldQuery>(&'w mut self, entity: Entity) -> Option<Q::Item<'w>>;
    fn components<'w, Q: WorldQuery>(&'w mut self, entity: Entity) -> Q::Item<'w>;
    fn get_component<T: 'static>(&self, entity: Entity) -> Option<&T>;
    fn get_component_mut<T: 'static>(&mut self, entity: Entity) -> Option<&mut T>;
}

impl<N, C> Entities for Nested<N, EntityData<C>>
where
    N: Entities,
    C: EntityComponent,
{
    #[inline(always)]
    fn query<'w, F, Q>(&'w mut self, mut f: F)
    where
        F: FnMut(<Q as WorldQuery>::Item<'w>),
        Q: WorldQuery,
    {
        unsafe {
            if let Some(x) = Q::get_component(&self.item.components) {
                f(x);
            }
            self.inner.query::<F, Q>(f)
        }
    }

    #[inline(always)]
    fn query_entity<'w, F, Q>(&'w mut self, entity: Entity, f: F)
    where
        F: FnMut(<Q as WorldQuery>::Item<'w>),
        Q: WorldQuery,
    {
        if entity == self.item.entity {
            self.query::<F, Q>(f);
        } else {
            self.query_entity::<F, Q>(entity, f);
        }
    }

    #[inline(always)]
    fn for_each<F>(&mut self, mut f: F)
    where
        F: EntityFnMut,
    {
        f.call_mut(&mut self.item);
        self.inner.for_each(f);
    }

    #[inline(always)]
    fn get_components<'w, Q: WorldQuery>(&'w mut self, entity: Entity) -> Option<Q::Item<'w>> {
        unsafe {
            if entity == self.item.entity {
                Q::get_component(&self.item.components)
            } else {
                self.inner.get_components::<Q>(entity)
            }
        }
    }

    #[inline(always)]
    fn components<'w, Q: WorldQuery>(&'w mut self, entity: Entity) -> Q::Item<'w> {
        expect_components::<Q, _>(entity, self.get_components::<Q>(entity))
    }

    #[inline(always)]
    fn get_component<T: 'static>(&self, entity: Entity) -> Option<&T> {
        if entity == self.item.entity {
            self.item.components.get_component()
        } else {
            self.inner.get_component(entity)
        }
    }

    #[inline(always)]
    fn get_component_mut<T: 'static>(&mut self, entity: Entity) -> Option<&mut T> {
        if entity == self.item.entity {
            self.item.components.get_component_mut()
        } else {
            self.inner.get_component_mut(entity)
        }
    }
}

impl Entities for StackedNest {
    #[inline(always)]
    fn query<'w, F, Q>(&'w mut self, _f: F)
    where
        F: FnMut(<Q as WorldQuery>::Item<'w>),
        Q: WorldQuery,
    {
    }

    #[inline(always)]
    fn query_entity<'w, F, Q>(&'w mut self, _entity: Entity, _f: F)
    where
        F: FnMut(<Q as WorldQuery>::Item<'w>),
        Q: WorldQuery,
    {
    }

    #[inline(always)]
    fn get_components<'w, Q: WorldQuery>(&'w mut self, _entity: Entity) -> Option<Q::Item<'w>> {
        None
    }

    #[inline(always)]
    fn components<'w, Q: WorldQuery>(&'w mut self, entity: Entity) -> Q::Item<'w> {
        expect_components::<Q, _>(entity, None)
    }

    #[inline(always)]
    fn get_component<T: 'static>(&self, _entity: Entity) -> Option<&T> {
        None
    }

    #[inline(always)]
    fn get_component_mut<T: 'static>(&mut self, _entity: Entity) -> Option<&mut T> {
        None
    }

    #[inline(always)]
    fn for_each<F>(&mut self, _f: F)
    where
        F: EntityFnMut,
    {
        // Empty
    }
}

#[inline(always)]
fn expect_components<Q, T>(entity: Entity, components: Option<T>) -> T {
    match components {
        Some(x) => x,
        None => panic!(
            "Entity `{}` with `{}` components not found",
            entity,
            core::any::type_name::<Q>()
        ),
    }
}
