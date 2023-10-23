use tinyvec::ArrayVec;

use crate::{entities::EntityFetch, *};

#[derive(Clone)]
pub struct EventsContainer<T, const N: usize> {
    events: ArrayVec<[Option<T>; N]>,
}

#[derive(Default, Clone, Debug, PartialEq)]
pub struct EventsTag;

impl<T, const N: usize> Default for EventsContainer<T, N> {
    #[inline(always)]
    fn default() -> Self {
        Self {
            events: ArrayVec::new(),
        }
    }
}

/// Stores events in a resource. It can only contain `N` events and they are cleared on
/// `inception::stages::Last`. Events are shared between systems and cannot be consumed.
pub struct Events<'w, 's, T, const N: usize> {
    res: &'w mut EventsContainer<T, N>,
    _marker: PhantomSystemParam<'w, 's, N>,
}

impl<'w, 's, T: 'static, const N: usize> SystemParam for Events<'w, 's, T, N> {
    // cast lifetimes
    type Item<'world, 'state, Wrld: World> = Events<'world, 'state, T, N>;
    type State = ();

    type Build<B: EcsBuilder, SB: SystemParamNameMapper + 'static, ParamName: 'static> =
        impl EcsBuilder;

    #[inline(always)]
    fn get_param<'world, 'state, Wrld: World, SB: SystemParamNameMapper, ParamName>(
        _state: &'state mut Self::State,
        world: &'world mut Wrld,
    ) -> Self::Item<'world, 'state, Wrld> {
        Events {
            res: world.resource_mut::<EventsContainer<T, N>>(),
            _marker: Default::default(),
        }
    }

    #[inline(always)]
    fn build<B: EcsBuilder, SB: SystemParamNameMapper + 'static, ParamName: 'static>(
        builder: B,
    ) -> Self::Build<B, SB, ParamName> {
        builder
            .add_system_without_plugin(events::new::<T>(), crate::stages::Last::new())
            .init_resource::<EventsContainer<T, N>>()
    }
}

#[system]
fn events<T: 'static>(mut channel: Res<EventsContainer<T, N>>) {
    channel.events.clear();
}

impl<'w, 's, T, const N: usize> Events<'w, 's, T, N> {
    #[inline(always)]
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.res.events.iter().filter_map(|x| x.as_ref())
    }

    #[inline(always)]
    pub fn send(&mut self, event: T) {
        self.res.events.push(Some(event));
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.res.events.is_empty()
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.res.events.len()
    }

    #[inline(always)]
    pub fn clear(&mut self) {
        self.res.events.clear();
    }
}

impl<'w, 's, T: 'static, const N: usize> IntoIterator for Events<'w, 's, T, N> {
    type Item = &'w T;

    type IntoIter = impl Iterator<Item = &'w T>;

    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        self.res.events.iter().filter_map(|x| x.as_ref())
    }
}

/// Each entity that has an `EventTag` component will be extended with an `EventsContainer`. This
/// allows to store multiple events per entity.
/// It can only contain `N` events and they are cleared on `inception::stages::Last`.
/// Events are shared between systems and cannot be consumed.
pub struct EntityEvents<'w, 's, T, const N: usize> {
    container: &'w mut EventsContainer<T, N>,
    _marker: PhantomSystemParam<'w, 's, N>,
}

impl<'w, 's, T: Clone + 'static, const N: usize> SystemParam for EntityEvents<'w, 's, T, N> {
    // cast lifetimes
    type Item<'world, 'state, Wrld: World> = EntityEvents<'world, 'state, T, N>;
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
        ParamName: 'static,
    {
        Some(EntityEvents {
            container: entity.get_component_mut::<EventsContainer<T, N>>()?,
            _marker: Default::default(),
        })
    }

    #[inline(always)]
    fn build<B: EcsBuilder, SB: SystemParamNameMapper + 'static, ParamName: 'static>(
        builder: B,
    ) -> Self::Build<B, SB, ParamName> {
        builder
            .add_system_without_plugin(
                entity_events::new::<T>(),
                crate::stages::Last::new(),
            )
            .extend_entities(EventsContainer::<T, N>::default())
    }
}

#[system]
fn entity_events<T: Clone + 'static>(events: EntityEvents<T>) {
    events.container.events.clear();
}

impl<'w, 's, T, const N: usize> EntityEvents<'w, 's, T, N> {
    #[inline(always)]
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.container.events.iter().filter_map(|x| x.as_ref())
    }

    #[inline(always)]
    pub fn send(&mut self, event: T) {
        self.container.events.push(Some(event));
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.container.events.is_empty()
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.container.events.len()
    }

    #[inline(always)]
    pub fn clear(&mut self) {
        self.container.events.clear();
    }
}

impl<'w, 's, T: 'static, const N: usize> IntoIterator for EntityEvents<'w, 's, T, N> {
    type Item = &'w T;

    type IntoIter = impl Iterator<Item = &'w T>;

    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        self.container.events.iter().filter_map(|x| x.as_ref())
    }
}
