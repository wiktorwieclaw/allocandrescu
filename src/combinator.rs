//! Allocator combinators.

use crate::AwareAllocator;
use allocator_api2::alloc::{AllocError, Allocator};
use core::{alloc::Layout, ptr::NonNull};

/// Conditional allocator combinator.
///
/// This `struct` is created by [`fallback`](crate::Allocandrescu::cond) method on [`Allocandrescu`](crate::Allocandrescu).
/// See its documentation for more details.

pub struct Cond<A, F> {
    alloc: A,
    pred: F,
}

impl<A, F> Cond<A, F> {
    pub fn new(alloc: A, pred: F) -> Self {
        Self { alloc, pred }
    }
}

unsafe impl<A, F> Allocator for Cond<A, F>
where
    A: Allocator,
    F: Fn(Layout) -> bool,
{
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        if (self.pred)(layout) {
            self.alloc.allocate(layout)
        } else {
            Err(AllocError)
        }
    }

    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        self.alloc.deallocate(ptr, layout)
    }

    // TODO: optimize default implementations where applicable
}

impl<A, F> AwareAllocator for Cond<A, F>
where
    A: AwareAllocator,
    F: Fn(Layout) -> bool,
{
    fn owns(&self, ptr: NonNull<u8>, layout: Layout) -> bool {
        self.alloc.owns(ptr, layout)
    }
}

/// Fallback allocator combinator.
///
/// This `struct` is created by [`fallback`](crate::Allocandrescu::fallback) method on [`Allocandrescu`](crate::Allocandrescu).
/// See its documentation for more details.
pub struct Fallback<P, S> {
    primary: P,
    secondary: S,
}

impl<P, S> Fallback<P, S> {
    pub fn new(primary: P, secondary: S) -> Self {
        Self { primary, secondary }
    }

    pub fn primary(&self) -> &P {
        &self.primary
    }

    pub fn secondary(&self) -> &S {
        &self.secondary
    }
}

unsafe impl<P, S> Allocator for Fallback<P, S>
where
    P: AwareAllocator,
    S: Allocator,
{
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        self.primary
            .allocate(layout)
            .or_else(|_| self.secondary.allocate(layout))
    }

    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        if self.primary.owns(ptr, layout) {
            self.primary.deallocate(ptr, layout)
        } else {
            self.secondary.deallocate(ptr, layout)
        }
    }
}

impl<P, S> AwareAllocator for Fallback<P, S>
where
    P: AwareAllocator,
    S: AwareAllocator,
{
    fn owns(&self, ptr: NonNull<u8>, layout: Layout) -> bool {
        self.primary.owns(ptr, layout) || self.secondary.owns(ptr, layout)
    }
}
