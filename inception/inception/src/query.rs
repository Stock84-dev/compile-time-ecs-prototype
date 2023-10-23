use core::marker::PhantomData;

use crate::{
    entities::WorldQuery,
    impl_no_plugin,
    system_param::{SystemParam},
    world::World,
    EcsBuilder, Entity, PhantomSystemParam, SystemParamNameMapper,
};

/// Reads entities that have specific components and executes a function on them.
/// # Example
/// ```
/// use inception::*;
/// #[system]
/// fn sum(mut query: Query<(&i32, &mut u32)>, mut total: Res<u32>) {
///     query.run(|(a, b)| {
///         *b += *a as u32;
///         **total += *b;
///     });
/// }
/// ```
pub struct Query<'w, 's, C, W, const N: usize> {
    world: &'w mut W,
    _marker: PhantomSystemParam<'w, 's, N>,
    _params: PhantomData<C>,
}

type QueryItem<'w, Q> = <Q as WorldQuery>::Item<'w>;

impl<'w, 's, C: WorldQuery, W: World, const N: usize> Query<'w, 's, C, W, N> {
    pub fn run<'this, F: FnMut(QueryItem<'this, C>)>(&'this mut self, f: F) {
        self.world.query::<F, C>(f);
    }

    pub fn components<'this>(&'this mut self, entity: Entity) -> QueryItem<'this, C> {
        self.world.components::<C>(entity)
    }

    pub fn run_entity<'this, F: FnMut(QueryItem<'this, C>)>(&'this mut self, entity: Entity, f: F) {
        self.world.query_entity::<F, C>(entity, f);
    }
}

impl<'w, 's, W: World + 'static, Params, const N: usize> SystemParam
    for Query<'w, 's, Params, W, N>
{
    type Item<'world, 'state, Wrld: World> = Query<'world, 'state, Params, Wrld, N>;
    type State = ();

    impl_no_plugin!();

    fn get_param<'world, 'state, Wrld: World, SB: SystemParamNameMapper, ParamName>(
        _state: &'state mut Self::State,
        world: &'world mut Wrld,
    ) -> Self::Item<'world, 'state, Wrld> {
        Query {
            _params: PhantomData,
            world,
            _marker: PhantomSystemParam::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{stages::*, *};

    pub type Update0 = Stage0;
    schedule! {
        struct Schedule,
        Stage0 as Update0,
    }

    #[test]
    fn query() {
        #[system]
        fn sum(mut query: Query<(&i32, &mut u32)>, mut total: Res<u32>) {
            query.run(|(a, b)| {
                *b += *a as u32;
                **total += *b;
            });
            assert_eq!(**total, 6);
        }
        let mut a = Entity::default();
        let mut b = Entity::default();
        let mut ecs = EcsBuilderStruct::new::<_, 0>(ConfigBuilder::new(), Schedule::builder())
            .spawn(&mut a)
            .spawn(&mut b)
            .add_resource(0u32)
            .insert_component(1i32, a)
            .insert_component(1u32, a)
            .insert_component(2i32, b)
            .insert_component(2u32, b)
            .add_system_to_stage(sum::new(), Update0::new())
            .build();
        ecs.run();
        // *ecs.get_resource::<u32>().unwrap()
    }
}
