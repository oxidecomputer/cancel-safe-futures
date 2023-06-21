use super::{Permit, Reserve};
use core::{
    future::Future,
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll},
};
use futures_core::{ready, FusedFuture};
use futures_sink::Sink;

/// Future for the [`flush_reserve`](super::SinkExt::flush_reserve) method.
#[derive(Debug)]
#[must_use]
pub struct FlushReserve<'a, Si: ?Sized, Item> {
    reserve: Reserve<'a, Si, Item>,
    state: FlushReserveState,
    _phantom: PhantomData<fn(Item)>,
}

// By default, Unpin would be implemented for FlushReserve even if Si isn't Unpin. But we explicitly
// only support Unpin sinks.
impl<Si: Unpin + ?Sized, Item> Unpin for FlushReserve<'_, Si, Item> {}

impl<'a, Item, Si: Sink<Item> + Unpin + ?Sized> FlushReserve<'a, Si, Item> {
    pub(super) fn new(sink: &'a mut Si) -> Self {
        Self {
            reserve: Reserve::new(sink),
            state: FlushReserveState::PollFlush,
            _phantom: PhantomData,
        }
    }
}

impl<'a, Si: Sink<Item> + Unpin + ?Sized, Item> Future for FlushReserve<'a, Si, Item> {
    type Output = Result<Permit<'a, Si, Item>, Si::Error>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = &mut *self;
        if this.state == FlushReserveState::PollFlush {
            ready!(this
                .reserve
                .sink_pin_mut()
                .expect("PollFlush => sink must be Some")
                .poll_flush(cx))?;
            // Move to the reserve state.
            this.state = FlushReserveState::Reserve;
        }

        debug_assert_eq!(this.state, FlushReserveState::Reserve);

        Pin::new(&mut this.reserve).poll(cx)
    }
}

impl<Si: Sink<Item> + Unpin + ?Sized, Item> FusedFuture for FlushReserve<'_, Si, Item> {
    fn is_terminated(&self) -> bool {
        // Once a permit has been issued, the sink becomes None.
        self.reserve.is_terminated()
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum FlushReserveState {
    PollFlush,
    Reserve,
}
