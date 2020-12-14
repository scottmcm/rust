#![stable(feature = "futures_api", since = "1.36.0")]

use crate::ops::{self, ControlFlow};
use crate::result::Result;

/// Indicates whether a value is available or if the current task has been
/// scheduled to receive a wakeup instead.
#[must_use = "this `Poll` may be a `Pending` variant, which should be handled"]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[stable(feature = "futures_api", since = "1.36.0")]
pub enum Poll<T> {
    /// Represents that a value is immediately ready.
    #[lang = "Ready"]
    #[stable(feature = "futures_api", since = "1.36.0")]
    Ready(#[stable(feature = "futures_api", since = "1.36.0")] T),

    /// Represents that a value is not ready yet.
    ///
    /// When a function returns `Pending`, the function *must* also
    /// ensure that the current task is scheduled to be awoken when
    /// progress can be made.
    #[lang = "Pending"]
    #[stable(feature = "futures_api", since = "1.36.0")]
    Pending,
}

impl<T> Poll<T> {
    /// Changes the ready value of this `Poll` with the closure provided.
    #[stable(feature = "futures_api", since = "1.36.0")]
    pub fn map<U, F>(self, f: F) -> Poll<U>
    where
        F: FnOnce(T) -> U,
    {
        match self {
            Poll::Ready(t) => Poll::Ready(f(t)),
            Poll::Pending => Poll::Pending,
        }
    }

    /// Returns `true` if this is `Poll::Ready`
    #[inline]
    #[rustc_const_stable(feature = "const_poll", since = "1.49.0")]
    #[stable(feature = "futures_api", since = "1.36.0")]
    pub const fn is_ready(&self) -> bool {
        matches!(*self, Poll::Ready(_))
    }

    /// Returns `true` if this is `Poll::Pending`
    #[inline]
    #[rustc_const_stable(feature = "const_poll", since = "1.49.0")]
    #[stable(feature = "futures_api", since = "1.36.0")]
    pub const fn is_pending(&self) -> bool {
        !self.is_ready()
    }
}

impl<T, E> Poll<Result<T, E>> {
    /// Changes the success value of this `Poll` with the closure provided.
    #[stable(feature = "futures_api", since = "1.36.0")]
    pub fn map_ok<U, F>(self, f: F) -> Poll<Result<U, E>>
    where
        F: FnOnce(T) -> U,
    {
        match self {
            Poll::Ready(Ok(t)) => Poll::Ready(Ok(f(t))),
            Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
            Poll::Pending => Poll::Pending,
        }
    }

    /// Changes the error value of this `Poll` with the closure provided.
    #[stable(feature = "futures_api", since = "1.36.0")]
    pub fn map_err<U, F>(self, f: F) -> Poll<Result<T, U>>
    where
        F: FnOnce(E) -> U,
    {
        match self {
            Poll::Ready(Ok(t)) => Poll::Ready(Ok(t)),
            Poll::Ready(Err(e)) => Poll::Ready(Err(f(e))),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl<T, E> Poll<Option<Result<T, E>>> {
    /// Changes the success value of this `Poll` with the closure provided.
    #[stable(feature = "poll_map", since = "1.51.0")]
    pub fn map_ok<U, F>(self, f: F) -> Poll<Option<Result<U, E>>>
    where
        F: FnOnce(T) -> U,
    {
        match self {
            Poll::Ready(Some(Ok(t))) => Poll::Ready(Some(Ok(f(t)))),
            Poll::Ready(Some(Err(e))) => Poll::Ready(Some(Err(e))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }

    /// Changes the error value of this `Poll` with the closure provided.
    #[stable(feature = "poll_map", since = "1.51.0")]
    pub fn map_err<U, F>(self, f: F) -> Poll<Option<Result<T, U>>>
    where
        F: FnOnce(E) -> U,
    {
        match self {
            Poll::Ready(Some(Ok(t))) => Poll::Ready(Some(Ok(t))),
            Poll::Ready(Some(Err(e))) => Poll::Ready(Some(Err(f(e)))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

#[stable(feature = "futures_api", since = "1.36.0")]
impl<T> From<T> for Poll<T> {
    /// Convert to a `Ready` variant.
    ///
    /// # Example
    ///
    /// ```
    /// # use core::task::Poll;
    /// assert_eq!(Poll::from(true), Poll::Ready(true));
    /// ```
    fn from(t: T) -> Poll<T> {
        Poll::Ready(t)
    }
}

#[stable(feature = "futures_api", since = "1.36.0")]
impl<T, E> ops::Try2015 for Poll<Result<T, E>> {
    type Ok = Poll<T>;
    type Error = E;

    #[inline]
    fn into_result(self) -> Result<Self::Ok, Self::Error> {
        match self {
            Poll::Ready(Ok(x)) => Ok(Poll::Ready(x)),
            Poll::Ready(Err(e)) => Err(e),
            Poll::Pending => Ok(Poll::Pending),
        }
    }

    #[inline]
    fn from_error(e: Self::Error) -> Self {
        Poll::Ready(Err(e))
    }

    #[inline]
    fn from_ok(x: Self::Ok) -> Self {
        x.map(Ok)
    }
}

#[unstable(feature = "try_trait_v2", issue = "42327")]
impl<T, E> ops::TryCore for Poll<Result<T, E>> {
    //type Continue = Poll<T>;
    type Ok = Poll<T>;
    //type Holder = PollResultHolder<E>;
    type Holder = <Result<T, E> as ops::TryCore>::Holder;

    #[inline]
    fn continue_with(c: Self::Ok) -> Self {
        c.map(Ok)
    }

    #[inline]
    fn branch(self) -> ControlFlow<Self::Holder, Self::Ok> {
        match self {
            Poll::Ready(Ok(x)) => ControlFlow::Continue(Poll::Ready(x)),
            Poll::Ready(Err(e)) => ControlFlow::Break(Err(e)),
            Poll::Pending => ControlFlow::Continue(Poll::Pending),
        }
    }
}

/* This is needed if the Try::Holder bound gets tighter again

#[unstable(feature = "try_trait_v2_never_stable", issue = "42327")]
#[allow(missing_debug_implementations)]
/// This type is *only* useful for `expand`ing,
/// so it's intentional that it implements no traits.
pub struct PollResultHolder<E>(E);

#[unstable(feature = "try_trait_v2", issue = "42327")]
impl<T, E> BreakHolder<Poll<T>> for PollResultHolder<E> {
    type Output = Poll<Result<T, E>>;

    fn expand(x: Self) -> Self::Output {
        match x {
            PollResultHolder(e) => Poll::Ready(Err(e)),
        }
    }
}

*/

#[unstable(feature = "try_trait_v2", issue = "42327")]
impl<T, E, F: From<E>> ops::Try2021<Result<!, E>> for Poll<Result<T, F>> {
    fn from_holder(x: Result<!, E>) -> Self {
        match x {
            Err(e) => Poll::Ready(Err(From::from(e))),
        }
    }
}

#[stable(feature = "futures_api", since = "1.36.0")]
impl<T, E> ops::Try2015 for Poll<Option<Result<T, E>>> {
    type Ok = Poll<Option<T>>;
    type Error = E;

    #[inline]
    fn into_result(self) -> Result<Self::Ok, Self::Error> {
        match self {
            Poll::Ready(Some(Ok(x))) => Ok(Poll::Ready(Some(x))),
            Poll::Ready(Some(Err(e))) => Err(e),
            Poll::Ready(None) => Ok(Poll::Ready(None)),
            Poll::Pending => Ok(Poll::Pending),
        }
    }

    #[inline]
    fn from_error(e: Self::Error) -> Self {
        Poll::Ready(Some(Err(e)))
    }

    #[inline]
    fn from_ok(x: Self::Ok) -> Self {
        x.map(|x| x.map(Ok))
    }
}

#[unstable(feature = "try_trait_v2", issue = "42327")]
impl<T, E> ops::TryCore for Poll<Option<Result<T, E>>> {
    //type Continue = Poll<Option<T>>;
    type Ok = Poll<Option<T>>;
    //type Holder = PollOptionResultHolder<E>;
    type Holder = <Result<T, E> as ops::TryCore>::Holder;

    #[inline]
    fn continue_with(c: Self::Ok) -> Self {
        c.map(|x| x.map(Ok))
    }

    #[inline]
    fn branch(self) -> ControlFlow<Self::Holder, Self::Ok> {
        match self {
            Poll::Ready(Some(Ok(x))) => ControlFlow::Continue(Poll::Ready(Some(x))),
            Poll::Ready(Some(Err(e))) => ControlFlow::Break(Err(e)),
            Poll::Ready(None) => ControlFlow::Continue(Poll::Ready(None)),
            Poll::Pending => ControlFlow::Continue(Poll::Pending),
        }
    }
}

/* This is needed if the Try::Holder bound gets tighter again

#[unstable(feature = "try_trait_v2_never_stable", issue = "42327")]
#[allow(missing_debug_implementations)]
/// This type is *only* useful for `expand`ing,
/// so it's intentional that it implements no traits.
pub struct PollOptionResultHolder<E>(E);

#[unstable(feature = "try_trait_v2", issue = "42327")]
impl<T, E> BreakHolder<Poll<Option<T>>> for PollOptionResultHolder<E> {
    type Output = Poll<Option<Result<T, E>>>;

    fn expand(x: Self) -> Self::Output {
        match x {
            PollOptionResultHolder(e) => Poll::Ready(Some(Err(e))),
        }
    }
}

*/

#[unstable(feature = "try_trait_v2", issue = "42327")]
impl<T, E, F: From<E>> ops::Try2021<Result<!, E>> for Poll<Option<Result<T, F>>> {
    fn from_holder(x: Result<!, E>) -> Self {
        match x {
            Err(e) => Poll::Ready(Some(Err(From::from(e)))),
        }
    }
}
