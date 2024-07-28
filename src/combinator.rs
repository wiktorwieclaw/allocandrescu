//! A collection of allocator combinator `struct`s.
//!
//! See the [`Allocandrescu`](`crate::Allocandrescu`) extension trait for an ergonomic way of combining allocators.

use crate::ArenaAllocator;
use allocator_api2::alloc::{AllocError, Allocator};
use core::{alloc::Layout, ptr::NonNull};

/// An allocator that forwards allocation to `alloc` if the passed predicate succeeds. Fails allocation otherwise.
///
/// This `struct` is created by [`fallback`](crate::Allocandrescu::cond) method on [`Allocandrescu`](crate::Allocandrescu).
/// See its documentation for more details.
#[derive(Debug)]
pub struct Cond<A, F> {
    alloc: A,
    pred: F,
}

impl<A, F> Cond<A, F> {
    #[inline]
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

    #[inline]
    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        self.alloc.deallocate(ptr, layout)
    }

    // TODO: optimize default implementations where applicable
}

impl<A, F> ArenaAllocator for Cond<A, F>
where
    A: ArenaAllocator,
    F: Fn(Layout) -> bool,
{
    #[inline]
    fn contains(&self, ptr: NonNull<u8>, layout: Layout) -> bool {
        self.alloc.contains(ptr, layout)
    }
}

/// An allocator that forwards allocation to `primary` allocator. If the allocation fails, it fallbacks to the `secondary` allocator.
///
/// This `struct` is created by [`fallback`](crate::Allocandrescu::fallback) method on [`Allocandrescu`](crate::Allocandrescu).
/// See its documentation for more details.
#[derive(Debug)]
pub struct Fallback<P, S> {
    primary: P,
    secondary: S,
}

impl<P, S> Fallback<P, S> {
    #[inline]
    pub fn new(primary: P, secondary: S) -> Self {
        Self { primary, secondary }
    }

    #[inline]
    pub fn primary(&self) -> &P {
        &self.primary
    }

    #[inline]
    pub fn secondary(&self) -> &S {
        &self.secondary
    }
}

unsafe impl<P, S> Allocator for Fallback<P, S>
where
    P: ArenaAllocator,
    S: Allocator,
{
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        self.primary
            .allocate(layout)
            .or_else(|_| self.secondary.allocate(layout))
    }

    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        if self.primary.contains(ptr, layout) {
            self.primary.deallocate(ptr, layout)
        } else {
            self.secondary.deallocate(ptr, layout)
        }
    }
}

impl<P, S> ArenaAllocator for Fallback<P, S>
where
    P: ArenaAllocator,
    S: ArenaAllocator,
{
    #[inline]
    fn contains(&self, ptr: NonNull<u8>, layout: Layout) -> bool {
        self.primary.contains(ptr, layout) || self.secondary.contains(ptr, layout)
    }
}

/// An allocator that forwards allocation to `alloc` and calls the provided closure on each result.
///
/// This `struct` is created by [`fallback`](crate::Allocandrescu::inspect) method on [`Allocandrescu`](crate::Allocandrescu).
/// See its documentation for more details.
#[derive(Debug)]
pub struct Inspect<A, F> {
    alloc: A,
    f: F,
}

impl<A, F> Inspect<A, F> {
    pub fn new(alloc: A, f: F) -> Self {
        Self { alloc, f }
    }
}

unsafe impl<A, F> Allocator for Inspect<A, F>
where
    A: Allocator,
    F: Fn(Layout, Result<NonNull<[u8]>, AllocError>),
{
    #[inline]
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        let result = self.alloc.allocate(layout);
        (self.f)(layout, result);
        result
    }

    #[inline]
    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        self.alloc.deallocate(ptr, layout);
    }
}

impl<A, F> ArenaAllocator for Inspect<A, F>
where
    A: ArenaAllocator,
    F: Fn(Layout, Result<NonNull<[u8]>, AllocError>),
{
    #[inline]
    fn contains(&self, ptr: NonNull<u8>, layout: Layout) -> bool {
        self.alloc.contains(ptr, layout)
    }
}
