#![feature(generic_associated_types)]
#![feature(type_alias_impl_trait)]
#![feature(type_name_of_val)]
// CUDA compiler uses old toolchain
#![allow(stable_features)]
#![no_std]

//! # ESL - Engine standard library
//! Example is in `workspace/lstok` and `examples` directory.

pub use esl_macros::{impl_metric, indicator, input, metric, resource_value, strategy, Value};
pub use inception::{self, *};
pub use inception_macros::system;

pub use crate::{
    backtest::{
        hlcv_backtest::HlcvBacktestPlugin,
        orderflow_backtest::OrderflowBacktestPlugin,
        BacktestPlugin,
    },
    components::*,
    events::*,
    indicator::{ComputeIndicatorPlugin, Indicator},
    inputs::*,
    loop_index::LoopIndex,
    metrics::{Sum, *},
    metrics_builder::*,
    order::*,
    orders::Orders,
    param::{HyperParam, ParamConfig},
    plugin::CorePlugin,
    prev::Prev,
    resources::*,
    schema::Reader,
    series::Series,
    // ta::yata::Ind,
    value::Value,
};

extern crate no_std_compat as std;
// Makes doctests and tests pass when they are using proc macros.
extern crate self as esl;

mod backtest;
pub mod components;
mod events;
mod indicator;
pub mod inputs;
mod loop_index;
pub mod metrics;
mod metrics_builder;
mod order;
mod orders;
pub mod param;
mod plugin;
mod prev;
mod print;
pub mod resources;
mod schema;
mod series;
pub mod stages;
pub mod ta;
pub mod types;
mod value;
