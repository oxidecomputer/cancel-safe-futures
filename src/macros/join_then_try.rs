/// Waits on multiple concurrent branches for **all** futures to complete, returning Ok(_) or an
/// error.
///
/// Unlike [`tokio::try_join`], this macro does not cancel remaining futures if one of them returns
/// an error. Instead, this macro runs all futures to completion.
///
/// If more than one future produces an error, `join_then_try!` returns the error from the first
/// future listed in the macro that produces an error.
///
/// The `join_then_try!` macro must be used inside of async functions, closures, and blocks.
///
/// # Why use `join_then_try`?
///
/// Consider what happens if you're wrapping a set of
/// [`AsyncWriteExt::flush`](tokio::io::AsyncWriteExt::flush) operations.
///
/// ```
/// use tokio::io::AsyncWriteExt;
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> anyhow::Result<()> {
/// let temp_dir = tempfile::tempdir()?;
/// let mut file1 = tokio::fs::File::create(temp_dir.path().join("file1")).await?;
/// let mut file2 = tokio::fs::File::create(temp_dir.path().join("file2")).await?;
///
/// // ... write some data to file1 and file2
///
/// tokio::try_join!(file1.flush(), file2.flush())?;
///
/// # Ok(()) }
/// ```
///
/// If `file1.flush()` returns an error, `file2.flush()` will be cancelled. This is not ideal, since
/// we'd like to make an effort to flush both files as far as possible.
///
/// One way to run all futures to completion is to use the [`tokio::join`] macro.
///
/// ```
/// # use tokio::io::AsyncWriteExt;
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> anyhow::Result<()> {
/// # let temp_dir = tempfile::tempdir()?;
/// let mut file1 = tokio::fs::File::create(temp_dir.path().join("file1")).await?;
/// let mut file2 = tokio::fs::File::create(temp_dir.path().join("file2")).await?;
///
/// // tokio::join! is unaware of errors and runs all futures to completion.
/// let (res1, res2) = tokio::join!(file1.flush(), file2.flush());
/// res1?;
/// res2?;
/// # Ok(()) }
/// ```
///
/// This, too, is not ideal because it requires you to manually handle the results of each future.
///
/// The `join_then_try` macro behaves identically to the above `tokio::join` example, except it is
/// more user-friendly.
///
/// ```
/// # use tokio::io::AsyncWriteExt;
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> anyhow::Result<()> {
/// # let temp_dir = tempfile::tempdir()?;
/// let mut file1 = tokio::fs::File::create(temp_dir.path().join("file1")).await?;
/// let mut file2 = tokio::fs::File::create(temp_dir.path().join("file2")).await?;
///
/// // With join_then_try, if one of the operations errors out the other one will still be
/// // run to completion.
/// cancel_safe_futures::join_then_try!(file1.flush(), file2.flush())?;
/// # Ok(()) }
/// ```
///
/// If an error occurs, the error from the first future listed in the macro that errors out will be
/// returned.
///
/// # Notes
///
/// The supplied futures are stored inline. This macro is no-std and no-alloc compatible and does
/// not require allocating a `Vec`.
///
/// This adapter does not expose a way to gather and combine all returned errors. Implementing that
/// is a future goal, but it requires some design work for a generic way to combine errors. To
/// do that today, use [`tokio::join`] and combine errors at the end.
///
/// # Runtime characteristics
///
/// By running all async expressions on the current task, the expressions are able to run
/// **concurrently** but not in **parallel**. This means all expressions are run on the same thread
/// and if one branch blocks the thread, all other expressions will be unable to continue. If
/// parallelism is required, spawn each async expression using [`tokio::task::spawn`] and pass the
/// join handle to `join_then_try!`.
#[macro_export]
#[cfg_attr(docsrs, doc(cfg(feature = "macros")))]
macro_rules! join_then_try {
    (@ {
        // One `_` for each branch in the `join_then_try!` macro. This is not used once
        // normalization is complete.
        ( $($count:tt)* )

        // The expression `0+1+1+ ... +1` equal to the number of branches.
        ( $($total:tt)* )

        // Normalized join_then_try! branches
        $( ( $($skip:tt)* ) $e:expr, )*

    }) => {{
        use $crate::macros::support::{maybe_done, poll_fn, Future, Pin};
        use $crate::macros::support::Poll::{Ready, Pending};

        // Safety: nothing must be moved out of `futures`. This is to satisfy
        // the requirement of `Pin::new_unchecked` called below.
        //
        // We can't use the `pin!` macro for this because `futures` is a tuple
        // and the standard library provides no way to pin-project to the fields
        // of a tuple.
        let mut futures = ( $( maybe_done($e), )* );

        // This assignment makes sure that the `poll_fn` closure only has a
        // reference to the futures, instead of taking ownership of them. This
        // mitigates the issue described in
        // <https://internals.rust-lang.org/t/surprising-soundness-trouble-around-pollfn/17484>
        let mut futures = &mut futures;

        // Each time the future created by poll_fn is polled, a different future will be polled first
        // to ensure every future passed to join! gets a chance to make progress even if
        // one of the futures consumes the whole budget.
        //
        // This is number of futures that will be skipped in the first loop
        // iteration the next time.
        let mut skip_next_time: u32 = 0;

        poll_fn(move |cx| {
            const COUNT: u32 = $($total)*;

            let mut is_pending = false;

            let mut to_run = COUNT;

            // The number of futures that will be skipped in the first loop iteration
            let mut skip = skip_next_time;

            skip_next_time = if skip + 1 == COUNT { 0 } else { skip + 1 };

            // This loop runs twice and the first `skip` futures
            // are not polled in the first iteration.
            loop {
            $(
                if skip == 0 {
                    if to_run == 0 {
                        // Every future has been polled
                        break;
                    }
                    to_run -= 1;

                    // Extract the future for this branch from the tuple.
                    let ( $($skip,)* fut, .. ) = &mut *futures;

                    // Safety: future is stored on the stack above
                    // and never moved.
                    let mut fut = unsafe { Pin::new_unchecked(fut) };

                    // Try polling
                    if fut.as_mut().poll(cx).is_pending() {
                        is_pending = true;
                    }
                } else {
                    // Future skipped, one less future to skip in the next iteration
                    skip -= 1;
                }
            )*
            }

            if is_pending {
                Pending
            } else {
                Ready(Ok(($({
                    // Extract the future for this branch from the tuple.
                    let ( $($skip,)* fut, .. ) = &mut futures;

                    // Safety: future is stored on the stack above
                    // and never moved.
                    let mut fut = unsafe { Pin::new_unchecked(fut) };

                    let output = fut.take_output().expect("expected completed future");
                    match output {
                        Ok(output) => output,
                        Err(error) => return Ready(Err(error)),
                    }
                },)*)))
            }
        }).await
    }};

    // ===== Normalize =====

    (@ { ( $($s:tt)* ) ( $($n:tt)* ) $($t:tt)* } $e:expr, $($r:tt)* ) => {
      $crate::join_then_try!(@{ ($($s)* _) ($($n)* + 1) $($t)* ($($s)*) $e, } $($r)*)
    };

    // ===== Entry point =====

    ( $($e:expr),+ $(,)?) => {
        $crate::join_then_try!(@{ () (0) } $($e,)*)
    };

    () => { async { Ok(()) }.await }
}
