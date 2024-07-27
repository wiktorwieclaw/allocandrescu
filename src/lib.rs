#![cfg_attr(not(test), no_std)]

use allocator_api2::alloc::Allocator;
use combinator::Fallback;
use core::{alloc::Layout, ptr::NonNull};

pub mod alloc;
pub mod combinator;

pub mod prelude {
    pub use super::AwareAllocatorExt as _;
}

/// Allocator that is aware of which pointers are owned by it.
pub trait AwareAllocator: Allocator {
    fn owns(&self, ptr: NonNull<u8>, layout: Layout) -> bool;
}

/// [`AwareAllocator`] extension trait.
pub trait AwareAllocatorExt: Sized {
    fn fallback<S: Allocator>(self, secondary: S) -> Fallback<Self, S>;
}

impl<A: AwareAllocator> AwareAllocatorExt for A {
    fn fallback<S: Allocator>(self, secondary: S) -> Fallback<Self, S> {
        Fallback::new(self, secondary)
    }
}
