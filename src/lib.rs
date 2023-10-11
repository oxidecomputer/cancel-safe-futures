#![cfg_attr(not(feature = "std"), no_std)]

//! Alternative futures adapters that are more cancellation-aware.
//!
//! # What is this crate?
//!
//! This crate solves a few related but distinct problems:
//!
//! ## Cancel-safe futures adapters
//!
//! The [`futures`](https://docs.rs/futures/latest/futures/) library contains many adapters that
//! make writing asynchronous Rust code more pleasant. However, some of those combinators make it
//! hard to write code that can withstand cancellation in the case of timeouts, `select!` branches
//! or similar.
//!
//! For a more detailed explanation, see the documentation for [`SinkExt::reserve`].
//!
//! ### Example
//!
//! Attempt to send an item in a loop with a timeout:
//!
//! ```
//! use cancel_safe_futures::prelude::*;
//! use std::time::Duration;
//!
//! # #[tokio::main(flavor = "current_thread")]
//! # async fn main() -> Result<(), std::convert::Infallible> {
//! # /*
//! let mut my_sink = /* ... */;
//! # */
//! # let mut my_sink = futures_util::sink::drain();
//!
//! // This item is stored here and will be set to None once the loop exits successfully.
//! let mut item = Some("hello".to_owned());
//! let do_flush = false;
//!
//! while item.is_some() {
//!     match tokio::time::timeout(Duration::from_secs(10), my_sink.reserve()).await {
//!         Ok(Ok(permit)) => {
//!             let item = item.take().unwrap();
//!             if !do_flush {
//!                 // permit.feed() feeds the item into the sink without flushing
//!                 // the sink afterwards. This is a synchronous method.
//!                 permit.feed(item)?;
//!             } else {
//!                 // Alternatively, permit.send() writes the item into the sink
//!                 // synchronously, then returns a future which can be awaited to
//!                 // flush the sink.
//!                 permit.send(item)?.await?;
//!             }
//!         }
//!         Ok(Err(error)) => return Err(error),
//!         Err(timeout_error) => continue,
//!     }
//! }
//!
//! # Ok(()) }
//! ```
//!
//! ## `then_try` adapters that don't perform cancellations
//!
//! The futures and tokio libraries come with a number of `try_` adapters and macros, for example
//! [`tokio::try_join!`]. These adapters have the property that if one of the futures under
//! consideration fails, all other futures are cancelled.
//!
//! This is not always desirable and has led to correctness bugs (e.g. [omicron PR
//! 3707](https://github.com/oxidecomputer/omicron/pull/3707)). To address this issue, this crate
//! provides a set of `then_try` adapters and macros that behave like their `try_` counterparts,
//! except that if one or more of the futures errors out, the others will still be run to
//! completion.
//!
//! The `then_try` family includes:
//!
//! * [`join_then_try`]: similar to [`tokio::try_join`].
//! * [`future::join_all_then_try`]: similar to [`futures::future::try_join_all`].
//! * [`TryStreamExt`]: contains alternative extension methods to [`futures::stream::TryStreamExt`],
//!   such as `collect_then_try`.
//!
//! ### Example
//!
//! For a detailed example, see the documentation for the [`join_then_try`] macro.
//!
//! ## Cooperative cancellation
//!
//! Executors like Tokio support forcible cancellation for async tasks via facilities like
//! [`tokio::task::JoinHandle::abort`]. However, this can cause cancellations at any arbitrary await
//! point. If the future is in the middle of cancel-unsafe code, this can cause invariant violations
//! or other issues.
//!
//! Instead, async cancellation can be done cooperatively: code can check for cancellation
//! explicitly via [`tokio::select!`]. This crate provides the [`coop_cancel`] module that can be
//! used to accomplish that goal.
//!
//! ### Example
//!
//! For a detailed example, see the documentation for [`coop_cancel`].
//!
//! # Notes
//!
//! This library is not complete: adapters and macros are added on an as-needed basis. If you need
//! an adapter that is not yet implemented, please open an issue or a pull request.
//!
//! # Optional features
//!
//! * `macros` (enabled by default): Enables macros.
//! * `std` (enabled by default): Enables items that depend on `std`, including items that depend on
//!   `alloc`.
//! * `alloc` (enabled by default): Enables items that depend on `alloc`.
//!
//! No-std users must turn off default features while importing this crate.

#![warn(missing_docs)]
#![cfg_attr(doc_cfg, feature(doc_cfg, doc_auto_cfg))]

#[cfg(feature = "alloc")]
extern crate alloc;

// Includes re-exports used by macros.
//
// This module is not intended to be part of the public API. In general, any
// `doc(hidden)` code is not part of the public and stable API.
#[macro_use]
#[doc(hidden)]
pub mod macros;

#[cfg(feature = "std")]
pub mod coop_cancel;
pub mod future;
pub mod prelude;
pub mod sink;
pub mod stream;
mod support;
pub mod sync;

pub use sink::SinkExt;
pub use stream::TryStreamExt;
