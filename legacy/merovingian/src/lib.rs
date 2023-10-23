#![deny(unused_must_use)]
#![feature(extend_one)]
#![feature(try_blocks)]
#![feature(bufreader_seek_relative)]
#![feature(option_result_unwrap_unchecked)]

#[allow(pub_use_of_private_extern_crate)]
#[macro_use]
extern crate macros;
#[macro_use]
extern crate log;
#[macro_use]
pub extern crate speedy;
#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate serde;
//#[macro_use]
// extern crate num_enum; // do not load, it overrides `Default` derive macro
#[macro_use]
extern crate async_trait;
#[macro_use]
extern crate thiserror;
#[macro_use]
extern crate mouse;

use std::sync::Arc;

pub use serde::{Deserialize, Serialize};
pub use speedy::{Readable, Writable};
pub use uuid;

pub mod candles;
pub mod candles_builder;
pub mod error;
pub mod hlcv;
pub mod minable_models;
pub mod non_minable_models;
pub mod variable;
