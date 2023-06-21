use super::Permit;
use core::{
    future::Future,
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll},
};
use futures_core::{ready, FusedFuture};
use futures_sink::Sink;

/// Future for the [`reserve`](super::SinkExt::reserve) method.
#[derive(Debug)]
#[must_use]
pub struct Reserve<'a, Si: ?Sized, Item> {
    // This is None once a permit has been issued.
    sink: Option<&'a mut Si>,
    // Not entirely sure why this is fn(Item), but presumably it's for variance reasons (an argument
    // to a fn parameter is contravariant with respect to lifetimes). This is copied from
    // futures_util::SinkExt's futures which all use PhantomData<fn(Item)>.
    _phantom: PhantomData<fn(Item)>,
}

// By default, Unpin would be implemented for Reserve even if Si isn't Unpin. But we explicitly only
// support Unpin sinks.
impl<Si: Unpin + ?Sized, Item> Unpin for Reserve<'_, Si, Item> {}

impl<'a, Item, Si: Sink<Item> + Unpin + ?Sized> Reserve<'a, Si, Item> {
    pub(super) fn new(sink: &'a mut Si) -> Self {
        Self {
            sink: Some(sink),
            _phantom: PhantomData,
        }
    }

    pub(super) fn sink_pin_mut(&mut self) -> Option<Pin<&mut Si>> {
        // Can't use Option::map here due to lifetime issues.
        #[allow(clippy::manual_map)]
        match &mut self.sink {
            Some(sink) => Some(Pin::new(sink)),
            None => None,
        }
    }
}

impl<'a, Si: Sink<Item> + Unpin + ?Sized, Item> Future for Reserve<'a, Si, Item> {
    type Output = Result<Permit<'a, Si, Item>, Si::Error>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if let Some(sink) = self.sink.as_mut() {
            ready!(Pin::new(sink).poll_ready(cx))?;
            let sink = self.sink.take().unwrap();
            Poll::Ready(Ok(Permit::new(sink)))
        } else {
            Poll::Pending
        }
    }
}

impl<Si: Sink<Item> + Unpin + ?Sized, Item> FusedFuture for Reserve<'_, Si, Item> {
    fn is_terminated(&self) -> bool {
        // Once a permit has been issued, the sink becomes None.
        self.sink.is_none()
    }
}
