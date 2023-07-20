use core::{mem, pin::Pin};
use futures_core::{
    future::{FusedFuture, Future},
    ready,
    stream::{FusedStream, TryStream},
    task::{Context, Poll},
};
use pin_project_lite::pin_project;

pin_project! {
    /// Future for the [`try_collect`](super::TryStreamExt::try_collect) method.
    #[derive(Debug)]
    #[must_use = "futures do nothing unless you `.await` or poll them"]
    pub struct CollectThenTry<St: TryStream, C> {
        #[pin]
        stream: St,
        items: C,
        first_error: Option<St::Error>,
    }
}

impl<St: TryStream, C: Default> CollectThenTry<St, C> {
    pub(super) fn new(s: St) -> Self {
        Self {
            stream: s,
            items: Default::default(),
            first_error: None,
        }
    }
}

impl<St: TryStream, C> FusedFuture for CollectThenTry<St, C>
where
    St: TryStream + FusedStream,
    C: Default + Extend<St::Ok>,
{
    fn is_terminated(&self) -> bool {
        self.stream.is_terminated()
    }
}

impl<St, C> Future for CollectThenTry<St, C>
where
    St: TryStream,
    C: Default + Extend<St::Ok>,
{
    type Output = Result<C, St::Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut this = self.project();
        Poll::Ready(loop {
            match ready!(this.stream.as_mut().try_poll_next(cx)) {
                Some(Ok(x)) => this.items.extend(Some(x)),
                Some(Err(e)) => {
                    if this.first_error.is_none() {
                        *this.first_error = Some(e);
                    }
                }
                None => {
                    if let Some(e) = this.first_error.take() {
                        break Err(e);
                    }
                    break Ok(mem::take(this.items));
                }
            }
        })
    }
}
