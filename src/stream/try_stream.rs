use super::CollectThenTry;
use crate::support::assert_future;
use futures_core::TryStream;

impl<S: ?Sized + TryStream> TryStreamExt for S {}

/// Alternative adapters for `Result`-returning streams
pub trait TryStreamExt: TryStream {
    /// Attempts to run this stream to completion, executing the provided asynchronous closure for
    /// each element on the stream concurrently as elements become available. Runs the stream to
    /// completion, then exits with:
    ///
    /// - `Ok(())` if all elements were processed successfully.
    /// - `Err(error)` if an error occurred while processing an element. The first error encountered
    ///   is cached and returned.
    ///
    /// This is similar to
    /// [`try_for_each_concurrent`](futures::stream::TryStreamExt::try_for_each_concurrent),
    /// but will continue running the stream to completion even if an error is encountered.
    ///
    /// This method is only available when the `std` or `alloc` feature of this library is
    /// activated, and it is activated by default.
    ///
    /// # Examples
    ///
    /// ```
    /// use cancel_safe_futures::stream::TryStreamExt;
    /// use tokio::sync::oneshot;
    /// use futures_util::{stream, FutureExt, StreamExt};
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() {
    /// let (tx1, rx1) = oneshot::channel();
    /// let (tx2, rx2) = oneshot::channel();
    /// let (tx3, rx3) = oneshot::channel();
    ///
    /// let stream = stream::iter(vec![rx1, rx2, rx3]);
    /// let fut = stream.map(Ok).for_each_concurrent_then_try(
    ///     /* limit */ 2,
    ///     |rx| async move {
    ///         let res: Result<(), oneshot::error::RecvError> = rx.await;
    ///         res
    ///     }
    /// );
    ///
    /// tx1.send(()).unwrap();
    /// // Drop the second sender so that `rx2` resolves to `Canceled`.
    /// drop(tx2);
    ///
    /// // Unlike `try_for_each_concurrent`, tx3 also needs to be resolved
    /// // before the future will finish execution. This causes `now_or_never` to
    /// // return None.
    /// let mut fut = std::pin::pin!(fut);
    /// assert_eq!(fut.as_mut().now_or_never(), None);
    ///
    /// tx3.send(()).unwrap();
    ///
    /// // The final result is an error because the second future
    /// // resulted in an error.
    /// fut.await.unwrap_err();
    /// # }
    /// ```
    #[cfg(not(futures_no_atomic_cas))]
    #[cfg(feature = "alloc")]
    fn for_each_concurrent_then_try<Fut, F>(
        self,
        limit: impl Into<Option<usize>>,
        f: F,
    ) -> super::ForEachConcurrentThenTry<Self, Fut, F>
    where
        F: FnMut(Self::Ok) -> Fut,
        Fut: core::future::Future<Output = Result<(), Self::Error>>,
        Self: Sized,
    {
        assert_future::<Result<(), Self::Error>, _>(super::ForEachConcurrentThenTry::new(
            self,
            limit.into(),
            f,
        ))
    }

    /// Attempt to transform a stream into a collection, returning a future representing the result
    /// of that computation.
    ///
    /// This adapter will collect all successful results of this stream and collect them into the
    /// specified collection type. Unlike
    /// [`try_collect`](futures::stream::TryStreamExt::try_collect), if an error happens then the
    /// stream will still be run to completion.
    ///
    /// If more than one error is produced, this adapter will return the first error encountered.
    ///
    /// The returned future will be resolved when the stream terminates.
    ///
    /// # Notes
    ///
    /// This adapter does not expose a way to gather and combine all returned errors. Implementing that
    /// is a future goal, but it requires some design work for a generic way to combine errors. To
    /// do that today, use [`futures::StreamExt::collect`] and combine errors at the end.
    ///
    /// # Examples
    ///
    /// This example uses the [`async-stream`](https://docs.rs/async-stream) crate to create a
    /// stream with interspersed `Ok` and `Err` values.
    ///
    /// ```
    /// use cancel_safe_futures::stream::TryStreamExt;
    /// use std::sync::atomic::{AtomicBool, Ordering};
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() {
    /// let end_of_stream = AtomicBool::new(false);
    /// let end_ref = &end_of_stream;
    ///
    /// // This stream generates interspersed Ok and Err values.
    /// let stream = async_stream::stream! {
    ///     for i in 1..=4 {
    ///         yield Ok(i);
    ///     }
    ///     yield Err(5);
    ///     for i in 6..=9 {
    ///         yield Ok(i);
    ///     }
    ///     yield Err(10);
    ///
    ///     end_ref.store(true, Ordering::SeqCst);
    /// };
    ///
    /// let output: Result<Vec<i32>, i32> = stream.collect_then_try().await;
    ///
    /// // The first error encountered is returned.
    /// assert_eq!(output, Err(5));
    ///
    /// // The stream is still run to completion even though it errored out in the middle.
    /// assert!(end_of_stream.load(Ordering::SeqCst));
    /// # }
    /// ```
    fn collect_then_try<C: Default + Extend<Self::Ok>>(self) -> CollectThenTry<Self, C>
    where
        Self: Sized,
    {
        assert_future::<Result<C, Self::Error>, _>(CollectThenTry::new(self))
    }
}
