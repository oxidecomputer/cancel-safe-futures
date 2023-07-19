//! A "prelude" for crates using the `cancel-safe-futures` crate.
//!
//! This prelude is similar to the [standard library's prelude] in that you'll
//! almost always want to import its entire contents, but unlike the
//! standard library's prelude you'll have to do so manually:
//!
//! ```
//! # #[allow(unused_imports)]
//! use cancel_safe_futures::prelude::*;
//! ```
//!
//! The prelude may grow over time as additional items see ubiquitous use.
//!
//! [standard library's prelude]: https://doc.rust-lang.org/std/prelude/index.html

pub use crate::sink::SinkExt as _;
