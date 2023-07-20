#![cfg(feature = "alloc")]

use cancel_safe_futures::future::join_all_then_try;
use futures_util::future::{err, ok};
use std::future::Future;
use tokio::sync::oneshot;
#[cfg(not(tokio_wasm_not_wasi))]
use tokio::test as maybe_tokio_test;
use tokio_test::{assert_pending, assert_ready, task};
#[cfg(tokio_wasm_not_wasi)]
use wasm_bindgen_test::wasm_bindgen_test as maybe_tokio_test;

#[maybe_tokio_test]
async fn basic() {
    assert_eq!(
        join_all_then_try(vec![ok(1), ok(2)]).await,
        Ok::<_, usize>(vec![1, 2]),
    );
    assert_eq!(join_all_then_try(vec![ok(1), err(2)]).await, Err(2));
}

#[maybe_tokio_test]
async fn iter_lifetime() {
    // In futures-rs version 0.1, this function would fail to typecheck due to an overly
    // conservative type parameterization of `TryJoinAll`.
    fn sizes(bufs: Vec<&[u8]>) -> impl Future<Output = Result<Vec<usize>, ()>> {
        let iter = bufs.into_iter().map(|b| ok::<usize, ()>(b.len()));
        join_all_then_try(iter)
    }

    assert_eq!(
        sizes(vec![&[1, 2, 3], &[], &[0]]).await,
        Ok(vec![3_usize, 0, 1])
    );
}

#[maybe_tokio_test]
async fn err_no_abort_early() {
    // Run this test for several sizes of vectors, since the implementation is different depending
    // on the length of the input vector (the implementation changes at 30 elements).
    for len in [4, 8, 16, 32, 64] {
        err_no_abort_early_helper(len).await;
    }
}

async fn err_no_abort_early_helper(len: usize) {
    let mut txs = Vec::with_capacity(len);
    let mut futures = Vec::with_capacity(len);
    for ix in 0..len {
        let (tx, rx) = oneshot::channel();
        txs.push(tx);
        futures.push(async move {
            rx.await
                .map_err(|_| format!("(len = {len}) rx[{ix}] failed"))
        });
    }

    let mut join = task::spawn(join_all_then_try(futures));

    assert_pending!(join.poll());

    // Send values in reverse order, ensuring that the first future to fail is not the first future
    // returned by join_all_then_try.
    for ix in (0..len).rev() {
        let tx = txs.pop().unwrap();

        if ix % 3 == 0 {
            // Drop tx, resulting in an error.
            std::mem::drop(tx);
        } else {
            // Send a value (it's arbitrary).
            tx.send(ix).unwrap();
        }

        assert!(join.is_woken());

        if ix != 0 {
            // The join should still be pending since not all rx futures have completed. This is a
            // difference from tokio::try_join.
            assert_pending!(
                join.poll(),
                "(len = {len}) join still pending after {ix} is processed"
            );
        }
    }

    // All futures have completed.
    let res = assert_ready!(
        join.poll(),
        "(len = {len}) join ready after all futures processed"
    );
    // Even though rx[0] was not the first future to fail chronologically, it was the first future
    // to fail numerically.
    assert_eq!(res.unwrap_err(), format!("(len = {len}) rx[0] failed"));
}
