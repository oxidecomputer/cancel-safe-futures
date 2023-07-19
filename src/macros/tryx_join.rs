/// Waits on multiple concurrent branches for **all** futures to complete, returning Ok(_) or an
/// error.
///
/// Unlike [`tokio::try_join`], this macro does not cancel remaining futures if one of them returns
/// an error. Instead, this macro runs all futures to completion.
///
/// If more than one future produces an error, `tryx_join!` returns the error from the first future
/// listed in the macro that produces an error.
///
/// The `tryx_join!` macro must be used inside of async functions, closures, and blocks.
///
/// # Why use `tryx_join`?
///
/// Consider what happens if you're wrapping a set of
/// [`AsyncWriteExt::write_all`](tokio::io::AsyncWriteExt::write_all) operations.
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
/// // With try_join, if one of the operations errors out the other one will be cancelled.
/// tokio::try_join!(
///     file1.write_all("data1".as_bytes()),
///     file2.write_all("data2".as_bytes()),
/// )?;
///
/// # Ok(()) }
/// ```
///
/// One way to run all futures to completion is to use the [`tokio::join`] macro.
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
/// // tokio_join is unaware of errors and runs all futures to completion.
/// let (res1, res2) = tokio::join!(
///     file1.write_all("data1".as_bytes()),
///     file2.write_all("data2".as_bytes()),
/// );
///
/// res1?;
/// res2?;
///
/// # Ok(()) }
/// ```
///
/// However, this is not ideal because it requires you to manually handle the results of each
/// future. The `tryx_join` macro is a user-friendly equivalent to the above `tokio::join` example.
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
/// // With tryx_join, if one of the operations errors out the other one will still be
/// // run to completion.
/// cancel_safe_futures::tryx_join!(
///     file1.write_all("data1".as_bytes()),
///     file2.write_all("data2".as_bytes()),
/// )?;
///
/// # Ok(()) }
/// ```
///
/// If an error occurs, the error from the first future listed in the macro that errors out will be
/// returned. This is identical to the [`tokio::join`] example above, where `res1` is checked before
/// `res2`.
///
/// # Notes
///
/// The supplied futures are stored inline and does not require allocating a `Vec`.
///
/// ### Runtime characteristics
///
/// By running all async expressions on the current task, the expressions are able to run
/// **concurrently** but not in **parallel**. This means all expressions are run on the same thread
/// and if one branch blocks the thread, all other expressions will be unable to continue. If
/// parallelism is required, spawn each async expression using [`tokio::task::spawn`] and pass the
/// join handle to `tryx_join!`.
#[macro_export]
#[cfg_attr(docsrs, doc(cfg(feature = "macros")))]
macro_rules! tryx_join {
    (@ {
        // One `_` for each branch in the `tryx_join!` macro. This is not used once
        // normalization is complete.
        ( $($count:tt)* )

        // The expression `0+1+1+ ... +1` equal to the number of branches.
        ( $($total:tt)* )

        // Normalized tryx_join! branches
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
      $crate::tryx_join!(@{ ($($s)* _) ($($n)* + 1) $($t)* ($($s)*) $e, } $($r)*)
    };

    // ===== Entry point =====

    ( $($e:expr),+ $(,)?) => {
        $crate::tryx_join!(@{ () (0) } $($e,)*)
    };

    () => { async { Ok(()) }.await }
}
