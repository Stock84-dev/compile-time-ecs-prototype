use std::prelude::v1::*;

use either::Either;

use crate::{entities::WorldQuery, resources::Break, stage::Stage, world::World, Entity};

pub trait Ecs {
    fn run(&mut self);
    fn should_break_loop(&mut self) -> bool;

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

pub struct EcsStruct<W, S> {
    pub(crate) world: W,
    pub(crate) stages: S,
}

impl<W, S> Ecs for EcsStruct<W, S>
where
    W: World,
    S: Stage<W>,
{
    #[inline(always)]
    fn run(&mut self) {
        self.stages.run(&mut self.world);
    }

    #[inline(always)]
    fn should_break_loop(&mut self) -> bool {
        let break_ = self.world.resource_mut::<Break>();
        if break_.0 {
            break_.0 = false;
            true
        } else {
            false
        }
    }

    #[inline(always)]
    fn get_resource<T: 'static>(&self) -> Option<&T> {
        self.world.get_resource()
    }

    #[inline(always)]
    fn resource<T: 'static>(&self) -> &T {
        self.world.resource()
    }

    #[inline(always)]
    fn get_resource_mut<T: 'static>(&mut self) -> Option<&mut T> {
        self.world.get_resource_mut()
    }

    #[inline(always)]
    fn resource_mut<T: 'static>(&mut self) -> &mut T {
        self.world.resource_mut()
    }

    #[inline(always)]
    fn get_component<T: 'static>(&self, entity: Entity) -> Option<&T> {
        self.world.get_component(entity)
    }

    #[inline(always)]
    fn component<T: 'static>(&self, entity: Entity) -> &T {
        self.world.component(entity)
    }

    #[inline(always)]
    fn get_component_mut<T: 'static>(&mut self, entity: Entity) -> Option<&mut T> {
        self.world.get_component_mut(entity)
    }

    #[inline(always)]
    fn component_mut<T: 'static>(&mut self, entity: Entity) -> &mut T {
        self.world.component_mut(entity)
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

impl<L: Ecs, R: Ecs> Ecs for Either<L, R> {
    #[inline(always)]
    fn run(&mut self) {
        match self {
            Either::Left(l) => l.run(),
            Either::Right(r) => r.run(),
        }
    }

    #[inline(always)]
    fn should_break_loop(&mut self) -> bool {
        match self {
            Either::Left(l) => l.should_break_loop(),
            Either::Right(r) => r.should_break_loop(),
        }
    }

    #[inline(always)]
    fn get_resource<T: 'static>(&self) -> Option<&T> {
        match self {
            Either::Left(l) => l.get_resource(),
            Either::Right(r) => r.get_resource(),
        }
    }

    #[inline(always)]
    fn resource<T: 'static>(&self) -> &T {
        match self {
            Either::Left(l) => l.resource(),
            Either::Right(r) => r.resource(),
        }
    }

    #[inline(always)]
    fn get_resource_mut<T: 'static>(&mut self) -> Option<&mut T> {
        match self {
            Either::Left(l) => l.get_resource_mut(),
            Either::Right(r) => r.get_resource_mut(),
        }
    }

    #[inline(always)]
    fn resource_mut<T: 'static>(&mut self) -> &mut T {
        match self {
            Either::Left(l) => l.resource_mut(),
            Either::Right(r) => r.resource_mut(),
        }
    }

    #[inline(always)]
    fn get_component<T: 'static>(&self, entity: Entity) -> Option<&T> {
        match self {
            Either::Left(l) => l.get_component(entity),
            Either::Right(r) => r.get_component(entity),
        }
    }

    #[inline(always)]
    fn component<T: 'static>(&self, entity: Entity) -> &T {
        match self {
            Either::Left(l) => l.component(entity),
            Either::Right(r) => r.component(entity),
        }
    }

    #[inline(always)]
    fn get_component_mut<T: 'static>(&mut self, entity: Entity) -> Option<&mut T> {
        match self {
            Either::Left(l) => l.get_component_mut(entity),
            Either::Right(r) => r.get_component_mut(entity),
        }
    }

    #[inline(always)]
    fn component_mut<T: 'static>(&mut self, entity: Entity) -> &mut T {
        match self {
            Either::Left(l) => l.component_mut(entity),
            Either::Right(r) => r.component_mut(entity),
        }
    }

    #[inline(always)]
    fn query<'w, F, Q>(&'w mut self, f: F)
    where
        F: FnMut(<Q as WorldQuery>::Item<'w>),
        Q: WorldQuery,
    {
        match self {
            Either::Left(l) => l.query::<F, Q>(f),
            Either::Right(r) => r.query::<F, Q>(f),
        }
    }
}
