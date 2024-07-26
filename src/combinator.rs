use crate::AwareAllocator;
use allocator_api2::alloc::{AllocError, Allocator};
use core::{alloc::Layout, ptr::NonNull};

pub struct Fallback<P, F> {
    primary: P,
    fallback: F,
}

impl<P, F> Fallback<P, F> {
    pub fn new(primary: P, fallback: F) -> Self {
        Self { primary, fallback }
    }
}

unsafe impl<P, F> Allocator for Fallback<P, F>
where
    P: AwareAllocator,
    F: Allocator,
{
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        self.primary
            .allocate(layout)
            .or_else(|_| self.fallback.allocate(layout))
    }

    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        if self.primary.owns(ptr, layout) {
            self.primary.deallocate(ptr, layout)
        } else {
            self.fallback.deallocate(ptr, layout)
        }
    }
}
