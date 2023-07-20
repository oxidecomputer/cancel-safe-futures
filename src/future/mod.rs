//! Alternative adapters for asynchronous values.
//!
//! This module contains adapters that are similar to the `try_` adapters in the [`futures::future`]
//! module, but don't cancel other futures if one of them errors out.

#[cfg(feature = "alloc")]
mod tryx_join_all;
#[cfg(feature = "alloc")]
pub use tryx_join_all::{tryx_join_all, TryxJoinAll};
