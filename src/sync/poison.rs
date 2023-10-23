// This file is adapted from
// https://github.com/rust-lang/rust/blob/475c71da0710fd1d40c046f9cee04b733b5b2b51/library/std/src/sync/poison.rs
// and is used under the MIT and Apache 2.0 licenses.

use core::sync::atomic::{AtomicU8, Ordering};
use std::{error, fmt, thread};

/// A type of error which can be returned whenever a lock is acquired.
///
/// [`RobustMutex`]es poisoned whenever a thread panics or a cancellation happens while the lock is
/// held. The precise semantics for when a lock is poisoned is documented on each lock, but once a
/// lock is poisoned then all future acquisitions will return this error.
///
/// [`RobustMutex`]: crate::sync::RobustMutex
pub struct PoisonError<T> {
    guard: T,
    flags: u8,
}

impl<T> PoisonError<T> {
    fn new(guard: T, flags: u8) -> PoisonError<T> {
        PoisonError { guard, flags }
    }

    /// Returns true if this error indicates that the lock was poisoned by a
    /// panic from another task.
    pub fn is_panic(&self) -> bool {
        self.flags & PANIC_POISON != 0
    }

    /// Returns true if this error indicates that the lock was poisoned by an early cancellation.
    pub fn is_cancel(&self) -> bool {
        self.flags & CANCEL_POISON != 0
    }

    /// Consumes this error indicating that a lock is poisoned, returning the
    /// underlying guard to allow access regardless.
    pub fn into_inner(self) -> T {
        self.guard
    }

    /// Reaches into this error indicating that a lock is poisoned, returning a
    /// reference to the underlying guard to allow access regardless.
    pub fn get_ref(&self) -> &T {
        &self.guard
    }

    /// Reaches into this error indicating that a lock is poisoned, returning a
    /// mutable reference to the underlying guard to allow access regardless.
    pub fn get_mut(&mut self) -> &mut T {
        &mut self.guard
    }
}

impl<T> fmt::Debug for PoisonError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PoisonError")
            .field("flags", &DebugFlags(self.flags))
            .finish_non_exhaustive()
    }
}

impl<T> fmt::Display for PoisonError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "poisoned lock {:?}", DebugFlags(self.flags))
    }
}

impl<T> error::Error for PoisonError<T> {}

/// An enumeration of possible errors associated with a [`TryLockResult`] which
/// can occur while trying to acquire a lock, from the [`try_lock`] method on a
/// [`RobustMutex`].
///
/// [`try_lock`]: crate::sync::RobustMutex::try_lock
/// [`RobustMutex`]: crate::sync::RobustMutex
pub enum TryLockError<T> {
    /// The lock could not be acquired because another task failed while holding
    /// the lock, or an early cancellation occurred.
    Poisoned(PoisonError<T>),

    /// The lock could not be acquired at this time because the operation would
    /// otherwise block.
    WouldBlock,
}

impl<T> From<PoisonError<T>> for TryLockError<T> {
    fn from(error: PoisonError<T>) -> TryLockError<T> {
        TryLockError::Poisoned(error)
    }
}

impl<T> fmt::Debug for TryLockError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TryLockError::Poisoned(error) => f.debug_tuple("Poisoned").field(&error).finish(),
            TryLockError::WouldBlock => f.debug_tuple("WouldBlock").finish(),
        }
    }
}

impl<T> fmt::Display for TryLockError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TryLockError::Poisoned(error) => {
                write!(f, "poisoned lock {:?}", DebugFlags(error.flags))
            }
            TryLockError::WouldBlock => {
                f.write_str("try_lock failed because the operation would block")
            }
        }
    }
}

impl<T> error::Error for TryLockError<T> {
    fn cause(&self) -> Option<&dyn error::Error> {
        match self {
            TryLockError::Poisoned(error) => Some(error),
            TryLockError::WouldBlock => None,
        }
    }
}

/// A type alias for the result of a lock method which can be poisoned.
///
/// The [`Ok`] variant of this result indicates that the primitive was not
/// poisoned, and the `Guard` is contained within. The [`Err`] variant indicates
/// that the primitive was poisoned. Note that the [`Err`] variant *also* carries
/// the associated guard, and it can be acquired through the [`into_inner`]
/// method.
///
/// [`into_inner`]: PoisonError::into_inner
pub type LockResult<Guard> = Result<Guard, PoisonError<Guard>>;

/// A type alias for the result of a nonblocking locking method.
///
/// For more information, see [`LockResult`]. A `TryLockResult` doesn't
/// necessarily hold the associated guard in the [`Err`] type as the lock might not
/// have been acquired for other reasons.
pub type TryLockResult<Guard> = Result<Guard, TryLockError<Guard>>;

pub const NO_POISON: u8 = 0;
pub const PANIC_POISON: u8 = 1 << 0;
pub const CANCEL_POISON: u8 = 1 << 1;

pub struct Flag {
    failed: AtomicU8,
}

impl fmt::Debug for Flag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let flags = self.failed.load(Ordering::Relaxed);
        DebugFlags(flags).fmt(f)
    }
}

// Note that the Ordering uses to access the `failed` field of `Flag` below is always `Relaxed`, and
// that's because this isn't actually protecting any data, it's just a flag whether we've panicked
// or not.
//
// The actual location that this matters is when a mutex is **locked** which is where we have
// external synchronization ensuring that we see memory reads/writes to this flag.
//
// As a result, if it matters, we should see the correct value for `failed` in all cases.

impl Flag {
    #[inline]
    pub const fn new() -> Flag {
        Flag {
            failed: AtomicU8::new(NO_POISON),
        }
    }

    /// Check the flag for an unguarded borrow, where we only care about existing poison.
    #[inline]
    pub fn borrow(&self) -> LockResult<()> {
        let flags = self.get_flags();
        if flags > NO_POISON {
            Err(PoisonError::new((), flags))
        } else {
            Ok(())
        }
    }

    #[inline]
    pub fn guard_assuming_no_poison(&self) -> Guard {
        Guard {
            panicking: thread::panicking(),
        }
    }

    #[inline]
    pub fn done(&self, guard: &Guard, cancel_poison: bool) {
        let mut new_flags = 0;
        if !guard.panicking && thread::panicking() {
            new_flags |= PANIC_POISON;
        }
        if cancel_poison {
            new_flags |= CANCEL_POISON;
        }
        self.failed.fetch_or(new_flags, Ordering::Relaxed);
    }

    #[inline]
    pub fn get_flags(&self) -> u8 {
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
        Err(error) => {
            let flags = error.flags;
            Err(PoisonError::new(f(error.into_inner()), flags))
        }
    }
}

struct DebugFlags(u8);

impl fmt::Debug for DebugFlags {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0 == NO_POISON {
            write!(f, "(not poisoned)")
        } else {
            let mut poisoned_by = Vec::new();
            if self.0 & PANIC_POISON != 0 {
                poisoned_by.push("panic");
            }
            if self.0 & CANCEL_POISON != 0 {
                poisoned_by.push("async cancellation");
            }

            write!(f, "(poisoned by {})", poisoned_by.join(", "))
        }
    }
}
