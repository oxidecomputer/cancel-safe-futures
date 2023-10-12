//! Alternative synchronization primitives that are more cancellation-aware.
//!
//! Currently, this crate contains [`CMutex`], which is a variant on an async mutex that is more
//! cancel-safe. For more about how this differs from [`tokio::sync::Mutex`], see the documentation
//! for [`CMutex`].

#[cfg(feature = "std")]
mod mutex;
#[cfg(feature = "std")]
mod poison;

#[cfg(feature = "std")]
pub use mutex::*;
