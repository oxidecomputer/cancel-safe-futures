use core::sync::atomic::{AtomicBool, Ordering};
use std::{
    sync::{LockResult, PoisonError},
    thread,
};

use futures_core::Future;

#[derive(Debug)]
pub struct Flag {
    failed: AtomicBool,
}

// Note that the Ordering uses to access the `failed` field of `Flag` below is
// always `Relaxed`, and that's because this isn't actually protecting any data,
// it's just a flag whether we've panicked or not.
//
// The actual location that this matters is when a mutex is **locked** which is
// where we have external synchronization ensuring that we see memory
// reads/writes to this flag.
//
// As a result, if it matters, we should see the correct value for `failed` in
// all cases.

impl Flag {
    #[inline]
    pub const fn new() -> Flag {
        Flag {
            failed: AtomicBool::new(false),
        }
    }

    /// Check the flag for an unguarded borrow, where we only care about existing poison.
    #[inline]
    pub fn borrow(&self) -> LockResult<()> {
        if self.get() {
            Err(PoisonError::new(()))
        } else {
            Ok(())
        }
    }

    /// Check the flag for a guarded borrow, where we may also set poison when `done`.
    #[inline]
    pub fn guard(&self) -> LockResult<Guard> {
        let ret = Guard {
            panicking: thread::panicking(),
        };
        if self.get() {
            Err(PoisonError::new(ret))
        } else {
            Ok(ret)
        }
    }

    #[inline]
    pub fn done(&self, guard: &Guard) {
        if !guard.panicking && thread::panicking() {
            self.failed.store(true, Ordering::Relaxed);
        }
    }

    #[inline]
    pub fn get(&self) -> bool {
        self.failed.load(Ordering::Relaxed)
    }
}

pub struct Guard {
    panicking: bool,
}

pub fn map_result<T, U, F>(result: LockResult<T>, f: F) -> LockResult<U>
where
    F: FnOnce(T) -> U,
{
    match result {
        Ok(t) => Ok(f(t)),
        Err(error) => Err(PoisonError::new(f(error.into_inner()))),
    }
}

pub async fn map_result_async<T, U, F, Fut>(result: LockResult<T>, f: F) -> LockResult<U>
where
    F: FnOnce(T) -> Fut,
    Fut: Future<Output = U>,
{
    match result {
        Ok(t) => Ok(f(t).await),
        Err(error) => Err(PoisonError::new(f(error.into_inner()).await)),
    }
}
