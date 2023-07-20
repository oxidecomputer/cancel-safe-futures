//! Alternative adapters for asynchronous streams.
//!
//! This module contains adapters that are similar to the `try_` adapters in the [`futures::stream`]
//! module, but don't cancel other futures if one of them errors out.

mod try_stream;
pub use try_stream::TryStreamExt;

mod tryx_collect;
pub use tryx_collect::TryxCollect;
