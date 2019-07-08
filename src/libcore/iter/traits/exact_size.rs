/// An iterator that knows its exact length.
///
/// Many [`Iterator`]s don't know how many times they will iterate, but some do.
/// If an iterator knows how many times it can iterate, providing access to
/// that information can be useful. For example, if you want to iterate
/// backwards, a good start is to know where the end is.
///
/// When implementing an `ExactSizeIterator`, you must also implement
/// [`Iterator`]. When doing so, the implementation of [`size_hint`] *must*
/// return the exact size of the iterator.
///
/// [`Iterator`]: trait.Iterator.html
/// [`size_hint`]: trait.Iterator.html#method.size_hint
///
/// The [`len`] method has a default implementation, so you usually shouldn't
/// implement it. However, you may be able to provide a more performant
/// implementation than the default, so overriding it in this case makes sense.
///
/// [`len`]: #method.len
///
/// # Examples
///
/// Basic usage:
///
/// ```
/// // a finite range knows exactly how many times it will iterate
/// let five = 0..5;
///
/// assert_eq!(5, five.len());
/// ```
///
/// In the [module level docs][moddocs], we implemented an [`Iterator`],
/// `Counter`. Let's implement `ExactSizeIterator` for it as well:
///
/// [moddocs]: index.html
///
/// ```
/// # struct Counter {
/// #     count: usize,
/// # }
/// # impl Counter {
/// #     fn new() -> Counter {
/// #         Counter { count: 0 }
/// #     }
/// # }
/// # impl Iterator for Counter {
/// #     type Item = usize;
/// #     fn next(&mut self) -> Option<Self::Item> {
/// #         self.count += 1;
/// #         if self.count < 6 {
/// #             Some(self.count)
/// #         } else {
/// #             None
/// #         }
/// #     }
/// # }
/// impl ExactSizeIterator for Counter {
///     // We can easily calculate the remaining number of iterations.
///     fn len(&self) -> usize {
///         5 - self.count
///     }
/// }
///
/// // And now we can use it!
///
/// let counter = Counter::new();
///
/// assert_eq!(5, counter.len());
/// ```
#[stable(feature = "rust1", since = "1.0.0")]
pub trait ExactSizeIterator: Iterator {
    /// Returns the exact number of times the iterator will iterate.
    ///
    /// This method has a default implementation, so you usually should not
    /// implement it directly. However, if you can provide a more efficient
    /// implementation, you can do so. See the [trait-level] docs for an
    /// example.
    ///
    /// This function has the same safety guarantees as the [`size_hint`]
    /// function.
    ///
    /// [trait-level]: trait.ExactSizeIterator.html
    /// [`size_hint`]: trait.Iterator.html#method.size_hint
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// // a finite range knows exactly how many times it will iterate
    /// let five = 0..5;
    ///
    /// assert_eq!(5, five.len());
    /// ```
    #[inline]
    #[stable(feature = "rust1", since = "1.0.0")]
    fn len(&self) -> usize {
        let (lower, upper) = self.size_hint();
        // Note: This assertion is overly defensive, but it checks the invariant
        // guaranteed by the trait. If this trait were rust-internal,
        // we could use debug_assert!; assert_eq! will check all Rust user
        // implementations too.
        assert_eq!(upper, Some(lower));
        lower
    }
}

#[stable(feature = "rust1", since = "1.0.0")]
impl<I: ExactSizeIterator + ?Sized> ExactSizeIterator for &mut I {
    fn len(&self) -> usize {
        (**self).len()
    }
}

/// An iterator whose size hint knows whether it's empty,
/// even if it might not know exactly how many items it will to produce.
///
/// This means that its `.size_hint()` must either be `(0, Some(0))`, in which case
/// it's definitely empty, or `(a, _)` where `a > 0`, because it's not empty.
///
/// This trait has a default implementation for `ExactSizeIterator` and `TrustedLen`,
/// so you often don't need to implement it yourself.
#[unstable(feature = "exact_size_is_empty", issue = "35428")]
pub trait KnowsEmptyIterator: Iterator {
    /// Returns `true` if the iterator is empty.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// #![feature(exact_size_is_empty)]
    ///
    /// # // This shouldn't be needed, as it's in the prelude,
    /// # // but I'm getting doc test failures without it.
    /// # use std::iter::KnowsEmptyIterator;
    ///
    /// let mut one_element = std::iter::once(0);
    /// assert!(!one_element.is_empty());
    ///
    /// assert_eq!(one_element.next(), Some(0));
    /// assert!(one_element.is_empty());
    ///
    /// assert_eq!(one_element.next(), None);
    /// ```
    #[unstable(feature = "exact_size_is_empty", issue = "35428")]
    #[inline]
    fn is_empty(&self) -> bool {
        let (lower, upper) = self.size_hint();
        debug_assert!(lower > 0 || upper == Some(0));
        lower == 0
   }
}

#[unstable(feature = "implementation_details", issue = "0")]
#[marker] pub trait KnownLength: Iterator {}
#[unstable(feature = "implementation_details", issue = "0")]
impl<I: ExactSizeIterator> KnownLength for I {}
#[unstable(feature = "implementation_details", issue = "0")]
impl<I: super::TrustedLen> KnownLength for I {}

#[unstable(feature = "exact_size_is_empty", issue = "35428")]
impl<I: KnownLength> KnowsEmptyIterator for I {}
