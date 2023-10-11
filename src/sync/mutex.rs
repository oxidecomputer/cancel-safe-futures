use std::fmt;
use std::ops::{Deref, DerefMut};
use std::sync::{LockResult, TryLockError, TryLockResult};

use super::poison;
use futures_core::Future;
use tokio::sync::MutexGuard;

pub struct Mutex<T> {
    inner: tokio::sync::Mutex<T>,
    poison: poison::Flag,
}

impl<T> Mutex<T> {
    pub fn new(value: T) -> Self {
        Self {
            inner: tokio::sync::Mutex::new(value),
            poison: poison::Flag::new(),
        }
    }

    pub fn lock(&self) -> LockResult<impl Future<Output = ActionPermit<'_, T>>> {
        ActionPermit::new(self)
    }

    pub fn try_lock(&self) -> TryLockResult<ActionPermit<'_, T>> {
        match self.inner.try_lock() {
            Ok(guard) => {
                ActionPermit::from_guard(&self.poison, guard).map_err(TryLockError::Poisoned)
            }
            Err(_) => Err(TryLockError::WouldBlock),
        }
    }

    /// Determines whether the mutex is poisoned.
    ///
    /// If another thread is active, the mutex can still become poisoned at any
    /// time. You should not trust a `false` value for program correctness
    /// without additional synchronization.
    ///
    /// # Examples
    ///
    /// ```
    /// use cancel_safe_futures::sync::Mutex;
    /// use std::sync::Arc;
    /// use std::thread;
    ///
    /// # #[tokio::main]
    /// # async fn main() {
    ///
    /// let mutex = Arc::new(Mutex::new(0));
    /// let c_mutex = Arc::clone(&mutex);
    ///
    /// let _ = tokio::task::spawn(async move {
    ///     let _lock = c_mutex.lock().unwrap().await;
    ///     panic!(); // the mutex gets poisoned
    /// }).await;
    /// assert_eq!(mutex.is_poisoned(), true);
    /// # }
    /// ```
    pub fn is_poisoned(&self) -> bool {
        self.poison.get()
    }

    pub fn into_inner(self) -> LockResult<T>
    where
        T: Sized,
    {
        let data = self.inner.into_inner();
        poison::map_result(self.poison.borrow(), |()| data)
    }
}

impl<T: fmt::Debug> fmt::Debug for Mutex<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut d = f.debug_struct("Mutex");
        match self.try_lock() {
            Ok(inner) => d.field("data", &*inner.guard),
            Err(_) => d.field("data", &format_args!("<locked>")),
        };
        d.field("poisoned", &self.poison.get());
        d.finish()
    }
}

/// A token that grants the ability to run one closure against the data guarded
/// by a [`Mutex`].
///
/// This is produced by the `lock` family of operations on [`Mutex`] and is
/// intended to provide a more robust API than the traditional "smart pointer"
/// mutex guard.
pub struct ActionPermit<'a, T: ?Sized> {
    guard: MutexGuard<'a, T>,
    poison: &'a poison::Flag,
    poison_guard: poison::Guard,
}

impl<'a, T> ActionPermit<'a, T> {
    pub(crate) fn new(
        mutex: &'a Mutex<T>,
    ) -> LockResult<impl Future<Output = ActionPermit<'a, T>>> {
        poison::map_result(mutex.poison.guard(), |poison_guard| async {
            let guard = mutex.inner.lock().await;
            Self {
                guard,
                poison: &mutex.poison,
                poison_guard,
            }
        })
    }

    pub(crate) fn from_guard(
        poison: &'a poison::Flag,
        guard: MutexGuard<'a, T>,
    ) -> LockResult<Self> {
        poison::map_result(poison.guard(), |poison_guard| Self {
            guard,
            poison,
            poison_guard,
        })
    }

    /// Runs a closure with access to the guarded data, consuming the permit in
    /// the process.
    pub fn perform<R>(mut self, action: impl FnOnce(&mut T) -> R) -> R {
        action(&mut *self.guard)

        // Note: we're relying on the Drop impl for `self` to unlock the mutex.
    }
}

impl<T: ?Sized> Drop for ActionPermit<'_, T> {
    #[inline]
    fn drop(&mut self) {
        self.poison.done(&self.poison_guard);
    }
}

impl<T: ?Sized + fmt::Debug> fmt::Debug for ActionPermit<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&*self.guard, f)
    }
}

impl<T: ?Sized + fmt::Display> fmt::Display for ActionPermit<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&*self, f)
    }
}
