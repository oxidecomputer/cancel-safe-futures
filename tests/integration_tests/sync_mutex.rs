#![cfg(feature = "std")]

use cancel_safe_futures::sync::RobustMutex;
use std::{
    sync::{Arc, TryLockError},
    time::Duration,
};
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

/// Ensure a mutex is poisoned if a task panics in the middle of ActionPermit::map.
#[tokio::test]
async fn panicking_task_map() {
    let m1: Arc<RobustMutex<usize>> = Arc::new(RobustMutex::new(0));
    {
        let m2 = m1.clone();
        tokio::task::spawn(async move {
            let permit = m2.lock().await.unwrap();
            permit.map::<(), _>(|_| {
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

/// Ensure a mutex is poisoned if a task panics in the middle of ActionPermit::try_map.
#[tokio::test]
async fn panicking_task_try_map() {
    let m1: Arc<RobustMutex<usize>> = Arc::new(RobustMutex::new(0));
    {
        let m2 = m1.clone();
        tokio::task::spawn(async move {
            let permit = m2.lock().await.unwrap();
            _ = permit.try_map::<(), _>(|_| {
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

/// Ensure a mutex is poisoned if a task panics in the middle of MappedActionPermit::map.
#[tokio::test]
async fn panicking_task_mapped_map() {
    #[derive(Debug)]
    struct Foo(u32);

    let m1: Arc<RobustMutex<Foo>> = Arc::new(RobustMutex::new(Foo(0)));
    {
        let m2 = m1.clone();
        tokio::task::spawn(async move {
            let permit = m2.lock().await.unwrap().map(|x| &mut x.0);
            permit.map::<(), _>(|_| {
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

/// Ensure a mutex is poisoned if a task panics in the middle of MappedActionPermit::try_map.
#[tokio::test]
async fn panicking_task_mapped_try_map() {
    #[derive(Debug)]
    struct Foo(u32);

    let m1: Arc<RobustMutex<Foo>> = Arc::new(RobustMutex::new(Foo(0)));
    {
        let m2 = m1.clone();
        tokio::task::spawn(async move {
            let permit = m2.lock().await.unwrap().map(|x| &mut x.0);
            _ = permit.try_map::<(), _>(|_| {
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

/// Ensure a mutex is poisoned if a task panics in the middle of `MappedActionPermit::perform`.
#[tokio::test]
async fn panicking_task_mapped_action_permit_perform() {
    #[derive(Debug)]
    struct Foo(u32);

    let m1: Arc<RobustMutex<Foo>> = Arc::new(RobustMutex::new(Foo(0)));
    {
        let m2 = m1.clone();
        tokio::task::spawn(async move {
            let permit = m2.lock().await.unwrap().map(|x| &mut x.0);
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
    assert_eq!(format!("{:?}", s), format!("{:?}", permit));
    assert_eq!(s, format!("{}", permit));
}

#[tokio::test]
async fn mutex_debug_display() {
    let s = "data";
    let m = Arc::new(RobustMutex::new(s.to_string()));
    assert_eq!(
        format!("{:?}", m),
        r#"RobustMutex { data: "data", poisoned: false }"#
    );
    let _permit = m.lock().await.unwrap();
    assert_eq!(
        format!("{:?}", m),
        r#"RobustMutex { data: <locked>, poisoned: false }"#
    );
    std::mem::drop(_permit);
    assert_eq!(
        format!("{:?}", m),
        r#"RobustMutex { data: "data", poisoned: false }"#
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
        r#"RobustMutex { data: <locked>, poisoned: true }"#
    );
}
