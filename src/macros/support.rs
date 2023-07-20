//! Support for futures.

pub use core::{
    future::{poll_fn, Future},
    pin::Pin,
    task::{Context, Poll},
};
pub use futures_util::future::{maybe_done, MaybeDone};
