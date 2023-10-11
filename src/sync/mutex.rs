use std::fmt;
use std::sync::{LockResult, TryLockError, TryLockResult};

use super::poison;
use tokio::sync::MutexGuard;

pub struct Mutex<T: ?Sized> {
    poison: poison::Flag,
    inner: tokio::sync::Mutex<T>,
}

impl<T: ?Sized> Mutex<T> {
    /// Creates a new lock in an unlocked state ready for use.
    ///
    /// # Examples
    ///
    /// ```
    /// use cancel_safe_futures::sync::Mutex;
    ///
    /// let lock = Mutex::new(5);
    /// ```
    #[track_caller]
    pub fn new(value: T) -> Self
    where
        T: Sized,
    {
        Self {
            inner: tokio::sync::Mutex::new(value),
            poison: poison::Flag::new(),
        }
    }

    /// Creates a new lock in an unlocked state ready for use.
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio::sync::Mutex;
    ///
    /// static LOCK: Mutex<i32> = Mutex::const_new(5);
    /// ```
    #[cfg(all(feature = "parking_lot", not(all(loom, test)),))]
    #[cfg_attr(doc_cfg, doc(cfg(feature = "parking_lot")))]
    pub const fn const_new(value: T) -> Self
    where
        T: Sized,
    {
        Self {
            inner: tokio::sync::Mutex::const_new(value),
            poison: poison::Flag::new(),
        }
    }

    /// Locks this mutex, causing the current task to yield until the lock has
    /// been acquired.  When the lock has been acquired, function returns a
    /// [`ActionPermit`].
    ///
    /// # Cancel safety
    ///
    /// This method uses a queue to fairly distribute locks in the order they
    /// were requested. Cancelling a call to `lock` makes you lose your place in
    /// the queue.
    ///
    /// # Examples
    ///
    /// ```
    /// use cancel_safe_futures::sync::Mutex;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let mutex = Mutex::new(1);
    ///
    ///     let mut permit = mutex.lock().await.unwrap();
    ///     permit.perform(|n| *n = 2);
    /// }
    /// ```
    pub async fn lock(&self) -> LockResult<ActionPermit<'_, T>> {
        let guard = self.inner.lock().await;
        ActionPermit::new(guard, &self.poison)
    }

    /// Blockingly locks this `Mutex`. When the lock has been acquired, function returns a
    /// [`ActionPermit`].
    ///
    /// This method is intended for use cases where you need to use this mutex in asynchronous code
    /// as well as in synchronous code.
    ///
    /// # Panics
    ///
    /// This function panics if called within an asynchronous execution context.
    ///
    ///   - If you find yourself in an asynchronous execution context and needing to call some
    ///     (synchronous) function which performs one of these `blocking_` operations, then consider
    ///     wrapping that call inside [`spawn_blocking()`][crate::runtime::Handle::spawn_blocking]
    ///     (or [`block_in_place()`][crate::task::block_in_place]).
    ///
    /// # Examples
    ///
    /// ```
    /// use cancel_safe_futures::sync::Mutex;
    /// use std::sync::Arc;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let mutex =  Arc::new(Mutex::new(1));
    ///     let permit = mutex.lock().await.unwrap();
    ///
    ///     let mutex1 = Arc::clone(&mutex);
    ///     let blocking_task = tokio::task::spawn_blocking(move || {
    ///         // This shall block until the `lock` is released.
    ///         let permit = mutex1.blocking_lock().unwrap();
    ///         permit.perform(|n| *n = 2);
    ///     });
    ///
    ///     permit.perform(|n| { assert_eq!(*n, 1) });
    ///
    ///     // Await the completion of the blocking task.
    ///     blocking_task.await.unwrap();
    ///
    ///     // Assert uncontended.
    ///     let permit = mutex.try_lock().unwrap();
    ///     permit.perform(|n| { assert_eq!(*n, 2) });
    /// }
    /// ```
    #[track_caller]
    #[cfg_attr(doc_cfg, doc(alias = "lock_blocking"))]
    pub fn blocking_lock(&self) -> LockResult<ActionPermit<'_, T>> {
        let guard = self.inner.blocking_lock();
        ActionPermit::new(guard, &self.poison)
    }

    /// Attempts to acquire the lock, and returns [`TryLockError`] if the
    /// lock is currently held somewhere else.
    ///
    /// [`TryLockError`]: TryLockError
    /// # Examples
    ///
    /// ```
    /// use cancel_safe_futures::sync::Mutex;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let mutex = Mutex::new(1);
    ///
    ///     let permit = mutex.try_lock().unwrap();
    ///     permit.perform(|n| {
    ///         assert_eq!(*n, 1);
    ///     });
    /// }
    /// ```
    pub fn try_lock(&self) -> TryLockResult<ActionPermit<'_, T>> {
        match self.inner.try_lock() {
            Ok(guard) => ActionPermit::new(guard, &self.poison).map_err(TryLockError::Poisoned),
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
    ///     let _lock = c_mutex.lock().await.unwrap();
    ///     panic!(); // the mutex gets poisoned
    /// }).await;
    /// assert_eq!(mutex.is_poisoned(), true);
    /// # }
    /// ```
    pub fn is_poisoned(&self) -> bool {
        self.poison.get()
    }

    /// Returns a mutable reference to the underlying data.
    ///
    /// Since this call borrows the `Mutex` mutably, no actual locking needs to
    /// take place -- the mutable borrow statically guarantees no locks exist.
    ///
    /// # Examples
    ///
    /// ```
    /// use cancel_safe_futures::sync::Mutex;
    ///
    /// fn main() {
    ///     let mut mutex = Mutex::new(1);
    ///
    ///     let n = mutex.get_mut();
    ///     *n = 2;
    /// }
    /// ```
    pub fn get_mut(&mut self) -> &mut T {
        self.inner.get_mut()
    }

    /// Consumes the mutex, returning the underlying data.
    ///
    /// # Examples
    ///
    /// ```
    /// use cancel_safe_futures::sync::Mutex;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let mutex = Mutex::new(1);
    ///
    ///     let n = mutex.into_inner().unwrap();
    ///     assert_eq!(n, 1);
    /// }
    /// ```
    pub fn into_inner(self) -> LockResult<T>
    where
        T: Sized,
    {
        let data = self.inner.into_inner();
        poison::map_result(self.poison.borrow(), |()| data)
    }
}

impl<T: ?Sized + fmt::Debug> fmt::Debug for Mutex<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut d = f.debug_struct("Mutex");
        match self.try_lock() {
            Ok(inner) => d.field("data", &inner.guard),
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
    poison: &'a poison::Flag,
    poison_guard: poison::Guard,
    guard: MutexGuard<'a, T>,
}

impl<'a, T: ?Sized> ActionPermit<'a, T> {
    /// Invariant: the mutex must be locked when this is called.
    pub(crate) fn new(guard: MutexGuard<'a, T>, poison: &'a poison::Flag) -> LockResult<Self> {
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
        fmt::Display::fmt(&*self.guard, f)
    }
}
