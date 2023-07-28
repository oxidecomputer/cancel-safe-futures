//! A multi-producer, single-consumer channel for cooperative (explicit) cancellation.
//!
//! This is similar in nature to a [`tokio::task::AbortHandle`], except it uses a cooperative model
//! for cancellation.
//!
//! # Motivation
//!
//! Executors like Tokio support forcible cancellation for async tasks via facilities like
//! [`tokio::task::JoinHandle::abort`]. However, this causes cancellations at any arbitrary await
//! point. This is often not desirable because it can lead to invariant violations.
//!
//! For example, consider this code that consists of both the cancel-safe
//! [`AsyncWriteExt::write_buf`](tokio::io::AsyncWriteExt::write_buf) and some cancel-unsafe code:
//!
//! ```
//! use bytes::Buf;
//! use std::io::Cursor;
//! use tokio::{io::AsyncWriteExt, sync::mpsc};
//!
//! struct DataWriter {
//!     writer: tokio::fs::File,
//!     bytes_written_channel: mpsc::Sender<usize>,
//! }
//!
//! impl DataWriter {
//!     async fn write(&mut self, cursor: &mut Cursor<&[u8]>) -> std::io::Result<()> {
//!         // Cursor<&[u8]> implements the bytes::Buf trait, which is used by `write_buf`.
//!         while cursor.has_remaining() {
//!             let bytes_written = self.writer.write_buf(cursor).await?; // (1)
//!             self.bytes_written_channel.send(bytes_written).await; // (2)
//!         }
//!
//!         Ok(())
//!     }
//! }
//! ```
//!
//! The invariant upheld by `DataWriter` is that if some bytes are written, the corresponding
//! `bytes_written` is sent over `self.bytes_written_channel`. This means that cancelling at await
//! point (1) is okay, but cancelling at await point (2) is not.
//!
//! If we use [`tokio::task::JoinHandle::abort`] to cancel the task, it is possible that the task is
//! cancelled at await point (2), breaking the invariant. Instead, we can use cooperative
//! cancellation with a `select!` loop.
//!
//! ```
//! use bytes::Buf;
//! use cancel_safe_futures::coop_cancel;
//! use std::io::Cursor;
//! use tokio::{io::AsyncWriteExt, sync::mpsc};
//!
//! struct DataWriter {
//!     writer: tokio::fs::File,
//!     bytes_written_channel: mpsc::Sender<usize>,
//!     cancel_receiver: coop_cancel::Receiver<()>,
//! }
//!
//! impl DataWriter {
//!     async fn write(&mut self, cursor: &mut Cursor<&[u8]>) -> std::io::Result<()> {
//!         while cursor.has_remaining() {
//!             tokio::select! {
//!                 res = self.writer.write_buf(cursor) => {
//!                     let bytes_written = res?;
//!                     self.bytes_written_channel.send(bytes_written).await;
//!                 }
//!                 Some(()) = self.cancel_receiver.recv() => {
//!                     // A cancellation notice was sent over the
//!                     // channel. Cancel here.
//!                     println!("cancelling!");
//!                     break;
//!                 }
//!             }
//!         }
//!
//!         Ok(())
//!     }
//! }
//! ```
//!
//! # Attaching a cancel message
//!
//! [`Canceler::cancel`] can be used to send a message of any type `T` along with the cancellation
//! event. This message is received via the `Some` variant of [`Receiver::recv`].
//!
//! For a given [`Receiver`], only the first message sent via any corresponding [`Canceler`] is
//! received. Subsequent calls to [`Receiver::recv`] will always return `None`, no matter whether
//! further cancellation messages are sent. (This can change in the future if there's a good use
//! case for it.)
//!
//! # Notes
//!
//! This module implements "fan-in" cancellation -- it supports many cancelers but only one
//! receiver. For "fan-out" cancellation with one sender and many receivers, consider using the
//! [`drain`](https://docs.rs/drain) crate. This module and `drain` can be combined: create a task
//! that listens to a [`Receiver`], and notify downstream receivers via `drain` in that task.

use core::{
    fmt,
    future::Future,
    marker::PhantomData,
    pin::Pin,
    task::{ready, Poll},
};
use futures_util::FutureExt;
use tokio::sync::{mpsc, oneshot};

use crate::support::statically_unreachable;

/// Creates and returns a cooperative cancellation pair.
///
/// For more information, see [the module documentation](`self`).
pub fn new_pair<T>() -> (Canceler<T>, Receiver<T>) {
    let (sender, receiver) = mpsc::unbounded_channel();
    (
        Canceler { sender },
        Receiver {
            receiver,
            first_sender: None,
        },
    )
}

/// A cooperative cancellation receiver.
///
/// For more information, see [the module documentation](`self`).
pub struct Receiver<T> {
    receiver: mpsc::UnboundedReceiver<CancelPayload<T>>,
    // This is cached and stored here until `Self` is dropped. The senders are really just a way to
    // signal that the cooperative cancel has completed.
    first_sender: Option<oneshot::Sender<Never>>,
}

impl<T> fmt::Debug for Receiver<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Receiver")
            .field("receiver", &self.receiver)
            .field("first_sender", &self.first_sender)
            .finish()
    }
}

impl<T> Receiver<T> {
    /// Receives a cancellation payload, or `None` if either:
    ///
    /// * a message was received in a previous attempt, or
    /// * all [`Canceler`] instances have been dropped.
    ///
    /// It is expected that after the first time `recv()` returns `Some`, the receiver will be
    /// dropped.
    pub async fn recv(&mut self) -> Option<T> {
        if self.first_sender.is_some() {
            None
        } else {
            match self.receiver.recv().await {
                Some(payload) => {
                    self.first_sender = Some(payload.dropped_sender);
                    Some(payload.message)
                }
                None => None,
            }
        }
    }
}

/// A cooperative cancellation sender.
///
/// For more information, see [the module documentation](`self`).
pub struct Canceler<T> {
    // This is an unbounded sender to make Self::cancel not async. In general we
    // don't expect too many messages to ever be sent via this channel.
    sender: mpsc::UnboundedSender<CancelPayload<T>>,
}

impl<T> Clone for Canceler<T> {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
        }
    }
}

impl<T> fmt::Debug for Canceler<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Canceler")
            .field("sender", &self.sender)
            .finish()
    }
}

impl<T> Canceler<T> {
    /// Performs a cancellation with a message.
    ///
    /// This sends the message immediately, and returns a [`Waiter`] that can be optionally waited
    /// against to block until the corresponding [`Receiver`] is dropped.
    ///
    /// Only the first message ever sent via any `Canceler` is received by the [`Receiver`].
    ///
    /// Returns `Err(message)` if the corresponding [`Receiver`] has already been dropped, which
    /// means that the cancel operation failed.
    pub fn cancel(&self, message: T) -> Result<Waiter<T>, T> {
        let (message, dropped_receiver) = CancelPayload::new(message);
        match self.sender.send(message) {
            Ok(()) => Ok(Waiter {
                dropped_receiver,
                _marker: PhantomData,
            }),
            Err(error) => Err(error.0.message),
        }
    }
}

#[derive(Debug)]
enum Never {}

/// A future which can be used to optionally block until a [`Receiver`] is dropped.
///
/// A [`Waiter`] is purely advisory, and optional to wait on. Dropping this future does
/// not affect cancellation.
pub struct Waiter<T> {
    // dropped_receiver is just a way to signal that the Receiver has been dropped.
    dropped_receiver: oneshot::Receiver<Never>,
    _marker: PhantomData<T>,
}

// oneshot::Receiver is Unpin, and PhantomData is irrelevant to the Unpin-ness of
// `Waiter`.
impl<T> Unpin for Waiter<T> {}

impl<T> fmt::Debug for Waiter<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Waiter")
            .field("dropped_receiver", &self.dropped_receiver)
            .finish()
    }
}

impl<T> Future for Waiter<T> {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut core::task::Context<'_>) -> Poll<Self::Output> {
        match ready!(self.as_mut().dropped_receiver.poll_unpin(cx)) {
            Ok(_) => {
                // Never is uninhabited.
                statically_unreachable()
            }
            Err(_) => {}
        }
        Poll::Ready(())
    }
}

struct CancelPayload<T> {
    message: T,
    dropped_sender: oneshot::Sender<Never>,
}

impl<T> fmt::Debug for CancelPayload<T>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CancelPayload")
            .field("message", &self.message)
            .field("dropped_sender", &self.dropped_sender)
            .finish()
    }
}

impl<T> CancelPayload<T> {
    fn new(message: T) -> (Self, oneshot::Receiver<Never>) {
        let (dropped_sender, dropped_receiver) = oneshot::channel();
        (
            Self {
                message,
                dropped_sender,
            },
            dropped_receiver,
        )
    }
}
