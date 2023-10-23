use core::marker::PhantomData;

use crate::{
    ecs_builder::EcsBuilder,
    nest_module::{Nest, Nestable, Nested, StackedNest},
    resources::Break,
};

pub trait Plugin {
    type Deps<L: PluginLoader>: PluginLoader;
    type Build<B: EcsBuilder>: EcsBuilder;

    fn deps<L: PluginLoader>(&mut self, loader: L) -> Self::Deps<L>;
    fn build<B: EcsBuilder>(self, builder: B) -> Self::Build<B>;
}

pub trait SystemParamPlugin {
    type Build<B: EcsBuilder>: EcsBuilder;

    fn build<B: EcsBuilder>(builder: B) -> Self::Build<B>;
}

pub trait PluginLoader: sealed::IntoEcsBuilder {
    type Finish: EcsBuilder;
    type Load<P: Plugin + 'static>: PluginLoader + sealed::IntoEcsBuilder;
    type LoadMultiple<P: Plugin + 'static>: PluginLoader + sealed::IntoEcsBuilder;

    fn load_once<Plug: Plugin + 'static>(self, plugin: Plug) -> Self::Load<Plug>;
    fn load<Plug: Plugin + 'static>(self, plugin: Plug) -> Self::LoadMultiple<Plug>;
}

pub trait Plugins {
    type Push<Plugin: 'static>: Plugins;

    fn contains_plugin<Plugin: 'static>(&self) -> bool;
    fn push<Plugin: 'static>(self) -> Self::Push<Plugin>;
}

pub struct CorePlugin;

impl Plugin for CorePlugin {
    type Build<B: EcsBuilder> = B::AddResource<Break>;
    type Deps<L: PluginLoader> = L;

    fn deps<L: PluginLoader>(&mut self, loader: L) -> Self::Deps<L> {
        loader
    }

    fn build<B: EcsBuilder>(self, builder: B) -> Self::Build<B> {
        builder.add_resource(Break(false))
    }
}

pub(crate) mod sealed {
    pub trait IntoEcsBuilder {
        type Builder: crate::EcsBuilder;
        /// Used to prevent users form building plugins in Plugin::deps function. Don't use it.
        fn __into_builder(self) -> Self::Builder;
    }
}

impl<T: PluginLoader + EcsBuilder> sealed::IntoEcsBuilder for T {
    type Builder = T;

    fn __into_builder(self) -> Self::Builder {
        // SAFETY: the type is the same. Cannot just return self.
        unsafe {
            let new = core::mem::transmute_copy(&self);
            core::mem::forget(self);
            new
        }
    }
}

impl<N: Nest + Plugins, P: 'static> Plugins for Nested<N, PhantomData<P>> {
    type Push<Plugin: 'static> = Nested<Self, PhantomData<Plugin>>;

    fn contains_plugin<Plugin: 'static>(&self) -> bool {
        if core::any::TypeId::of::<P>() == core::any::TypeId::of::<Plugin>() {
            true
        } else {
            self.inner.contains_plugin::<Plugin>()
        }
    }

    fn push<Plugin: 'static>(self) -> Self::Push<Plugin> {
        <Self as Nestable>::push(self, PhantomData::<Plugin>)
    }
}

impl Plugins for StackedNest {
    type Push<Plugin: 'static> = Nested<Self, PhantomData<Plugin>>;

    fn contains_plugin<Plugin: 'static>(&self) -> bool {
        false
    }

    fn push<Plugin: 'static>(self) -> Self::Push<Plugin> {
        <Self as Nestable>::push(self, PhantomData::<Plugin>)
    }
}
