#![deny(unused_must_use)]
#![feature(thread_id_value)]
#![feature(auto_traits)]
#![feature(backtrace)]
#![feature(specialization)]
#![feature(pattern)]
#![feature(negative_impls)]
#![feature(const_trait_impl)]
#![feature(const_size_of_val)]
#![feature(associated_type_bounds)]
#![feature(extend_one)]

#[macro_use]
extern crate async_trait;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate derive_new;
#[macro_use]
extern crate thiserror;
#[macro_use]
extern crate derive_more;
#[macro_use]
extern crate bytemuck;

pub use bytemuck::*;
pub use bytes;
pub use derive_more::*;
pub use derive_new::*;
pub use io_arc;
pub use rand;
pub use smallbox;
pub use thiserror::*;

#[macro_use]
pub mod macros;
pub mod definitions;
pub mod error;
pub mod ext;
pub mod handlers;
pub mod helpers;
pub mod log;
pub mod mem;
pub mod num;
pub mod stream;
pub mod sync;
pub mod thread_pool;
pub mod time;
pub mod traits;
pub mod websocket;

pub use rayon;
pub use serde;

pub mod derive {
    pub use serde::Serialize;
    //    pub use serde::{Serialize, Deserialize};
}

pub mod futures_util {
    pub use futures_util::*;
}

pub mod dyn_clone {
    pub use dyn_clone::*;
}

pub mod prelude {
    pub use tracing::instrument;
    pub use async_for::*;
    // Needed for async_for
    pub use pin_utils;
    pub use fomat_macros::*;
    pub use async_trait::async_trait;
    pub use bitflags::bitflags;
    pub use derive_more::*;
    pub use derive_new::new;
    pub use downcast_rs::*;
    pub use itertools::Itertools;
    pub use lazy_static::lazy_static;
    pub use num_traits::FloatConst;
    pub use path_slash::{PathBufExt as PathBufSlashExt, PathExt as PathSlashExt};
    pub use rayon::prelude::*;
    pub use smallstr::SmallString;
    pub use smallvec::*;
    pub use static_assertions::*;
    pub use thiserror::Error;

    pub use crate::{
        error::{
            anyhow, bail, ensure, AnyResult, ErrCtxExt, Error, ErrorExt, FailureExt, Result,
            ResultCtxExt,
        },
        time::{Timestamp, IntoDateTime},
        ext::*,
        helpers::*,
        log::{debug, error, info, trace, warn},
        ok, ok_loop, path, ready, ready_loop, some, some_loop,
    };
}

pub mod minipre {
    pub use minipre::{process, process_str, Context as PreprocessorContext, Error};
}
