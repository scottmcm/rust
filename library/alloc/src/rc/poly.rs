//! Polymorphic parts of `Rc` & `Weak` which are the same for all `T`s.
//!
//! In programs that use lots of different kinds of `Rc`s, we can save a bunch
//! of duplicate work at codegen time by not monoing these things for every `T`.

#![cfg_attr(no_global_oom_handling, allow(unused))]

use core::alloc::{AllocError, Allocator, Layout};
use core::cell::Cell;
use core::convert::Infallible;
use core::mem::SizedTypeProperties;
use core::ptr::{without_provenance_mut, NonNull};

use crate::alloc::handle_alloc_error;

#[inline]
fn call_handler_on_alloc_error<T>(
    f: impl FnOnce(Layout) -> Result<T, AllocError>,
) -> impl FnOnce(Layout) -> Result<T, Infallible> {
    move |x| Ok(f(x).unwrap_or_else(|_| handle_alloc_error(x)))
}

pub struct AllocatedLayoutAndDataOffet {
    layout: Layout,
    offset: usize,
}

impl AllocatedLayoutAndDataOffet {
    /// Calculate the allocated layout for an `Rc<T>` using the inner value's layout.
    ///
    /// Note that this is **NOT** padded to alignment.  It's thus possible that a
    /// `Rc<u8>` could be `size: 17, align: 8`, though for any value aligned at least
    /// as much as `RcCounts` it'll end up padded to alignment anyway.
    #[inline]
    const fn for_value_layout(value_layout: Layout) -> Self {
        match Layout::new::<RcCounts>().extend(value_layout) {
            Ok((layout, offset)) => Self { layout, offset },
            Err(_) => panic!("This size or alignment is too big to work in an `Rc`"),
        }
    }

    #[inline]
    fn try_allocate<E>(
        &self,
        allocator: impl FnOnce(Layout) -> Result<NonNull<[u8]>, E>,
    ) -> Result<NonNull<RcCounts>, E> {
        let alloc_ptr = allocator(self.layout)?.cast::<RcCounts>();
        // SAFETY: The layout calculations guarantee this is within the just-allocated object.
        Ok(unsafe { alloc_ptr.byte_add(self.offset) })
    }

    #[inline]
    unsafe fn deallocate(&self, allocator: impl Allocator, data_ptr: NonNull<RcCounts>) {
        let alloc_ptr = unsafe { data_ptr.byte_sub(self.offset) };
        unsafe { allocator.deallocate(alloc_ptr.cast(), self.layout) };
    }
}

pub trait CanStoreInRc {
    const LAYOUT_AND_OFFSET: AllocatedLayoutAndDataOffet;
}
impl<T> CanStoreInRc for T {
    const LAYOUT_AND_OFFSET: AllocatedLayoutAndDataOffet =
        { AllocatedLayoutAndDataOffet::for_value_layout(T::LAYOUT) };
}

pub struct RcCounts {
    strong: Cell<usize>,
    weak: Cell<usize>,
}

impl RcCounts {
    fn init_for_new_strong(&self) {
        // There is an implicit weak pointer owned by all the strong
        // pointers, which ensures that the weak destructor never frees
        // the allocation while the strong destructor is running, even
        // if the weak pointer is stored inside the strong one.
        self.strong.set(1);
        self.weak.set(1);
    }
}

/// The polymorphic part of an `Rc`, which doesn't change no matter the held `T`.
#[repr(transparent)]
pub struct StrongPoly {
    /// This pointer is to the data in the RC, as well as the past-the-end pointer for the counts.
    /// That way `deref`/`from_raw`/`into_raw`/etc are always identity,
    /// and the count adjustment instructions are identical for every type.
    /// Invariant: the pointer is always valid as it's allocated.
    after_counts_ptr: NonNull<RcCounts>,
}

impl StrongPoly {
    #[inline]
    fn counts(&self) -> &RcCounts {
        unsafe { self.after_counts_ptr.sub(1).as_ref() }
    }

    #[inline]
    pub fn as_ptr_to<T>(&self) -> NonNull<T> {
        self.after_counts_ptr.cast()
    }

    #[inline]
    pub unsafe fn from_ptr_to<T>(ptr: NonNull<T>) -> Self {
        Self { after_counts_ptr: ptr.cast() }
    }

    #[inline]
    fn try_new<E>(
        layout_and_offset: AllocatedLayoutAndDataOffet,
        allocator: impl FnOnce(Layout) -> Result<NonNull<[u8]>, E>,
    ) -> Result<Self, E> {
        let poly = Self { after_counts_ptr: layout_and_offset.try_allocate(allocator)? };
        poly.counts().init_for_new_strong();
        Ok(poly)
    }

    #[inline]
    pub fn new_uninit(
        layout_and_offset: AllocatedLayoutAndDataOffet,
        allocator: impl Allocator,
    ) -> Self {
        match Self::try_new(
            layout_and_offset,
            call_handler_on_alloc_error(move |x| allocator.allocate(x)),
        ) {
            Ok(poly) => poly,
        }
    }

    #[inline]
    pub fn new_zeroed(
        layout_and_offset: AllocatedLayoutAndDataOffet,
        allocator: impl Allocator,
    ) -> Self {
        match Self::try_new(
            layout_and_offset,
            call_handler_on_alloc_error(move |x| allocator.allocate_zeroed(x)),
        ) {
            Ok(poly) => poly,
        }
    }

    #[inline]
    pub fn try_new_uninit(
        layout_and_offset: AllocatedLayoutAndDataOffet,
        allocator: impl Allocator,
    ) -> Result<Self, AllocError> {
        Self::try_new(layout_and_offset, move |x| allocator.allocate(x))
    }

    #[inline]
    pub fn try_new_zeroed(
        layout_and_offset: AllocatedLayoutAndDataOffet,
        allocator: impl Allocator,
    ) -> Result<Self, AllocError> {
        Self::try_new(layout_and_offset, move |x| allocator.allocate_zeroed(x))
    }
}

#[repr(transparent)]
pub struct WeakPoly {
    /// Either `Self::SENTINEL`, or the same as in a `StrongPoly`.
    after_counts_ptr: NonNull<RcCounts>,
}

impl WeakPoly {
    const SENTINEL: NonNull<RcCounts> = {
        // Because the pointer is to *after* the two counts,
        // `1` can never be a valid a
        let ptr = without_provenance_mut(1);
        NonNull::new(ptr).unwrap()
    };

    #[inline]
    fn as_strong(&self) -> Option<StrongPoly> {
        let Self { after_counts_ptr } = *self;
        if after_counts_ptr == Self::SENTINEL {
            None
        } else {
            Some(StrongPoly { after_counts_ptr })
        }
    }
}
