mod flush_reserve;
pub use flush_reserve::FlushReserve;

mod permit;
pub use permit::Permit;

mod reserve;
pub use reserve::Reserve;

use core::future::Future;

use futures_sink::Sink;

pub trait SinkExt<Item>: Sink<Item> {
    fn reserve(&mut self) -> Reserve<'_, Self, Item>
    where
        Self: Unpin,
    {
        assert_future::<Result<Permit<'_, Self, Item>, Self::Error>, _>(Reserve::new(self))
    }

    fn flush_reserve(&mut self) -> FlushReserve<'_, Self, Item>
    where
        Self: Unpin,
    {
        assert_future::<Result<Permit<'_, Self, Item>, Self::Error>, _>(FlushReserve::new(self))
    }
}

impl<T: ?Sized, Item> SinkExt<Item> for T where T: Sink<Item> {}

// Helper function to ensure that the futures we're returning all have the right implementations.
pub(crate) fn assert_future<T, F>(future: F) -> F
where
    F: Future<Output = T>,
{
    future
}
