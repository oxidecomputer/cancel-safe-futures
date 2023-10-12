//! Alternative synchronization primitives that are more cancellation-aware.
//!
//! Currently, this crate contains a [`Mutex`] implementation that is more cancellation-aware. For
//! more about how this differs from [`tokio::sync::Mutex`], see the documentation for [`Mutex`].

#[cfg(feature = "std")]
mod mutex;
#[cfg(feature = "std")]
mod poison;

#[cfg(feature = "std")]
pub use mutex::*;
