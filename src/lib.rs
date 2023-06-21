#![no_std]

//! Alternative futures adapters that are more cancel-safe.
//!
//! # What is this crate?
//!
//! The [`futures`](https://docs.rs/futures/latest/futures/) library contains many adapters that
//! make writing asynchronous Rust code more pleasant. However, some of those combinators make it
//! hard to write code that can withstand cancellation in the case of timeouts, `select!` branches
//! or similar.
//!
//! For a more detailed explanation, see the documentation for [`SinkExt::reserve`].
//!
//! This crate contains alternative adapters that are designed for use in scenarios where
//! cancellation is expected.
//!
//! # Example
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

pub mod prelude;
pub mod sink;

pub use sink::SinkExt;
