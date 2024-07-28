//! A collection of various allocators and allocator combinators
//!
//! This library is inspired by [Andrei Alexandrescu](https://en.wikipedia.org/wiki/Andrei_Alexandrescu)'s
//! CppCon 2015 talk [std::allocator Is to Allocation what std::vector Is to Vexation](https://www.youtube.com/watch?v=LIb3L4vKZ7U)
//! and the [Zig programming language](https://ziglang.org/).
//! 
//! `allocandrescu` allows you to safely compose allocators using combinators such as
//! [`cond`](Allocandrescu::cond) and [`fallback`](Allocandrescu::fallback).
//! It also provides a variety of simple allocators like [`Stack`](crate::alloc::Stack).
//!
//! This crate depends on [`allocator-api2`](https://crates.io/crates/allocator-api2), a polyfill
//! for the unstable [`allocator_api`](https://doc.rust-lang.org/unstable-book/library-features/allocator-api.html) feature.
//!
//! # Usage
//! To extend allocators with methods from this crate, import the [`Allocandrescu`] trait:
//!
//! ```
//! use allocandrescu::Allocandrescu as _;
//! ```
//! or import the prelude:
//! ```
//! use allocandrescu::prelude::*;
//! ```
//! 
//! # Example
//! Allocator that allocates objects smaller than 16 bytes on a stack of size 1024 bytes.
//! For larger objects, it falls back to using the system allocator.
//! Additionally, it prints all allocation results. 
//! ```
//! use allocandrescu::{alloc::Stack, prelude::*};
//! use allocator_api2::vec;
//!
//! let stack = Stack::<1024>::new();
//! let alloc = stack
//!     .by_ref()
//!     .cond(|layout| layout.size() <= 16)
//!     .fallback(std::alloc::System)
//!     .inspect(|layout, result| println!("layout: {layout:?}, result: {result:?}"));
//! let v = vec![in &alloc; 0; 100];
//! ```
//! 
//! # Feature flags
//! - `bumpalo` enables support for [bumpalo](https://crates.io/crates/bumpalo) crate.
#![cfg_attr(not(any(test, docsrs)), no_std)]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]

use allocator_api2::alloc::{AllocError, Allocator};
use combinator::{Cond, Fallback, Inspect};
use core::{alloc::Layout, ptr::NonNull};

#[cfg(feature = "bumpalo")]
pub use bumpalo;

pub mod alloc;
pub mod combinator;

/// Prelude exports all the allocator-related traits.
pub mod prelude {
    pub use crate::{Allocandrescu as _, ArenaAllocator as _};
    pub use allocator_api2::alloc::Allocator as _;
}

/// Allocator that uses region-based memory management.
pub trait ArenaAllocator: Allocator {
    /// Returns `true` if the allocation specified by `ptr` and `layout` is within the allocator's arena.
    fn contains(&self, ptr: NonNull<u8>, layout: Layout) -> bool;
}

impl<A> ArenaAllocator for &A
where
    A: ArenaAllocator,
{
    #[inline]
    fn contains(&self, ptr: NonNull<u8>, layout: Layout) -> bool {
        (*self).contains(ptr, layout)
    }
}

/// Extension trait for [`Allocator`] trait that provides methods for combining allocators.
pub trait Allocandrescu: Sized {
    /// Combines an allocator with a condition. It allocates only if the condition is met.
    ///
    /// # Example
    /// ```
    /// use allocandrescu::{alloc::Stack, prelude::*};
    /// use allocator_api2::vec;
    /// use std::{alloc::Layout, ptr::{addr_of, NonNull}};
    ///
    /// let stack = Stack::<256>::new();
    /// let alloc = stack
    ///     .by_ref()
    ///     .cond(|layout| layout.size() <= 16);
    ///
    /// let mut v = vec![in &alloc; 0u8; 16];
    /// let layout = Layout::new::<u8>();
    /// assert!(stack.contains(NonNull::new(addr_of!(v[0]).cast_mut()).unwrap(), layout));
    /// assert!(stack.contains(NonNull::new(addr_of!(v[15]).cast_mut()).unwrap(), layout));
    ///
    /// let result = v.try_reserve(1);
    /// assert!(result.is_err());
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
    /// use allocandrescu::{alloc::Stack, prelude::*};
    /// use allocator_api2::{boxed::Box, vec};
    /// use std::{alloc::Layout, ptr::{addr_of, NonNull}};
    ///
    /// let stack = Stack::<16>::new();
    /// let alloc = stack.by_ref().fallback(std::alloc::System);
    /// let layout = Layout::new::<u8>();
    ///
    /// let v = vec![in &alloc; 0u8; 16];
    /// assert!(stack.contains(NonNull::new(addr_of!(v[0]).cast_mut()).unwrap(), layout));
    /// assert!(stack.contains(NonNull::new(addr_of!(v[15]).cast_mut()).unwrap(), layout));
    ///
    /// // Allocate a byte even though the stack is full.
    /// let b = Box::new_in(0, &alloc);
    /// assert!(!stack.contains(NonNull::new(addr_of!(*b).cast_mut()).unwrap(), layout));
    /// ```
    fn fallback<S>(self, secondary: S) -> Fallback<Self, S>
    where
        Self: ArenaAllocator,
        S: Allocator,
    {
        Fallback::new(self, secondary)
    }

    /// Combines allocator with a function that does something to each allocation result.
    ///
    /// This combinator is useful for adding logging to allocators.
    ///
    /// # Example
    /// ```
    /// use allocandrescu::{alloc::Stack, prelude::*};
    /// use allocator_api2::vec;
    /// use std::ptr::{addr_of, NonNull};
    ///
    /// let alloc = Stack::<4>::new().inspect(|layout, result| {
    ///     match result {
    ///         Ok(ptr) => println!(
    ///             "alloc success: size={}, align={}, addr={}",
    ///             layout.size(),
    ///             layout.align(),
    ///             ptr.as_ptr().cast::<u8>() as usize
    ///         ),
    ///         Err(AllocError) => println!(
    ///             "alloc failure: size={}, align={}",
    ///             layout.size(),
    ///             layout.align(),
    ///         ),
    ///     }
    /// });
    ///
    /// let mut v = vec![in &alloc; 0u8; 4];
    /// assert!(v.try_reserve(1).is_err());
    /// ```
    ///
    /// Outputs:
    /// ```text
    /// alloc success: size=4, align=1, addr=519418017120
    /// alloc failure: size=8, align=1
    /// ```
    fn inspect<F>(self, f: F) -> Inspect<Self, F>
    where
        F: Fn(Layout, Result<NonNull<[u8]>, AllocError>),
    {
        Inspect::new(self, f)
    }
}

impl<A: Allocator> Allocandrescu for A {}
