use core::fmt;
use core::num::NonZeroUsize;
use core::pin::Pin;
use futures_core::future::{FusedFuture, Future};
use futures_core::stream::TryStream;
use futures_core::task::{Context, Poll};
use futures_util::stream::{FuturesUnordered, StreamExt};
use pin_project_lite::pin_project;

pin_project! {
    /// Future for the
    /// [`for_each_concurrent_then_try`](super::TryStreamExt::for_each_concurrent_then_try)
    /// method.
    #[must_use = "futures do nothing unless you `.await` or poll them"]
    pub struct ForEachConcurrentThenTry<St: TryStream, Fut, F> {
        #[pin]
        stream: Option<St>,
        f: F,
        futures: FuturesUnordered<Fut>,
        limit: Option<NonZeroUsize>,
        first_error: Option<St::Error>,
    }
}

impl<St, Fut, F> fmt::Debug for ForEachConcurrentThenTry<St, Fut, F>
where
    St: TryStream + fmt::Debug,
    Fut: fmt::Debug,
    St::Error: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ForEachConcurrentThenTry")
            .field("stream", &self.stream)
            .field("futures", &self.futures)
            .field("limit", &self.limit)
            .field("first_error", &self.first_error)
            .finish()
    }
}

impl<St, Fut, F> FusedFuture for ForEachConcurrentThenTry<St, Fut, F>
where
    St: TryStream,
    F: FnMut(St::Ok) -> Fut,
    Fut: Future<Output = Result<(), St::Error>>,
{
    fn is_terminated(&self) -> bool {
        self.stream.is_none() && self.futures.is_empty()
    }
}

impl<St, Fut, F> ForEachConcurrentThenTry<St, Fut, F>
where
    St: TryStream,
    F: FnMut(St::Ok) -> Fut,
    Fut: Future<Output = Result<(), St::Error>>,
{
    pub(super) fn new(stream: St, limit: Option<usize>, f: F) -> Self {
        Self {
            stream: Some(stream),
            // Note: `limit` = 0 gets ignored.
            limit: limit.and_then(NonZeroUsize::new),
            f,
            futures: FuturesUnordered::new(),
            first_error: None,
        }
    }
}

impl<St, Fut, F> Future for ForEachConcurrentThenTry<St, Fut, F>
where
    St: TryStream,
    F: FnMut(St::Ok) -> Fut,
    Fut: Future<Output = Result<(), St::Error>>,
{
    type Output = Result<(), St::Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut this = self.project();
        loop {
            let mut made_progress_this_iter = false;

            // Check if we've already created a number of futures greater than `limit`
            if this
                .limit
                .map(|limit| limit.get() > this.futures.len())
                .unwrap_or(true)
            {
                let mut stream_completed = false;
                let elem = if let Some(stream) = this.stream.as_mut().as_pin_mut() {
                    match stream.try_poll_next(cx) {
                        Poll::Ready(Some(Ok(elem))) => {
                            made_progress_this_iter = true;
                            Some(elem)
                        }
                        Poll::Ready(Some(Err(error))) => {
                            if this.first_error.is_none() {
                                *this.first_error = Some(error);
                            }
                            None
                        }
                        Poll::Ready(None) => {
                            stream_completed = true;
                            None
                        }
                        Poll::Pending => None,
                    }
                } else {
                    None
                };
                if stream_completed {
                    this.stream.set(None);
                }
                if let Some(elem) = elem {
                    this.futures.push((this.f)(elem));
                }
            }

            match this.futures.poll_next_unpin(cx) {
                Poll::Ready(Some(item)) => {
                    made_progress_this_iter = true;
                    if let Err(error) = item {
                        if this.first_error.is_none() {
                            *this.first_error = Some(error);
                        }
                    }
                }
                Poll::Ready(None) => {
                    if this.stream.is_none() {
                        return Poll::Ready(this.first_error.take().map_or(Ok(()), Err));
                    }
                }
                Poll::Pending => {}
            }

            if !made_progress_this_iter {
                return Poll::Pending;
            }
        }
    }
}
