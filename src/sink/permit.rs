use core::{marker::PhantomData, pin::Pin};

use futures_sink::Sink;
use futures_util::{sink::Flush, SinkExt};

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

    pub fn feed(self, item: Item) -> Result<(), Si::Error> {
        Pin::new(self.sink).start_send(item)
    }

    pub fn send(mut self, item: Item) -> Result<Flush<'a, Si, Item>, Si::Error> {
        Pin::new(&mut self.sink).start_send(item)?;
        Ok(self.sink.flush())
    }
}
