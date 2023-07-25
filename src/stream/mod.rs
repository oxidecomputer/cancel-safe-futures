//! Alternative adapters for asynchronous streams.
//!
//! This module contains adapters that are similar to the `try_` adapters in the [`futures::stream`]
//! module, but don't cancel other futures if one of them errors out.

mod try_stream;
pub use try_stream::TryStreamExt;

mod collect_then_try;
pub use collect_then_try::CollectThenTry;

#[cfg(not(futures_no_atomic_cas))]
#[cfg(feature = "alloc")]
mod for_each_concurrent_then_try;
#[cfg(feature = "alloc")]
#[allow(unreachable_pub)] // https://github.com/rust-lang/rust/issues/57411
pub use for_each_concurrent_then_try::ForEachConcurrentThenTry;
