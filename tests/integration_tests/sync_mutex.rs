#![cfg(feature = "std")]

use cancel_safe_futures::sync::{ActionPermit, RobustMutex, TryLockError};
use futures_util::FutureExt;
use std::{sync::Arc, time::Duration};
use tokio::time::{interval, timeout};
use tokio_test::{assert_pending, assert_ready, task::spawn};

#[test]
fn straight_execution() {
    let l = RobustMutex::new(100);

    {
        let mut t = spawn(l.lock());
        let permit = assert_ready!(t.poll()).unwrap();
        permit.perform(|g| {
            assert_eq!(&*g, &100);
            *g = 99;
        });
    }
    {
        let mut t = spawn(l.lock());
        let permit = assert_ready!(t.poll()).unwrap();
        permit.perform(|g| {
            assert_eq!(&*g, &99);
            *g = 98;
        });
    }
    {
        let mut t = spawn(l.lock());
        let permit = assert_ready!(t.poll()).unwrap();
        permit.perform(|g| {
            assert_eq!(&*g, &98);
        });
    }
}

#[test]
fn readiness() {
    let l1 = Arc::new(RobustMutex::new(100));
    let l2 = Arc::clone(&l1);
    let mut t1 = spawn(l1.lock());
    let mut t2 = spawn(l2.lock());

    let g = assert_ready!(t1.poll()).unwrap();

    // We can't now acquire the lease since it's already held in g
    assert_pending!(t2.poll());

    // But once g unlocks, we can acquire it
    drop(g);
    assert!(t2.is_woken());
    let _t2 = assert_ready!(t2.poll()).unwrap();
}

/// Ensure a mutex is unlocked if a future holding the lock is aborted prematurely.
///
/// This does not provide internal access to the data held by the mutex, so concerns about invariant
/// violations don't apply.
#[tokio::test]
async fn aborted_future_1() {
    let m1: Arc<RobustMutex<usize>> = Arc::new(RobustMutex::new(0));
    {
        let m2 = m1.clone();
        // Try to lock mutex in a future that is aborted prematurely
        timeout(Duration::from_millis(1u64), async move {
            let iv = interval(Duration::from_millis(1000));
            tokio::pin!(iv);
            let _g = m2.lock().await.unwrap();
            iv.as_mut().tick().await;
            iv.as_mut().tick().await;
        })
        .await
        .unwrap_err();
    }
    // This should succeed as there is no lock left for the mutex.
    timeout(Duration::from_millis(1u64), async move {
        let _g = m1.lock().await.unwrap();
    })
    .await
    .expect("Mutex is locked");
}

/// This test is similar to `aborted_future_1` but this time the aborted future is waiting for the
/// lock.
#[tokio::test]
async fn aborted_future_2() {
    let m1: Arc<RobustMutex<usize>> = Arc::new(RobustMutex::new(0));
    {
        // Lock mutex
        let _lock = m1.lock().await;
        {
            let m2 = m1.clone();
            // Try to lock mutex in a future that is aborted prematurely
            timeout(Duration::from_millis(1u64), async move {
                let _g = m2.lock().await.unwrap();
            })
            .await
            .unwrap_err();
        }
    }
    // This should succeed as there is no lock left for the mutex.
    timeout(Duration::from_millis(1u64), async move {
        let _g = m1.lock().await.unwrap();
    })
    .await
    .expect("Mutex is locked");
}

#[tokio::test]
async fn cancelled_perform_async() {
    // Non-cloneable data to test moves.
    #[derive(Debug)]
    struct Foo(u32);

    let m1: Arc<RobustMutex<Foo>> = Arc::new(RobustMutex::new(Foo(0)));
    {
        let permit = m1.lock().await.unwrap();
        cancel_perform_async(permit).await;
    }

    // The mutex should be poisoned due to a cancellation.
    let error = m1.lock().await.unwrap_err();
    assert!(error.is_cancel());
    assert!(!error.is_panic());
}

/// Ensure a mutex is poisoned if a task panics in the middle of perform.
#[tokio::test]
async fn panicking_task() {
    let m1: Arc<RobustMutex<usize>> = Arc::new(RobustMutex::new(0));
    {
        let m2 = m1.clone();
        tokio::task::spawn(async move {
            let permit = m2.lock().await.unwrap();
            permit.perform(|_| {
                panic!("oh no!");
            });
        })
        .await
        .unwrap_err();
    }
    // This returns a PoisonError.
    m1.lock().await.unwrap_err();

    // This returns a TryLockError of the Poisoned kind.
    let error = m1.try_lock().unwrap_err();
    assert!(matches!(error, TryLockError::Poisoned(_)));
}

#[test]
fn try_lock() {
    let m: RobustMutex<usize> = RobustMutex::new(0);
    {
        let g1 = m.try_lock();
        assert!(g1.is_ok());
        let g2 = m.try_lock();
        assert!(g2.is_err());
    }
    let g3 = m.try_lock();
    assert!(g3.is_ok());
}

#[tokio::test]
async fn mutex_guard_debug_display() {
    let s = "internal";
    let m = RobustMutex::new(s.to_string());
    let permit = m.lock().await.unwrap();
    assert_eq!(
        format!("ActionPermit {{ poison: (not poisoned), guard: {:?} }}", s),
        format!("{:?}", permit)
    );
}

#[tokio::test]
async fn mutex_debug_display() {
    let s = "data";
    let m = Arc::new(RobustMutex::new(s.to_string()));
    assert_eq!(
        format!("{:?}", m),
        r#"RobustMutex { data: "data", poisoned: (not poisoned) }"#
    );
    let _permit = m.lock().await.unwrap();
    assert_eq!(
        format!("{:?}", m),
        r#"RobustMutex { data: <locked>, poisoned: (not poisoned) }"#
    );
    std::mem::drop(_permit);
    assert_eq!(
        format!("{:?}", m),
        r#"RobustMutex { data: "data", poisoned: (not poisoned) }"#
    );

    // Panic in the middle of perform.
    let m2 = m.clone();
    tokio::task::spawn(async move {
        let permit = m2.lock().await.unwrap();
        permit.perform(|_| {
            panic!("oh no!");
        });
    })
    .await
    .unwrap_err();

    assert_eq!(
        format!("{:?}", m),
        r#"RobustMutex { data: <locked>, poisoned: (poisoned by panic) }"#
    );
}

#[tokio::test]
async fn mutex_debug_display_cancellation() {
    let s = "data";
    let m = Arc::new(RobustMutex::new(s.to_string()));

    // Cancel in the middle of perform_async.
    let m2 = m.clone();
    tokio::task::spawn(async move {
        let permit = m2.lock().await.unwrap();
        cancel_perform_async(permit).await;
    })
    .await
    .unwrap();

    assert_eq!(
        format!("{:?}", m),
        r#"RobustMutex { data: <locked>, poisoned: (poisoned by cancellation) }"#
    );
}

/// A basic function to test that a permit can be used to cancel a future.
async fn cancel_perform_async<T>(permit: ActionPermit<'_, T>) {
    let fut = permit.perform_async_boxed(|_| {
        async move {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        .boxed()
    });
    tokio::select! {
        _ = fut => {
            panic!("Future should have been cancelled");
        }
        _ = tokio::time::sleep(Duration::from_millis(1)) => {
            // This is expected.
        }
    }
}
