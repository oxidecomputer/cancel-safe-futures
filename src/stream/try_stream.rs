use super::TryxCollect;
use crate::support::assert_future;
use futures_core::TryStream;

impl<S: ?Sized + TryStream> TryStreamExt for S {}

/// Alternative adapters for `Result`-returning streams
pub trait TryStreamExt: TryStream {
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
    /// let output: Result<Vec<i32>, i32> = stream.tryx_collect().await;
    ///
    /// // The first error encountered is returned.
    /// assert_eq!(output, Err(5));
    ///
    /// // The stream is still run to completion even though it errored out in the middle.
    /// assert!(end_of_stream.load(Ordering::SeqCst));
    /// # }
    /// ```
    fn tryx_collect<C: Default + Extend<Self::Ok>>(self) -> TryxCollect<Self, C>
    where
        Self: Sized,
    {
        assert_future::<Result<C, Self::Error>, _>(TryxCollect::new(self))
    }
}
