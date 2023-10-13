use super::poison;
use std::{
    fmt,
    sync::{LockResult, TryLockError, TryLockResult},
};
use tokio::sync::{MappedMutexGuard, MutexGuard};

/// A cancel-safe and panic-safe variant of [`tokio::sync::Mutex`].
///
/// A `RobustMutex` is a wrapper on top of a [`tokio::sync::Mutex`] which adds two further
/// guarantees: *panic safety* and *cancel safety*. Both of these guarantees are implemented to
/// ensure that mutex invariants aren't violated to the greatest extent possible.
///
/// # Motivation
///
/// A mutex is a synchronization structure which allows only one task to access some data at a time.
/// The general idea behind a mutex is that the data it owns has some *invariants*. When a task
/// acquires a lock on the mutex, it enters a *critical section*. Within this critical section, the
/// invariants can temporarily be violated. It is expected that the task will restore those
/// invariants before releasing the lock.
///
/// For example, let's say that we have a mutex which guards two `HashMap`s, with the invariant that
/// mutex are that the two `HashMap`s always contain the same keys. With a Tokio mutex, you might
/// write something like:
///
/// ```rust
/// use std::collections::HashMap;
/// use tokio::sync::Mutex;
///
/// struct MyStruct {
///     map1: HashMap<String, String>,
///     map2: HashMap<String, u32>,
/// }
///
/// impl MyStruct {
///     fn new() -> Self {
///         Self {
///             map1: HashMap::new(),
///             map2: HashMap::new(),
///         }
///     }
/// }
///
/// #[tokio::main]
/// async fn main() {
///     let mutex = Mutex::new(MyStruct::new());
///
///     let mut guard = mutex.lock().await;
///     guard.map1.insert("hello".to_owned(), "world".to_owned());  // (1)
///     // ... some code in between
///     guard.map2.insert("hello".to_owned(), 42);  // (2)
///
///     // (This happens implicitly but is made explicit here.)
///     std::mem::drop(guard);
/// }
/// ```
///
/// At point (1) we've temporarily violated the invariant that `map1` and `map2` contain the same
/// keys. However, at point (2) the invariant is restored.
///
/// * But what if the task panics between (1) and (2)? In that case, the mutex is left in a state
///   where the invariants are violated. This is a problem because this is an inconsistent state --
///   other tasks which acquire the lock can no longer assume that the invariants are upheld.
///
///   This is the problem that *poisoning* solves.
///
/// * In async code, what if there's an await point between (1) and (2), and the future is dropped
///   at that await point? Then, too, the invariants are violated. With synchronous code the only
///   possible interruptions in the middle of a critical section are due to panics, but with async
///   code cancellations are a fact of life.
///
///   This is the problem that *cancel safety* solves.
///
/// Both of these problems can also be solved in an ad-hoc manner (for example, by carefully
/// checking for and restoring invariants at the start of each critical section). However, the goal
/// of this mutex is to provide a systematic, if conservative, solution to these problems.
///
/// # Panic safety with poisoning
///
/// Like [`std::sync::Mutex`] but *unlike* [`tokio::sync::Mutex`], this mutex implements a strategy
/// called "poisoning" where a mutex is considered poisoned whenever a task panics while holding the
/// mutex. Once a mutex is poisoned, all other tasks are unable to access the data by default.
///
/// This means that the [`lock`](Self::lock) and [`try_lock`](Self::try_lock) methods return a
/// [`Result`] which indicates whether a mutex has been poisoned or not. Most usage of a mutex will
/// simply [`unwrap()`](Result::unwrap) these results, propagating panics among tasks to ensure that
/// a possibly invalid invariant is not witnessed.
///
/// A poisoned mutex, however, does not prevent all access to the underlying data. The
/// [`PoisonError`](std::sync::PoisonError) type has an
/// [`into_inner`](std::sync::PoisonError::into_inner) method which will return the guard that would
/// have otherwise been returned on a successful lock. This allows access to the data, despite the
/// lock being poisoned.
///
/// # Cancel safety
///
/// To prevent async cancellations in the middle of the critical section, this mutex does not allow
/// await points to be within a critical section. This is done by returning [`ActionPermit`]
/// instances which only provide access to the guarded data within a synchronous closure, as opposed
/// to the RAII style that [`std::sync::MutexGuard`] and [`tokio::sync::MutexGuard`] use.
///
/// This does mean that there are patterns that are not possible with this mutex. For example, you
/// cannot perform a pattern where:
///
/// 1. You acquire a lock *L₁*.
/// 2. You acquire a second lock *L₂*.
/// 3. You release *L₁*.
/// 4. You release *L₂*.
///
/// But generally speaking, you should release *L₂* before *L₁*. If you really do need to do this,
/// [`std::sync::Mutex`] and [`tokio::sync::Mutex`] remain available.
///
/// # Examples
///
/// The above example, rewritten to use a `RobustMutex`, would look like:
///
/// ```
/// use cancel_safe_futures::sync::RobustMutex;
/// use std::collections::HashMap;
///
/// struct MyStruct {
///     map1: HashMap<String, String>,
///     map2: HashMap<String, u32>,
/// }
///
/// impl MyStruct {
/// # /*
///     fn new() -> Self { /* ... */ }
/// # */
/// #    fn new() -> Self {
/// #        Self {
/// #            map1: HashMap::new(),
/// #            map2: HashMap::new(),
/// #        }
/// #    }
/// }
///
/// #[tokio::main]
/// async fn main() {
///     let mutex = RobustMutex::new(MyStruct::new());
///
///     let mut permit = mutex.lock().await.unwrap();  // note unwrap() here
///     permit.perform(|data| {
///         data.map1.insert("hello".to_owned(), "world".to_owned());  // (1)
///         // ... some code in between
///         data.map2.insert("hello".to_owned(), 42);  // (2)
///     });
/// }
/// ```
///
/// # Features
///
/// Basic mutex operations are supported, as well as [`ActionPermit::map`] to produce a
/// [`MappedActionPermit`]. In the future, this will support:
///
/// - An `OwnedActionPermit`, similar to [`tokio::sync::OwnedMutexGuard`].
///
/// # Why "robust"?
///
/// The name is derived from POSIX's [`pthread_mutexattr_getrobust` and
/// `pthread_mutexattr_setrobust`](https://pubs.opengroup.org/onlinepubs/9699919799/functions/pthread_mutexattr_getrobust.html).
/// These functions aim to achieve very similar goals to this mutex, except in slightly different
/// circumstances (*thread* cancellations and terminations rather than *task* cancellations and
/// panics).
pub struct RobustMutex<T: ?Sized> {
    poison: poison::Flag,
    inner: tokio::sync::Mutex<T>,
}

impl<T: ?Sized> RobustMutex<T> {
    /// Creates a new lock in an unlocked state ready for use.
    ///
    /// # Examples
    ///
    /// ```
    /// use cancel_safe_futures::sync::RobustMutex;
    ///
    /// let lock = RobustMutex::new(5);
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
    /// use cancel_safe_futures::sync::RobustMutex;
    ///
    /// static LOCK: RobustMutex<i32> = RobustMutex::const_new(5);
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

    /// Locks this mutex, causing the current task to yield until the lock has been acquired.  When
    /// the lock has been acquired, function returns a [`ActionPermit`].
    ///
    /// # Errors
    ///
    /// If another user of this mutex panicked while holding the mutex, then this call will return
    /// an error once the mutex is acquired.
    ///
    /// # Cancel safety
    ///
    /// This method uses a queue to fairly distribute locks in the order they were requested.
    /// Cancelling a call to `lock` makes you lose your place in the queue.
    ///
    /// # Examples
    ///
    /// ```
    /// use cancel_safe_futures::sync::RobustMutex;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let mutex = RobustMutex::new(1);
    ///
    ///     let mut permit = mutex.lock().await.unwrap();
    ///     permit.perform(|n| *n = 2);
    /// }
    /// ```
    pub async fn lock(&self) -> LockResult<ActionPermit<'_, T>> {
        let guard = self.inner.lock().await;
        ActionPermit::new(guard, &self.poison)
    }

    /// Blockingly locks this `Mutex`. When the lock has been acquired, the function returns a
    /// [`ActionPermit`].
    ///
    /// This method is intended for use cases where you need to use this mutex in asynchronous code
    /// as well as in synchronous code.
    ///
    /// # Errors
    ///
    /// If another user of this mutex panicked while holding the mutex, then this call will return
    /// an error once the mutex is acquired.
    ///
    /// # Panics
    ///
    /// This function panics if called within an asynchronous execution context.
    ///
    /// # Examples
    ///
    /// ```
    /// use cancel_safe_futures::sync::RobustMutex;
    /// use std::sync::Arc;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let mutex = Arc::new(RobustMutex::new(1));
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

    /// Attempts to acquire the lock, returning an [`ActionPermit`] if successful.
    ///
    /// # Errors
    ///
    /// Returns [`TryLockError::WouldBlock`] if the lock is currently held somewhere else.
    ///
    /// Returns [`TryLockError::Poisoned`] if another thread panicked while holding the lock.
    ///
    /// # Examples
    ///
    /// ```
    /// use cancel_safe_futures::sync::RobustMutex;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let mutex = RobustMutex::new(1);
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
    /// use cancel_safe_futures::sync::RobustMutex;
    /// use std::sync::Arc;
    /// use std::thread;
    ///
    /// # #[tokio::main]
    /// # async fn main() {
    ///
    /// let mutex = Arc::new(RobustMutex::new(0));
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
    /// use cancel_safe_futures::sync::RobustMutex;
    ///
    /// let mut mutex = RobustMutex::new(1);
    ///
    /// let n = mutex.get_mut();
    /// *n = 2;
    /// ```
    pub fn get_mut(&mut self) -> &mut T {
        self.inner.get_mut()
    }

    /// Consumes the mutex, returning the underlying data.
    ///
    /// # Errors
    ///
    /// If another user of this mutex panicked while holding the mutex, then this call will return
    /// an error.
    ///
    /// # Examples
    ///
    /// ```
    /// use cancel_safe_futures::sync::RobustMutex;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let mutex = RobustMutex::new(1);
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

impl<T: ?Sized + fmt::Debug> fmt::Debug for RobustMutex<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut d = f.debug_struct("RobustMutex");
        match self.try_lock() {
            Ok(inner) => d.field("data", &inner.guard),
            Err(_) => d.field("data", &format_args!("<locked>")),
        };
        d.field("poisoned", &self.poison.get());
        d.finish()
    }
}

/// A token that grants the ability to run one closure against the data guarded by a [`RobustMutex`].
///
/// This is produced by the `lock` family of operations on [`RobustMutex`] and is intended to provide
/// robust cancel safety.
///
/// For more information, see the documentation for [`RobustMutex`].
#[clippy::has_significant_drop]
pub struct ActionPermit<'a, T: ?Sized> {
    poison: &'a poison::Flag,
    poison_guard: poison::Guard,
    guard: MutexGuard<'a, T>,
}

impl<'a, T: ?Sized> ActionPermit<'a, T> {
    /// Invariant: the mutex must be locked when this is called.
    fn new(guard: MutexGuard<'a, T>, poison: &'a poison::Flag) -> LockResult<Self> {
        poison::map_result(poison.guard(), |poison_guard| Self {
            guard,
            poison,
            poison_guard,
        })
    }

    /// Runs a closure with access to the guarded data, consuming the permit in the process and
    /// unlocking the mutex once the closure completes.
    ///
    /// This is a synchronous closure, which means that it cannot have await points within it. This
    /// guarantees cancel safety for this mutex.
    ///
    /// # Notes
    ///
    /// `action` is *not* run inside a synchronous context. This means that operations like
    /// [`tokio::sync::mpsc::Sender::blocking_send`] will panic inside `action`.
    ///
    /// If `action` panics, the mutex is marked poisoned.
    ///
    /// # Examples
    ///
    /// ```
    /// use cancel_safe_futures::sync::RobustMutex;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let mutex = RobustMutex::new(1);
    ///
    ///     let permit = mutex.lock().await.unwrap();
    ///     permit.perform(|n| *n = 2);
    /// }
    /// ```
    pub fn perform<R, F>(mut self, action: F) -> R
    where
        F: FnOnce(&mut T) -> R,
    {
        action(&mut *self.guard)

        // Note: we're relying on the Drop impl for `self` to unlock the mutex.
    }

    /// Makes a new [`MappedActionPermit`] for a component of the locked data.
    ///
    /// This operation cannot fail as the [`ActionPermit`] passed in already locked the mutex.
    ///
    /// # Notes
    ///
    /// If `f` panics, the mutex is marked poisoned.
    ///
    /// # Examples
    ///
    /// ```
    /// use cancel_safe_futures::sync::RobustMutex;
    ///
    /// #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    /// struct Foo(u32);
    ///
    /// # #[tokio::main]
    /// # async fn main() {
    /// let foo = RobustMutex::new(Foo(1));
    ///
    /// {
    ///     let mapped = foo.lock().await.unwrap().map(|f| &mut f.0);
    ///     mapped.perform(|n| *n = 2);
    /// }
    ///
    /// let permit = foo.lock().await.unwrap();
    /// permit.perform(|f| assert_eq!(*f, Foo(2)));
    /// # }
    /// ```
    #[inline]
    pub fn map<U, F>(self, f: F) -> MappedActionPermit<'a, U>
    where
        F: FnOnce(&mut T) -> &mut U,
    {
        // SAFETY: This duplicates guard and then forgets the original. In the end, we have not
        // duplicated or forgotten any values.
        let guard = MutexGuard::map(unsafe { std::ptr::read(&self.guard) }, f);
        let inner = self.skip_drop();

        MappedActionPermit {
            poison: inner.poison,
            poison_guard: inner.poison_guard,
            guard,
        }
    }

    /// Attempts to make a new [`MappedActionPermit`] for a component of the locked data. The
    /// original guard is returned if the closure returns `None`.
    ///
    /// This operation cannot fail as the [`ActionPermit`] passed in already locked the mutex.
    ///
    /// # Notes
    ///
    /// If `f` panics, the mutex is marked poisoned.
    ///
    /// # Examples
    ///
    /// ```
    /// use cancel_safe_futures::sync::RobustMutex;
    ///
    /// #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    /// struct Foo(u32);
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let foo = RobustMutex::new(Foo(1));
    ///
    ///     {
    ///         let mapped = foo.lock().await.unwrap().try_map(|f| Some(&mut f.0))
    ///              .expect("should not fail");
    ///         mapped.perform(|n| *n = 2);
    ///     }
    ///
    ///     let permit = foo.lock().await.unwrap();
    ///     permit.perform(|f| assert_eq!(*f, Foo(2)));
    /// }
    #[inline]
    pub fn try_map<U, F>(self, f: F) -> Result<MappedActionPermit<'a, U>, Self>
    where
        F: FnOnce(&mut T) -> Option<&mut U>,
    {
        // SAFETY: This duplicates guard and then forgets the original. In the end, we have not
        // duplicated or forgotten any values.
        let guard = MutexGuard::try_map(unsafe { std::ptr::read(&self.guard) }, f);
        let inner = self.skip_drop();

        match guard {
            Ok(guard) => Ok(MappedActionPermit {
                poison: inner.poison,
                poison_guard: inner.poison_guard,
                guard,
            }),
            Err(guard) => Err(ActionPermit {
                poison: inner.poison,
                poison_guard: inner.poison_guard,
                // This is the original guard.
                guard,
            }),
        }
    }

    fn skip_drop(self) -> ActionPermitInner<'a> {
        let me = std::mem::ManuallyDrop::new(self);
        // SAFETY: This duplicates poison_guard and then forgets the original. In the end, we have
        // not duplicated or forgotten any values.
        unsafe {
            ActionPermitInner {
                poison: me.poison,
                poison_guard: std::ptr::read(&me.poison_guard),
            }
        }
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

/// A handle to a held [`RobustMutex`] that has had a function applied to it via [`ActionPermit::map`].
///
/// This can be used to hold a subfield of the protected data.
#[clippy::has_significant_drop]
pub struct MappedActionPermit<'a, T: ?Sized> {
    poison: &'a poison::Flag,
    poison_guard: poison::Guard,
    guard: MappedMutexGuard<'a, T>,
}

impl<'a, T: ?Sized> MappedActionPermit<'a, T> {
    /// Runs a closure with access to the guarded data, consuming the permit in the process and
    /// unlocking the mutex once the closure completes.
    ///
    /// This is a synchronous closure, which means that it cannot have await points within it. This
    /// guarantees cancel safety for this mutex.
    ///
    /// # Notes
    ///
    /// `action` is *not* run inside a synchronous context. This means that operations like
    /// [`tokio::sync::mpsc::Sender::blocking_send`] will panic inside `action`.
    ///
    /// If `action` panics, the mutex is marked poisoned.
    ///
    /// # Examples
    ///
    /// ```
    /// use cancel_safe_futures::sync::RobustMutex;
    ///
    /// #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    /// struct Foo(u32);
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let mutex = RobustMutex::new(Foo(1));
    ///
    ///     let permit = mutex.lock().await.unwrap();
    ///     let mapped = permit.map(|foo| &mut foo.0);
    ///     mapped.perform(|n| *n = 2);
    ///
    ///     let permit2 = mutex.lock().await.unwrap();
    ///     permit2.perform(|foo| assert_eq!(*foo, Foo(2)));
    /// }
    /// ```
    pub fn perform<R, F>(mut self, action: F) -> R
    where
        F: FnOnce(&mut T) -> R,
    {
        action(&mut *self.guard)

        // Note: we're relying on the Drop impl for `self` to unlock the mutex.
    }

    /// Makes a new [`MappedActionPermit`] for a component of the locked data.
    ///
    /// This operation cannot fail as the [`MappedActionPermit`] passed in already locked the mutex.
    ///
    /// # Notes
    ///
    /// If `f` panics, the mutex is marked poisoned.
    #[inline]
    pub fn map<U, F>(self, f: F) -> MappedActionPermit<'a, U>
    where
        F: FnOnce(&mut T) -> &mut U,
    {
        // SAFETY: This duplicates guard and then forgets the original. In the end, we have not
        // duplicated or forgotten any values.
        let guard = MappedMutexGuard::map(unsafe { std::ptr::read(&self.guard) }, f);
        let inner = self.skip_drop();

        MappedActionPermit {
            poison: inner.poison,
            poison_guard: inner.poison_guard,
            guard,
        }
    }

    /// Attempts to make a new [`MappedActionPermit`] for a component of the locked data. The
    /// original guard is returned if the closure returns `None`.
    ///
    /// This operation cannot fail as the [`MappedActionPermit`] passed in already locked the mutex.
    ///
    /// # Notes
    ///
    /// If `f` panics, the mutex is marked poisoned.
    #[inline]
    pub fn try_map<U, F>(self, f: F) -> Result<MappedActionPermit<'a, U>, Self>
    where
        F: FnOnce(&mut T) -> Option<&mut U>,
    {
        // SAFETY: This duplicates guard and then forgets the original. In the end, we have not
        // duplicated or forgotten any values.
        let guard = MappedMutexGuard::try_map(unsafe { std::ptr::read(&self.guard) }, f);
        let inner = self.skip_drop();

        match guard {
            Ok(guard) => Ok(MappedActionPermit {
                poison: inner.poison,
                poison_guard: inner.poison_guard,
                guard,
            }),
            Err(guard) => Err(MappedActionPermit {
                poison: inner.poison,
                poison_guard: inner.poison_guard,
                // This is the original guard.
                guard,
            }),
        }
    }

    fn skip_drop(self) -> MappedActionPermitInner<'a> {
        let me = std::mem::ManuallyDrop::new(self);
        // SAFETY: This duplicates poison_guard and then forgets the original. In the end,
        // we have not duplicated or forgotten any values.
        unsafe {
            MappedActionPermitInner {
                poison: me.poison,
                poison_guard: std::ptr::read(&me.poison_guard),
            }
        }
    }
}

impl<T: ?Sized> Drop for MappedActionPermit<'_, T> {
    #[inline]
    fn drop(&mut self) {
        self.poison.done(&self.poison_guard);
    }
}

impl<T: ?Sized + fmt::Debug> fmt::Debug for MappedActionPermit<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&*self.guard, f)
    }
}

impl<T: ?Sized + fmt::Display> fmt::Display for MappedActionPermit<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&*self.guard, f)
    }
}

/// A helper type used when taking apart an `ActionPermit` without running its
/// Drop implementation.
#[allow(dead_code)] // Unused fields are still used in Drop.
struct ActionPermitInner<'a> {
    poison: &'a poison::Flag,
    poison_guard: poison::Guard,
}

/// A helper type used when taking apart a `MappedActionPermit` without running its
/// Drop implementation.
/// #[allow(dead_code)] // Unused fields are still used in Drop.
struct MappedActionPermitInner<'a> {
    poison: &'a poison::Flag,
    poison_guard: poison::Guard,
}
