use crate::alloc::{Allocator, Global, Layout};
use core::array;
use core::fmt;
use core::intrinsics;
use core::iter::{FusedIterator, TrustedFused, TrustedLen, TrustedRandomAccessNoCoerce};
use core::marker::Unsize;
use core::num::NonZero;
use core::ops::CoerceUnsized;
use core::ptr::{NonNull, Unique};
use core::slice::DrainRaw;

/// A `SeqBox<[T], A>` is like a `RawVec<T, A>`,
/// but it also supports `SeqBox<[T; N], A>`
/// which doesn't need to store a separate capacity value.
struct SeqBox<T: ?Sized, A: Allocator> {
    ptr: Unique<T>,
    alloc: A,
}

#[unstable(feature = "coerce_unsized", issue = "18598")]
impl<T: ?Sized, U: ?Sized, A: Allocator> CoerceUnsized<SeqBox<U, A>> for SeqBox<T, A> where
    T: Unsize<U>
{
}

/// # Safety
///
/// For a non-thin `T`, the `ptr`'s metadata must be valid for a legal `T`.
unsafe fn layout_from_ptr<T: ?Sized>(ptr: NonNull<T>) -> Layout {
    let ptr: *const T = ptr.as_ptr();
    unsafe {
        let size = intrinsics::size_of_val::<T>(ptr);
        let align = intrinsics::min_align_of_val::<T>(ptr);
        Layout::from_size_align_unchecked(size, align)
    }
}

unsafe impl<#[may_dangle] T: ?Sized, A: Allocator> Drop for SeqBox<T, A> {
    /// Frees the memory owned by the `SeqBox` *without* trying to drop its contents.
    fn drop(&mut self) {
        let ptr = self.ptr.as_non_null_ptr();
        // SAFETY: this was allocated before, so must be valid now
        unsafe {
            let layout = layout_from_ptr(ptr);
            if layout.size() > 0 {
                self.alloc.deallocate(ptr.cast(), layout);
            }
        }
    }
}

#[unstable(feature = "alloc_internals", issue = "none")]
pub trait ArrayOrSlice {
    #[unstable(feature = "alloc_internals", issue = "none")]
    type Element;
    #[unstable(feature = "alloc_internals", issue = "none")]
    fn ptr_and_len(ptr: NonNull<Self>) -> (NonNull<Self::Element>, usize);
}
#[unstable(feature = "alloc_internals", issue = "none")]
impl<T> ArrayOrSlice for [T] {
    type Element = T;
    fn ptr_and_len(ptr: NonNull<Self>) -> (NonNull<Self::Element>, usize) {
        (ptr.as_non_null_ptr(), ptr.len())
    }
}
#[unstable(feature = "alloc_internals", issue = "none")]
impl<T, const N: usize> ArrayOrSlice for [T; N] {
    type Element = T;
    fn ptr_and_len(ptr: NonNull<Self>) -> (NonNull<Self::Element>, usize) {
        <[T] as ArrayOrSlice>::ptr_and_len(ptr)
    }
}

/// An iterator which moves out of a heap-allocated slice.
///
/// It's used for all of the following:
/// - `Vec<T>::into_iter`, as `IntoIter<T, [T]>`
/// - `Box<[T]>::into_iter`, as `IntoIter<T, [T]>`
/// - `Box<[T; N]>::into_iter`, as `IntoIter<[T; N]>`
#[stable(feature = "boxed_slice_into_iter", since = "CURRENT_RUSTC_VERSION")]
#[rustc_insignificant_dtor]
// The extra `T` parameter is needed for coercions to work, but we don't want people
// to see it in error messages or otherwise, so it has a default.
// The actually-reachable re-exports don't expose it.
#[allow(private_interfaces)]
// IMPORTANT: fields are dropped in-order, so DO NOT reorder them!
pub struct IntoIter<
S: ?Sized,
#[unstable(feature = "allocator_api", issue = "32838")] A: Allocator = Global,
#[unstable(feature = "alloc_internals", issue = "none")] T = <S as ArrayOrSlice>::Element> {
    // When dropping, this field drops the owned elements...
    pub(crate) drain: DrainRaw<T>,
    // ...then this one deallocates, even if a drop panicked.
    #[allow(dead_code)]
    raw: SeqBox<S, A>,
}

#[stable(feature = "boxed_slice_into_iter", since = "CURRENT_RUSTC_VERSION")]
impl<S: ?Sized, A: Allocator, T: fmt::Debug> fmt::Debug for IntoIter<S, A, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        f.debug_tuple("IntoIter").field(&self.as_shortlived_slice()).finish()
    }
}

#[unstable(feature = "coerce_unsized", issue = "18598")]
impl<T, X: ?Sized, Y: Unsize<X> + ?Sized, A: Allocator> CoerceUnsized<IntoIter<X, A, T>>
    for IntoIter<Y, A, T>
{
}

impl<S: ?Sized, T, A: Allocator> IntoIter<S, A, T> {
    pub(crate) unsafe fn from_unique_and_allocator(ptr: Unique<S>, alloc: A) -> Self
        where S: ArrayOrSlice<Element = T>
    {
        let (drain_ptr, len) = ArrayOrSlice::ptr_and_len(ptr.as_non_null_ptr());
        IntoIter {
            drain: unsafe { DrainRaw::from_parts(drain_ptr, len) },
            raw: SeqBox { ptr, alloc },
        }
    }

    pub(crate) fn as_shortlived_slice(&self) -> &[T] {
        unsafe { self.drain.as_nonnull_slice().as_ref() }
    }
}

#[stable(feature = "boxed_slice_into_iter", since = "CURRENT_RUSTC_VERSION")]
impl<S: ?Sized, T, A: Allocator> Iterator for IntoIter<S, A, T> {
    type Item = T;

    #[inline]
    fn next(&mut self) -> Option<T> {
        self.drain.next()
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.drain.size_hint()
    }

    #[inline]
    fn advance_by(&mut self, n: usize) -> Result<(), NonZero<usize>> {
        self.drain.advance_by(n)
    }

    #[inline]
    fn count(self) -> usize {
        self.len()
    }

    #[inline]
    fn next_chunk<const N: usize>(&mut self) -> Result<[T; N], array::IntoIter<T, N>> {
        self.drain.next_chunk()
    }

    unsafe fn __iterator_get_unchecked(&mut self, i: usize) -> Self::Item
    where
        Self: TrustedRandomAccessNoCoerce,
    {
        // SAFETY: the caller must guarantee that `i` is in bounds of the
        // `Vec<T>`, so `i` cannot overflow an `isize`, and the `self.ptr.add(i)`
        // is guaranteed to pointer to an element of the `Vec<T>` and
        // thus guaranteed to be valid to dereference.
        //
        // Also note the implementation of `Self: TrustedRandomAccessNoCoerce` requires
        // that `T: Copy` so reading elements from the buffer doesn't invalidate
        // them for `Drop`.
        unsafe { self.drain.as_nonnull_slice().get_unchecked_mut(i).read() }
    }
}

#[stable(feature = "boxed_slice_into_iter", since = "CURRENT_RUSTC_VERSION")]
impl<S: ?Sized, T, A: Allocator> DoubleEndedIterator for IntoIter<S, A, T> {
    #[inline]
    fn next_back(&mut self) -> Option<T> {
        self.drain.next_back()
    }

    #[inline]
    fn advance_back_by(&mut self, n: usize) -> Result<(), NonZero<usize>> {
        self.drain.advance_back_by(n)
    }
}

#[stable(feature = "boxed_slice_into_iter", since = "CURRENT_RUSTC_VERSION")]
impl<S: ?Sized, T, A: Allocator> ExactSizeIterator for IntoIter<S, A, T> {
    fn is_empty(&self) -> bool {
        self.drain.is_empty()
    }
    fn len(&self) -> usize {
        self.drain.len()
    }
}

#[stable(feature = "boxed_slice_into_iter", since = "CURRENT_RUSTC_VERSION")]
impl<S: ?Sized, T, A: Allocator> FusedIterator for IntoIter<S, A, T> {}

#[doc(hidden)]
#[unstable(issue = "none", feature = "trusted_fused")]
unsafe impl<S: ?Sized, T, A: Allocator> TrustedFused for IntoIter<S, A, T> {}

#[unstable(feature = "trusted_len", issue = "37572")]
unsafe impl<S: ?Sized, T, A: Allocator> TrustedLen for IntoIter<S, A, T> {}

#[doc(hidden)]
#[unstable(issue = "none", feature = "std_internals")]
#[rustc_unsafe_specialization_marker]
pub trait NonDrop {}

// T: Copy as approximation for !Drop since get_unchecked does not advance self.ptr
// and thus we can't implement drop-handling
#[unstable(issue = "none", feature = "std_internals")]
impl<T: Copy> NonDrop for T {}

#[doc(hidden)]
#[unstable(issue = "none", feature = "std_internals")]
// TrustedRandomAccess (without NoCoerce) must not be implemented because
// subtypes/supertypes of `T` might not be `NonDrop`
unsafe impl<S: ?Sized, T, A: Allocator> TrustedRandomAccessNoCoerce for IntoIter<S, A, T>
where
    T: NonDrop,
{
    const MAY_HAVE_SIDE_EFFECT: bool = false;
}
