//! Alternative extensions for [`Sink`].

use futures_sink::Sink;

mod flush_reserve;
pub use flush_reserve::FlushReserve;

mod permit;
pub use permit::Permit;

mod reserve;
use crate::support::assert_future;
pub use reserve::Reserve;

/// Extension trait for [`Sink`] that provides alternative adapters.
pub trait SinkExt<Item>: Sink<Item> {
    /// A future that completes once an item is ready to be sent to this sink.
    ///
    /// The future stays pending until [`Sink::poll_ready`] completes, then returns a [`Permit`]
    /// which can be used to send an item to a sink.
    ///
    /// # Motivation
    ///
    /// Consider a select loop that calls the [`send`](futures_util::SinkExt::send) adapter,
    /// something common while writing async Rust code:
    ///
    /// ```
    /// use futures_util::SinkExt;
    /// use std::time::Duration;
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() -> Result<(), std::convert::Infallible> {
    /// # /*
    /// let mut my_sink = /* ... */;
    /// # */
    /// # let mut my_sink = futures_util::sink::drain();
    /// // item must either be an Option<T> or be cloned, otherwise a
    /// // "value moved here, in previous iteration of loop" error occurs.
    /// let mut item = "hello".to_owned();
    ///
    /// let mut interval = tokio::time::interval(Duration::from_secs(10));
    /// loop {
    ///     tokio::select! {
    ///         res = my_sink.send(item.clone()) => {
    ///             res?;
    ///             break;
    ///         }
    ///         _ = interval.tick() => {
    ///             continue;
    ///         }
    ///     }
    /// }
    ///
    /// # Ok(()) }
    /// ```
    ///
    /// If `interval.tick()` occurs before `my_sink.send(item.clone())` completes, then it is
    /// impossible to tell if the item was actually sent to the sink or not, since `send` combines
    /// [`Sink::poll_ready`], [`Sink::start_send`] and [`Sink::poll_flush`].
    ///
    /// `reserve` separates out [`Sink::poll_ready`] from the latter two steps, so that `item` is
    /// only sent after the stream is ready to accept it. In the above case, this might look
    /// something like:
    ///
    /// ```
    /// use cancel_safe_futures::SinkExt;
    /// use std::time::Duration;
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() -> Result<(), std::convert::Infallible> {
    /// # /*
    /// let mut my_sink = /* ... */;
    /// # */
    /// # let mut my_sink = futures_util::sink::drain();
    /// // item is an Option<T>, and will be set to None once it is sent.
    /// let mut item = Some("hello".to_owned());
    ///
    /// let mut interval = tokio::time::interval(Duration::from_secs(10));
    /// while item.is_some() {
    ///     tokio::select! {
    ///         res = my_sink.reserve() => {
    ///             let permit = res?;
    ///             permit.send(item.take().unwrap())?.await?;
    ///             break;
    ///         }
    ///         _ = interval.tick() => {
    ///             continue;
    ///         }
    ///     }
    /// }
    ///
    /// # Ok(()) }
    /// ```
    fn reserve(&mut self) -> Reserve<'_, Self, Item>
    where
        Self: Unpin,
    {
        assert_future::<Result<Permit<'_, Self, Item>, Self::Error>, _>(Reserve::new(self))
    }

    /// A future that completes once the sink is flushed, and an item is ready to be sent to it.
    ///
    /// This is similar to [`reserve`](SinkExt::reserve), except it calls
    /// [`poll_flush`](Sink::poll_flush) on the sink before calling [`poll_ready`](Sink::poll_ready)
    /// on it.
    fn flush_reserve(&mut self) -> FlushReserve<'_, Self, Item>
    where
        Self: Unpin,
    {
        assert_future::<Result<Permit<'_, Self, Item>, Self::Error>, _>(FlushReserve::new(self))
    }
}

impl<T: ?Sized, Item> SinkExt<Item> for T where T: Sink<Item> {}
