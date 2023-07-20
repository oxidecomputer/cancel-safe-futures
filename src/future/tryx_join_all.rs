//! Definition of the `TryxJoinAll` adapter, waiting for all of a list of
//! futures to finish with either success or error.

#[cfg(not(futures_no_atomic_cas))]
use crate::stream::{TryStreamExt, TryxCollect};
use crate::support::assert_future;
use alloc::{boxed::Box, vec::Vec};
use core::{
    fmt,
    future::Future,
    iter::FromIterator,
    mem,
    pin::Pin,
    task::{Context, Poll},
};
use futures_core::future::TryFuture;
use futures_util::future::{IntoFuture, MaybeDone, TryFutureExt};
#[cfg(not(futures_no_atomic_cas))]
use futures_util::stream::FuturesOrdered;

#[cfg(not(futures_no_atomic_cas))]
pub(crate) const SMALL: usize = 30;

/// Future for the [`tryx_join_all`] function.
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct TryxJoinAll<F>
where
    F: TryFuture,
{
    kind: TryxJoinAllKind<F>,
}

enum TryxJoinAllKind<F>
where
    F: TryFuture,
{
    Small {
        elems: Pin<Box<[MaybeDone<IntoFuture<F>>]>>,
    },
    #[cfg(not(futures_no_atomic_cas))]
    Big {
        // The use of FuturesOrdered here ensures that in case of errors, the first future listed in
        // the iterator that errors out will be returned.
        fut: TryxCollect<FuturesOrdered<IntoFuture<F>>, Vec<F::Ok>>,
    },
}

impl<F> fmt::Debug for TryxJoinAll<F>
where
    F: TryFuture + fmt::Debug,
    F::Ok: fmt::Debug,
    F::Error: fmt::Debug,
    F::Output: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            TryxJoinAllKind::Small { ref elems } => {
                f.debug_struct("TryxJoinAll").field("elems", elems).finish()
            }
            #[cfg(not(futures_no_atomic_cas))]
            TryxJoinAllKind::Big { ref fut, .. } => fmt::Debug::fmt(fut, f),
        }
    }
}

/// Creates a future which represents either a collection of the results of the futures given or an
/// error.
///
/// The returned future will drive execution for all of its underlying futures, collecting the
/// results into a destination `Vec<T>` in the same order as they were provided.
///
/// Unlike [`futures::future::try_join_all`], if any future returns an error then all other futures
/// will **not** be canceled. Instead, all other futures will be run to completion.
///
/// * If all futures complete successfully, then the returned future will succeed with a
///   `Vec` of all the successful results.
/// * If one or more futures fail, then the returned future will error out with the error
///   for the first future listed that failed.
///
/// This function is only available when the `std` or `alloc` feature of this library is activated,
/// and it is activated by default.
///
/// # Why use `tryx_join_all`?
///
/// See the documentation for [`tryx_join`](crate::tryx_join) for a discussion of why you might
/// want to use a `tryx_` adapter.
///
/// # See Also
///
/// `tryx_join_all` will switch to the more powerful [`FuturesOrdered`] for performance reasons if
/// the number of futures is large. You may want to look into using it or its counterpart
/// [`FuturesUnordered`][futures::stream::FuturesUnordered] directly.
///
/// Some examples for additional functionality provided by these are:
///
///  * Adding new futures to the set even after it has been started.
///
///  * Only polling the specific futures that have been woken. In cases where you have a lot of
///    futures this will result in much more efficient polling.
///
/// # Examples
///
/// ```
/// # futures::executor::block_on(async {
/// use futures::future;
/// use cancel_safe_futures::future::tryx_join_all;
///
/// let futures = vec![
///     future::ok::<u32, u32>(1),
///     future::ok::<u32, u32>(2),
///     future::ok::<u32, u32>(3),
/// ];
///
/// assert_eq!(tryx_join_all(futures).await, Ok(vec![1, 2, 3]));
///
/// let futures = vec![
///     future::ok::<u32, u32>(1),
///     future::err::<u32, u32>(2),
///     future::ok::<u32, u32>(3),
/// ];
///
/// assert_eq!(tryx_join_all(futures).await, Err(2));
/// # });
/// ```
pub fn tryx_join_all<I>(iter: I) -> TryxJoinAll<I::Item>
where
    I: IntoIterator,
    I::Item: TryFuture,
{
    let iter = iter.into_iter().map(TryFutureExt::into_future);

    #[cfg(futures_no_atomic_cas)]
    {
        let kind = TryxJoinAllKind::Small {
            elems: iter.map(MaybeDone::Future).collect::<Box<[_]>>().into(),
        };

        assert_future::<Result<Vec<<I::Item as TryFuture>::Ok>, <I::Item as TryFuture>::Error>, _>(
            TryxJoinAll { kind },
        )
    }

    #[cfg(not(futures_no_atomic_cas))]
    {
        let kind = match iter.size_hint().1 {
            Some(max) if max <= SMALL => TryxJoinAllKind::Small {
                elems: iter.map(MaybeDone::Future).collect::<Box<[_]>>().into(),
            },
            _ => TryxJoinAllKind::Big {
                fut: iter.collect::<FuturesOrdered<_>>().tryx_collect(),
            },
        };

        assert_future::<Result<Vec<<I::Item as TryFuture>::Ok>, <I::Item as TryFuture>::Error>, _>(
            TryxJoinAll { kind },
        )
    }
}

impl<F> Future for TryxJoinAll<F>
where
    F: TryFuture,
{
    type Output = Result<Vec<F::Ok>, F::Error>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        enum FinalState {
            Pending,
            AllDone,
        }

        match &mut self.kind {
            TryxJoinAllKind::Small { elems } => {
                let mut state = FinalState::AllDone;

                for elem in iter_pin_mut(elems.as_mut()) {
                    match elem.poll(cx) {
                        Poll::Pending => state = FinalState::Pending,
                        Poll::Ready(()) => {}
                    }
                }

                match state {
                    FinalState::Pending => Poll::Pending,
                    FinalState::AllDone => {
                        let mut elems = mem::replace(elems, Box::pin([]));
                        let results: Result<Vec<_>, _> = iter_pin_mut(elems.as_mut())
                            .map(|e| e.take_output().unwrap())
                            .collect();
                        Poll::Ready(results)
                    }
                }
            }
            #[cfg(not(futures_no_atomic_cas))]
            TryxJoinAllKind::Big { fut } => Pin::new(fut).poll(cx),
        }
    }
}

impl<F> FromIterator<F> for TryxJoinAll<F>
where
    F: TryFuture,
{
    fn from_iter<T: IntoIterator<Item = F>>(iter: T) -> Self {
        tryx_join_all(iter)
    }
}

pub(crate) fn iter_pin_mut<T>(slice: Pin<&mut [T]>) -> impl Iterator<Item = Pin<&mut T>> {
    // Safety: `std` _could_ make this unsound if it were to decide Pin's
    // invariants aren't required to transmit through slices. Otherwise this has
    // the same safety as a normal field pin projection.
    unsafe { slice.get_unchecked_mut() }
        .iter_mut()
        .map(|t| unsafe { Pin::new_unchecked(t) })
}
