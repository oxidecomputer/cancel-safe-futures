use core::{marker::PhantomData, pin::Pin};

use futures_sink::Sink;
use futures_util::{sink::Flush, SinkExt};

/// A permit to send an item to a sink.
///
/// Permits are issued by the [`reserve`] and [`flush_reserve`] adapters, and indicate that
/// [`Sink::poll_ready`] has completed and that the sink is now ready to accept an item.
///
/// [`reserve`]: crate::sink::SinkExt::reserve
/// [`flush_reserve`]: crate::sink::SinkExt::flush_reserve
#[derive(Debug)]
#[must_use]
pub struct Permit<'a, Si: ?Sized, Item> {
    sink: &'a mut Si,
    _phantom: PhantomData<fn(Item)>,
}

// By default, Unpin would be implemented for Permit even if Si isn't Unpin. But we explicitly only
// support Unpin sinks.
impl<Si: Unpin + ?Sized, Item> Unpin for Permit<'_, Si, Item> {}

impl<'a, Item, Si: Sink<Item> + Unpin + ?Sized> Permit<'a, Si, Item> {
    pub(super) fn new(sink: &'a mut Si) -> Self {
        Self {
            sink,
            _phantom: PhantomData,
        }
    }

    /// Sends an item to the sink, akin to the [`SinkExt::feed`] adapter.
    ///
    /// Unlike [`SinkExt::feed`], `Permit::feed` is a synchronous method. This is because a `Permit`
    /// indicates that [`Sink::poll_ready`] has been called already, so the sink is immediately
    /// ready to accept an item.
    pub fn feed(self, item: Item) -> Result<(), Si::Error> {
        Pin::new(self.sink).start_send(item)
    }

    /// Sends an item to the sink and then flushes it, akin to the [`SinkExt::send`] adapter.
    ///
    /// Unlike [`SinkExt::send`], `Permit::send` has two parts:
    ///
    /// 1. A synchronous part, which sends the item to the sink. This part is identical to
    ///    [`Self::feed`].
    /// 2. An asynchronous part, which flushes the sink via the [`SinkExt::flush`] adapter.
    ///
    /// This structure means that users get immediate feedback about, and can then await the
    /// resulting [`Flush`] future, or cancel it if necessary.
    ///
    /// # Cancel safety
    ///
    /// The returned [`Flush`] future is cancel-safe. If it is dropped, the sink will no longer be
    /// flushed. It is recommended that `flush()` be called explicitly, either by itself or via
    /// the [`flush_reserve`](crate::SinkExt::flush_reserve) adapter.
    pub fn send(mut self, item: Item) -> Result<Flush<'a, Si, Item>, Si::Error> {
        Pin::new(&mut self.sink).start_send(item)?;
        Ok(self.sink.flush())
    }
}
