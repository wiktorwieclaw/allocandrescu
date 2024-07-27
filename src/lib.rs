//! Allocator combinators library.
//!
//! `allocandrescu` allows you to compose allocators using various combinators such as
//! [`cond`](Allocandrescu::cond) and [`fallback`](Allocandrescu::fallback).
//! It also provides a variety of simple allocators like [`Stack`](crate::alloc::Stack).
//!
//! This crate depends on [`allocator-api2`](https://crates.io/crates/allocator-api2), a polyfill
//! for the unstable [`allocator_api`](https://doc.rust-lang.org/unstable-book/library-features/allocator-api.html) feature.
//!
//! # Usage
//! To extend allocators with methods in this crate, import the [`Allocandrescu`] trait:
//!
//! ```
//! use allocandrescu::Allocandrescu as _;
//! ```
//!
//! # Feature flags
//! - `bumpalo` enables support for [`bumpalo`] crate.
#![cfg_attr(not(test), no_std)]

use allocator_api2::alloc::Allocator;
use combinator::{Cond, Fallback};
use core::{alloc::Layout, ptr::NonNull};

#[cfg(feature = "bumpalo")]
pub use bumpalo;

pub mod alloc;
pub mod combinator;

/// An allocator that knows which allocations it owns.
pub trait AwareAllocator: Allocator {
    /// Returns `true` if the allocator owns the allocation indicated by `ptr` with `layout`.
    fn owns(&self, ptr: NonNull<u8>, layout: Layout) -> bool;
}

/// Extension trait for [`Allocator`] trait that provides methods for combining allocators.
pub trait Allocandrescu: Sized {
    /// Combines an allocator with a predicate, allowing allocation only if the predicate returns
    /// `true`.
    ///
    /// # Example
    /// ```
    /// use allocandrescu::{alloc::Stack, Allocandrescu as _, AwareAllocator as _};
    /// use std::ptr::{addr_of, NonNull};
    ///
    /// // Use fallback allocator for allocations larger than 16.
    /// let alloc = Stack::<1024>::new()
    ///     .cond(|layout| layout.size() <= 16)
    ///     .fallback(std::alloc::System);
    /// let layout = std::alloc::Layout::new::<u8>();
    ///
    /// let mut v = allocator_api2::vec![in &alloc; 0u8; 16];
    /// assert!(alloc.primary().owns(NonNull::new(addr_of!(v[0]).cast_mut()).unwrap(), layout));
    /// assert!(alloc.primary().owns(NonNull::new(addr_of!(v[15]).cast_mut()).unwrap(), layout));
    ///
    /// v.push(0); // reallocates using fallback allocator
    /// assert!(!alloc.primary().owns(NonNull::new(addr_of!(v[0]).cast_mut()).unwrap(), layout));
    /// assert!(!alloc.primary().owns(NonNull::new(addr_of!(v[16]).cast_mut()).unwrap(), layout));
    /// ```
    fn cond<F>(self, pred: F) -> Cond<Self, F>
    where
        F: Fn(Layout) -> bool,
    {
        Cond::new(self, pred)
    }

    /// Combines allocator with a secondary allocator to be used if the primary one fails.
    ///
    /// # Example
    /// ```
    /// use allocandrescu::{alloc::Stack, Allocandrescu as _, AwareAllocator as _};
    /// use std::ptr::{addr_of, NonNull};
    ///
    /// let alloc = Stack::<1024>::new().fallback(std::alloc::System);
    /// let layout = std::alloc::Layout::new::<u8>();
    ///
    /// // `v` allocates using `Stack`
    /// let v = allocator_api2::vec![in &alloc; 0u8; 1024];
    /// assert!(alloc.primary().owns(NonNull::new(addr_of!(v[0]).cast_mut()).unwrap(), layout));
    /// assert!(alloc.primary().owns(NonNull::new(addr_of!(v[1023]).cast_mut()).unwrap(), layout));
    ///
    /// // `b` allocates using `System`
    /// let b = allocator_api2::boxed::Box::new_in(0, &alloc);
    /// assert!(!alloc.primary().owns(NonNull::new(addr_of!(*b).cast_mut()).unwrap(), layout));
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
