//! Allocator combinator library.
//!
//! To extend allocators with methods in this crate, import the [`Allocandrescu`] trait:
//!
//! ```
//! use allocandrescu::Allocandrescu as _;
//! ```
#![cfg_attr(not(test), no_std)]

use allocator_api2::alloc::Allocator;
use combinator::Fallback;
use core::{alloc::Layout, ptr::NonNull};

pub mod alloc;
pub mod combinator;

/// An allocator that knows which allocations it owns.
pub trait AwareAllocator: Allocator {
    /// Returns `true` if the allocator owns the allocation indicated by `ptr` with `layout`.
    fn owns(&self, ptr: NonNull<u8>, layout: Layout) -> bool;
}

/// Extension trait for [`Allocator`] trait that provides methods for combining allocators.
pub trait Allocandrescu: Sized {
    /// Combines allocator with a secondary allocator to be used if the primary one fails.
    ///
    /// # Example
    /// ```
    /// use allocandrescu::{alloc::Stack, Allocandrescu, AwareAllocator as _};
    /// use std::ptr;
    ///
    /// let alloc = Stack::<1024>::new().fallback(std::alloc::System);
    /// let layout = std::alloc::Layout::new::<u8>();
    ///
    /// // `v` allocates in `Stack`
    /// let v = allocator_api2::vec![in &alloc; 0u8; 1024];
    /// let ptr = ptr::NonNull::new(v.as_ptr().cast_mut()).unwrap();
    /// assert!(alloc.primary().owns(ptr, layout));
    /// assert!(alloc.primary().owns(unsafe { ptr.add(1023) }, layout));
    ///
    /// // `b` allocates in `System`
    /// let b = allocator_api2::boxed::Box::new_in(0, &alloc);
    /// let ptr = ptr::NonNull::new(ptr::addr_of!(*b).cast_mut()).unwrap();
    /// assert!(!alloc.primary().owns(ptr, layout));
    /// ```
    fn fallback<S>(self, secondary: S) -> Fallback<Self, S>
    where
        Self: AwareAllocator,
        S: Allocator,
    {
        Fallback::new(self, secondary)
    }
}

impl<A: Allocator> Allocandrescu for A {}
