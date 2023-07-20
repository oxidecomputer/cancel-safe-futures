//! Alternative adapters for asynchronous values.
//!
//! This module contains adapters that are similar to the `try_` adapters in the [`futures::future`]
//! module, but don't cancel other futures if one of them errors out.

#[cfg(feature = "alloc")]
mod join_all_then_try;
#[cfg(feature = "alloc")]
pub use join_all_then_try::{join_all_then_try, JoinAllThenTry};
