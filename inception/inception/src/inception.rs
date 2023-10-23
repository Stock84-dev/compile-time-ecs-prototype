#![feature(generic_associated_types)]
#![feature(type_alias_impl_trait)]
#![feature(type_name_of_val)]
// CUDA compiler uses old toolchain
#![allow(stable_features)]
#![no_std]

//! # Inception
//! Inception is a zero-cost library for building ECS based applications. It is heavily inspired by
//! [bevy](https://https://bevyengine.org/). It is designed to be used in a single threaded
//! environment. Everything is stack based. It uses a lot of generics and different tricks to
//! inline as much as possible. It's common that the type name of an ECS is measured in millions of
//! characters.
//!
//! Rust can optimize this quite well. Pretty much there is no cost of abstraction. But, that comes
//! at a cost. Compile times are very long, more RAM is required and the output binary is quite
//! large. [Usage of a ramdisk is recommended](https://endler.dev/2020/rust-compile-times/#use-a-ramdisk-for-compilation).
//!
//! ## Different features from Bevy
//! - Systems can be called for each entity instead of only once per stage.
//! The below systems are equivalent.
//!
//! ### Example
//! ```rust
//! use inception::*;
//! #[system]
//! fn sum_query(mut query: Query<(&i32, &mut u32)>, mut total: Res<u32>) {
//!     query.run(|(a, b)| {
//!         *b += *a as u32;
//!         **total += *b;
//!     });
//! }
//!
//! #[system]
//! fn sum(a: &i32, b: &mut u32, mut total: Res<u32>) {
//!     *b += *a as u32;
//!     **total += *b;
//! }
//! ```
//! - It has concept of configs. Configs are structs that can be used to configure the behavior of
//!   systems. They are separate from ECS builder so tha they could be more easily generated
//!   dynamically.
//! - Parameters can be passed to systems. If a system has a parameter of type `In<T>` or
//!   `Phantom<T>` then a parameter of type `T` would be required in the constructor of a system.
//! - Plugins have dependencies.
//! - System parameters can load plugins so that user doesn't forget to load them manually.
//! - Schedule can contain loops with stages.
//! - Events have a fixed size buffer. All event buffers have the same size, it can be configured
//!   through `EcsBuilder`.
//! - There are no commands, so no dynamic insertion of components and resources.

pub use inception_macros::{nest, schedule, system, system_param, SystemParamPlugin};
pub use static_assertions;

pub use crate::{
    config::EntityConfig,
    ecs::Ecs,
    ecs_builder::{EcsBuilder, EcsBuilderIter, EcsBuilderOperation, EcsBuilderStruct},
    entities::*,
    events::{EntityEvents, Events, EventsTag},
    input::{In, Input, InputItem, PhantomIn},
    nest_module::{Nest, Nestable, Nested, StackedNest},
    plugin::{CorePlugin, Plugin, PluginLoader, SystemParamPlugin},
    query::Query,
    schedule::ScheduleBuilderTrait,
    stage::{AddSystemToStageCommand, Stage, StageBuilder},
    stages::Last,
    system::{IntoInferredSystem, System, SystemBuilder, SystemState},
    system_param::{
        EntityParam, Mapper, ParamLabel, PhantomSystemParam, Res, SystemParam,
        SystemParamNameMapper, SystemParamState, UnknownSystem,
    },
    world::{FromWorld, World},
};

extern crate no_std_compat as std;

#[macro_use]
extern crate derive_more;

// Makes doctests and tests pass when they are using proc macros.
extern crate self as inception;

pub mod config;
mod ecs;
mod ecs_builder;
mod entities;
mod events;
mod input;
mod nest_module;
mod plugin;
mod query;
pub mod resources;
mod schedule;
mod stage;
pub mod stages;
mod system;
mod system_param;
mod world;
