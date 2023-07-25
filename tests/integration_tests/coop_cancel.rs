#![cfg(feature = "std")]

use std::convert::Infallible;

use tokio::sync::oneshot;
#[cfg(not(tokio_wasm_not_wasi))]
use tokio::test as maybe_tokio_test;
use tokio_test::{assert_pending, assert_ready, task};
#[cfg(tokio_wasm_not_wasi)]
use wasm_bindgen_test::wasm_bindgen_test as maybe_tokio_test;

#[maybe_tokio_test]
async fn basic_cancel() {
    // Test basic cancellation.
    {
        let (canceler, mut receiver) = cancel_safe_futures::coop_cancel::new_pair();

        let mut handle = task::spawn(async move {
            let waiter = canceler.cancel("hello").expect("receiver kept open");
            waiter.await;
        });

        assert_pending!(handle.poll());

        let v = receiver.recv().await.expect("a message was sent");
        assert_eq!(v, "hello");
        assert_pending!(handle.poll(), "receiver hasn't been dropped yet");

        std::mem::drop(receiver);
        assert_ready!(handle.poll());
    }

    // Test the receiver being dropped.
    {
        let (canceler, _) = cancel_safe_futures::coop_cancel::new_pair();

        let mut handle = task::spawn(async move {
            canceler.cancel("hello").expect_err("receiver dropped");
        });

        assert_ready!(handle.poll());
    }

    // Test the handle being dropped.
    {
        let (_, mut receiver) = cancel_safe_futures::coop_cancel::new_pair::<()>();
        assert!(receiver.recv().await.is_none(), "canceler dropped");
    }
}

#[maybe_tokio_test]
async fn multiple_messages() {
    // Test multiple messages.
    {
        let (canceler1, mut receiver) = cancel_safe_futures::coop_cancel::new_pair();
        let canceler2 = canceler1.clone();

        // Used to signal that handle1 is done sending a cancellation message. (Infallble) because
        // we only ever want to signal one value.
        let (ordering_sender, ordering_receiver) = oneshot::channel::<Infallible>();

        let mut handle1 = task::spawn(async move {
            let waiter = canceler1
                .cancel("handle1 cancelled")
                .expect("receiver kept open");
            std::mem::drop(ordering_sender);

            waiter.await;
        });

        assert_pending!(handle1.poll());

        let mut handle2 = task::spawn(async move {
            // expect_err because the only possible value that can be constructed is Err(). (This
            // could potentially be done via an irrefutable pattern if that were supported in stable
            // Rust.)
            ordering_receiver.await.expect_err("handle1 dropped");
            let waiter = canceler2
                .cancel("handle2 cancelled")
                .expect("receiver kept open");
            waiter.await;
        });

        assert_pending!(handle2.poll());

        let v = receiver.recv().await.expect("cancellation occurred");
        assert_eq!(v, "handle1 cancelled");

        assert_pending!(handle1.poll(), "handle1: receiver hasn't been dropped yet");
        assert_pending!(handle2.poll(), "handle2: receiver hasn't been dropped yet");

        std::mem::drop(receiver);

        assert_ready!(handle1.poll());
        assert_ready!(handle2.poll());
    }
}
