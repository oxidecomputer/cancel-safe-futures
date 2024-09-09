use super::poison::{self, LockResult, TryLockError, TryLockResult};
use futures_core::future::{BoxFuture, LocalBoxFuture};
use std::fmt;
use tokio::sync::MutexGuard;

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
///   This is the problem that *panic safety* solves.
///
/// * In async code, what if there's an await point between (1) and (2), and the future is dropped
///   at that await point? Then, too, the invariants are violated. With synchronous code the only
///   possible interruptions in the middle of a critical section are due to panics, but with async
///   code cancellations are a fact of life.
///
///   This is the problem that *cancel safety* solves.
///
/// Both of these problems can also be solved in an ad-hoc manner (for example, by carefully
/// checking for and restoring invariants at the start of each critical section). However, **the
/// goal of this mutex is to provide a systematic, if conservative, solution to these problems.**
///
/// # Panic safety
///
/// Like [`std::sync::Mutex`] but *unlike* [`tokio::sync::Mutex`], this mutex implements a strategy
/// called "poisoning" where a mutex is considered poisoned whenever a task panics within one of the
/// [`ActionPermit`] perform methods. Once a mutex is poisoned, all other tasks are unable to access
/// the data by default.
///
/// This means that the [`lock`](Self::lock) and [`try_lock`](Self::try_lock) methods return a
/// [`Result`] which indicates whether a mutex has been poisoned or not. Most usage of a mutex will
/// simply [`unwrap()`](Result::unwrap) these results, propagating panics among tasks to ensure that
/// a possibly invalid invariant is not witnessed.
///
/// A poisoned mutex, however, does not prevent all access to the underlying data. The
/// [`PoisonError`](crate::sync::PoisonError) type has an
/// [`into_inner`](crate::sync::PoisonError::into_inner) method which will return the guard that
/// would have otherwise been returned on a successful lock. This allows access to the data, despite
/// the lock being poisoned.
///
/// # Cancel safety
///
/// To guard against async cancellations in the middle of the critical section, the mutex uses a
/// callback approach. This is done by returning [`ActionPermit`] instances which provide access to
/// the guarded data in two ways:
///
/// 1. [`perform()`], which accepts a synchronous closure that cannot have await points within it.
/// 2. [`perform_async_boxed()`] and [`perform_async_boxed_local()`], which accept asynchronous
///    closures. If the future returned by these methods is cancelled in the middle of execution,
///    the mutex is marked as poisoned.
///
/// In general, it is recommended that [`perform()`] be used and mutexes not be held across await
/// points at all, since that can cause performance and correctness issues.
///
/// Not using an RAII guard like [`std::sync::MutexGuard`] does mean that there are patterns that
/// are not possible with this mutex. For example, you cannot perform a pattern where:
///
/// 1. You acquire a lock *L₁*.
/// 2. You acquire a second lock *L₂*.
/// 3. You release *L₁*.
/// 4. You release *L₂*.
///
/// If you really do need to do this or more complicated patterns, [`std::sync::Mutex`] and
/// [`tokio::sync::Mutex`] remain available.
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
/// Basic mutex operations are supported. In the future, this will support:
///
/// - An `OwnedActionPermit`, similar to [`tokio::sync::OwnedMutexGuard`].
///
/// Mapped action permits similar to [`tokio::sync::MappedMutexGuard`] will likely not be supported
/// because it's hard to define panic and cancel safety in that scenario.
///
/// # Why "robust"?
///
/// The name is derived from POSIX's [`pthread_mutexattr_getrobust` and
/// `pthread_mutexattr_setrobust`](https://pubs.opengroup.org/onlinepubs/9699919799/functions/pthread_mutexattr_getrobust.html).
/// These functions aim to achieve very similar goals to this mutex, except in slightly different
/// circumstances (*thread* cancellations and terminations rather than *task* cancellations and
/// panics).
///
/// [`perform()`]: ActionPermit::perform
/// [`perform_async_boxed()`]: ActionPermit::perform_async_boxed
/// [`perform_async_boxed_local()`]: ActionPermit::perform_async_boxed_local
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
    #[cfg(all(feature = "parking_lot", not(test)))]
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
    /// This is equivalent to [`Self::is_panic_poisoned`]` || `[`Self::is_cancel_poisoned`].
    ///
    /// If another task is active, the mutex can still become poisoned at any time. You should not
    /// trust a `false` value for program correctness without additional synchronization.
    #[inline]
    pub fn is_poisoned(&self) -> bool {
        self.poison.get_flags() != poison::NO_POISON
    }

    /// Determines whether the mutex is poisoned due to a panic.
    ///
    /// If another task is active, the mutex can still become poisoned at any time. You should not
    /// trust a `false` value for program correctness without additional synchronization.
    ///
    /// # Examples
    ///
    /// ```
    /// use cancel_safe_futures::sync::RobustMutex;
    /// use std::sync::Arc;
    ///
    /// # #[tokio::main]
    /// # async fn main() {
    ///
    /// let mutex = Arc::new(RobustMutex::new(0));
    /// let c_mutex = Arc::clone(&mutex);
    ///
    /// let _ = tokio::task::spawn(async move {
    ///     let permit = c_mutex.lock().await.unwrap();
    ///     permit.perform(|_| {
    ///         panic!(); // the mutex gets poisoned
    ///     });
    /// }).await;
    ///
    /// assert!(mutex.is_panic_poisoned());
    /// # }
    /// ```
    #[inline]
    pub fn is_panic_poisoned(&self) -> bool {
        self.poison.get_flags() & poison::PANIC_POISON != 0
    }

    /// Determines whether this mutex is poisoned due to a cancellation.
    ///
    /// If another task is active, the mutex can still become poisoned at any time. You should not
    /// trust a `false` value for program correctness without additional synchronization.
    ///
    /// # Examples
    ///
    /// ```
    /// use cancel_safe_futures::sync::RobustMutex;
    /// use futures::FutureExt;
    /// use std::sync::Arc;
    ///
    /// # #[tokio::main]
    /// # async fn main() {
    ///
    /// let mutex = Arc::new(RobustMutex::new(0));
    /// let c_mutex = Arc::clone(&mutex);
    ///
    /// tokio::task::spawn(async move {
    ///     let permit = c_mutex.lock().await.unwrap();
    ///     let fut = permit.perform_async_boxed(|n| async move {
    ///         // Sleep for 1 second.
    ///         tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    ///         *n = 1;
    ///     }.boxed());
    ///     tokio::select! {
    ///         _ = fut => {
    ///             panic!("this branch should not be encountered");
    ///         }
    ///         _ = tokio::time::sleep(std::time::Duration::from_millis(100)) => {
    ///             // Exit the task, causing `fut` to be cancelled after 100ms.
    ///         }
    ///     }
    /// }).await.unwrap();
    ///
    /// assert!(mutex.is_cancel_poisoned());
    ///
    /// # }
    /// ```
    #[inline]
    pub fn is_cancel_poisoned(&self) -> bool {
        self.poison.get_flags() & poison::CANCEL_POISON != 0
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
    #[inline]
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

impl<T> Default for RobustMutex<T>
where
    T: Default,
{
    #[inline]
    fn default() -> Self {
        Self::new(Default::default())
    }
}

impl<T> From<T> for RobustMutex<T> {
    #[inline]
    fn from(t: T) -> Self {
        Self::new(t)
    }
}

impl<T: ?Sized + fmt::Debug> fmt::Debug for RobustMutex<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut d = f.debug_struct("RobustMutex");
        match self.try_lock() {
            Ok(inner) => d.field("data", &inner.guard),
            Err(_) => d.field("data", &format_args!("<locked>")),
        };
        d.field("poisoned", &self.poison);
        d.finish()
    }
}

/// A token that grants the ability to run one closure against the data guarded by a
/// [`RobustMutex`].
///
/// This is produced by the `lock` family of operations on [`RobustMutex`] and is intended to
/// provide robust cancel safety.
///
/// For more information, see the documentation for [`RobustMutex`].
///
/// # Why is this its own type?
///
/// A question some users might have is: why not combine `lock` and `perform`? Why have this type
/// that sits in the middle?
///
/// The answer is that this structure is necessary to provide cancel safety. Consider what happens
/// with a hypothetical `lock_and_perform` function. Let's say we use it in a `select!` statement
/// thus:
///
/// ```rust,no_run
/// use std::sync::LockResult;
/// use std::time::Duration;
/// use tokio::time::sleep;
///
/// # /*
/// struct MyMutex<T> { /* ... */ }
/// # */
/// # struct MyMutex<T> { _marker: std::marker::PhantomData<T> }
///
/// impl<T> MyMutex<T> {
///     fn new(data: T) -> Self {
///         /* ... */
///         todo!();
///     }
///     async fn lock_and_perform<U>(self, action: impl FnOnce(&mut T) -> U) -> LockResult<U> {
///         /* ... */
/// #       todo!()
///     }
/// }
///
/// // Represents some kind of type that is unique and can't be cloned.
/// struct NonCloneableType(u32);
///
/// #[tokio::main]
/// async fn main() {
///     let mutex = MyMutex::new(1);
///     let data = NonCloneableType(2);
///     let sleep = sleep(Duration::from_secs(1));
///
///     let fut = mutex.lock_and_perform(|n| {
///         *n = data.0;
///     });
///
///     tokio::select! {
///         _ = fut => {
///             /* ... */
///         }
///         _ = sleep => {
///             /* ... */
///         }
///     }
/// }
/// ```
///
/// Then, if `sleep` fires before `fut`, the non-cloneable type is dropped without being used. This
/// leads to cancel unsafety.
///
/// This is very similar to the cancel unsafety that [`futures::SinkExt::send`] has, and that this
/// crate's [`SinkExt::reserve`](crate::SinkExt::reserve) solves.
#[derive(Debug)]
pub struct ActionPermit<'a, T: ?Sized> {
    poison: &'a poison::Flag,
    guard: MutexGuard<'a, T>,
}

impl<'a, T: ?Sized> ActionPermit<'a, T> {
    /// Invariant: the mutex must be locked when this is called. (This is ensured by requiring a
    /// guard).
    #[inline]
    fn new(guard: MutexGuard<'a, T>, poison: &'a poison::Flag) -> LockResult<Self> {
        poison::map_result(poison.borrow(), |()| Self { poison, guard })
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
        let poison_guard = self.poison.guard_assuming_no_poison();
        let _poisoner = Poisoner {
            poison: self.poison,
            poison_guard,
        };

        action(&mut *self.guard)

        // Note: we're relying on the Drop impl for `_poisoner` to unlock the mutex.
    }

    /// Runs an asynchronous block in the context of the guarded data, consuming the permit in the
    /// process and unlocking the mutex once the block completes.
    ///
    /// In general, holding asynchronous locks across await points can lead to surprising
    /// performance issues. It is strongly recommended that [`perform`](Self::perform) is used, or
    /// that the code is rewritten to use message passing.
    ///
    /// # Notes
    ///
    /// The mutex is marked poisoned if any of the following occur:
    ///
    /// * The future returned by `action` panics.
    /// * The future returned by this async function is cancelled before being driven to completion.
    ///
    /// Due to [limitations in stable
    /// Rust](https://kevincox.ca/2022/04/16/rust-generic-closure-lifetimes), this accepts a dynamic
    /// [`BoxFuture`] rather than a generic future. Once [async
    /// closures](https://rust-lang.github.io/async-fundamentals-initiative/roadmap/async_closures.html)
    /// are stabilized, this will switch to them.
    ///
    /// # Examples
    ///
    /// ```
    /// use cancel_safe_futures::sync::RobustMutex;
    /// use futures::FutureExt;  // for FutureExt::boxed()
    /// use std::time::Duration;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let mutex = RobustMutex::new(1);
    ///
    ///     let permit = mutex.lock().await.unwrap();
    ///     permit.perform_async_boxed(|n| {
    ///         async move {
    ///             tokio::time::sleep(
    ///                 std::time::Duration::from_millis(100),
    ///             ).await;
    ///             *n = 2;
    ///         }
    ///         .boxed()
    ///     }).await;
    ///
    ///     // Check that the new value of the mutex is 2.
    ///     let permit = mutex.lock().await.unwrap();
    ///     permit.perform(|n| assert_eq!(*n, 2));
    /// }
    /// ```
    pub async fn perform_async_boxed<R, F>(mut self, action: F) -> R
    where
        F: for<'lock> FnOnce(&'lock mut T) -> BoxFuture<'lock, R>,
    {
        let poison_guard = self.poison.guard_assuming_no_poison();
        let mut poisoner = AsyncPoisoner {
            poison: self.poison,
            poison_guard,
            terminated: false,
        };
        // At this point, the future can:
        // * panic, in which case both the panic and (since the future isn't complete) cancel poison
        //   flags are set.
        // * be dropped without being driven to completion, in which case the cancel poison flag is
        //   set.
        let ret = action(&mut *self.guard).await;

        // At this point, the future has completed.
        poisoner.terminated = true;
        ret
    }

    /// Runs a non-`Send` asynchronous block in the context of the guarded data, consuming the
    /// permit in the process and unlocking the mutex once the block completes.
    ///
    /// This is a variant of [`perform_async_boxed`](Self::perform_async_boxed) that allows the
    /// future to be non-`Send`.
    pub async fn perform_async_boxed_local<R, F>(mut self, action: F) -> R
    where
        F: for<'lock> FnOnce(&'lock mut T) -> LocalBoxFuture<'lock, R>,
    {
        let poison_guard = self.poison.guard_assuming_no_poison();
        let mut poisoner = AsyncPoisoner {
            poison: self.poison,
            poison_guard,
            terminated: false,
        };
        // At this point, the future can:
        // * panic, in which case both the panic and (since the future isn't complete) cancel poison
        //   flags are set.
        // * be dropped without being driven to completion, in which case the cancel poison flag is
        //   set.
        let ret = action(&mut *self.guard).await;

        // At this point, the future has completed.
        poisoner.terminated = true;
        ret
    }
}

#[clippy::has_significant_drop]
struct Poisoner<'a> {
    poison: &'a poison::Flag,
    poison_guard: poison::Guard,
}

impl<'a> Drop for Poisoner<'a> {
    #[inline]
    fn drop(&mut self) {
        self.poison.done(&self.poison_guard, false);
    }
}

#[clippy::has_significant_drop]
struct AsyncPoisoner<'a> {
    poison: &'a poison::Flag,
    poison_guard: poison::Guard,
    terminated: bool,
}

impl<'a> Drop for AsyncPoisoner<'a> {
    #[inline]
    fn drop(&mut self) {
        self.poison.done(&self.poison_guard, !self.terminated);
    }
}
