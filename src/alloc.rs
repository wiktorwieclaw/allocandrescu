//! Basic allocators.

use crate::ArenaAllocator;
use allocator_api2::alloc::{AllocError, Allocator};
use core::{
    alloc::Layout,
    cell::{Cell, UnsafeCell},
    ptr::{self, NonNull},
};

/// Allocator that always fails allocation.
///
/// Deallocation is a no-op.
#[derive(Debug)]
pub struct Failing;

unsafe impl Allocator for Failing {
    #[inline]
    fn allocate(&self, _layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        Err(AllocError)
    }

    #[inline]
    unsafe fn deallocate(&self, _ptr: NonNull<u8>, _layout: Layout) {}
}

impl ArenaAllocator for Failing {
    #[inline]
    fn contains(&self, _ptr: NonNull<u8>, _layout: Layout) -> bool {
        false
    }
}

/// Stack-based bump allocator.
#[derive(Debug)]
pub struct Stack<const SIZE: usize> {
    stack: UnsafeCell<[u8; SIZE]>,
    idx: Cell<usize>,
}

impl<const SIZE: usize> Default for Stack<SIZE> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<const SIZE: usize> Stack<SIZE> {
    #[inline]
    pub const fn new() -> Self {
        Self {
            stack: UnsafeCell::new([0; SIZE]),
            idx: Cell::new(0),
        }
    }

    /// Reset this stack allocator.
    ///
    /// Performs a mass deallocation on everything allocated in the stack by resetting the pointer.
    /// Does not run any `Drop` implementations on deallocated objects.
    #[inline]
    pub fn reset(&mut self) {
        self.idx.set(0)
    }
}

unsafe impl<const SIZE: usize> Allocator for Stack<SIZE> {
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        let stack = self.stack.get();
        let unaligned_start = self.idx.get();
        let align_offset = stack.align_offset(layout.align());
        let aligned_start = unaligned_start
            .checked_add(align_offset)
            .ok_or(AllocError)?;
        let aligned_end = aligned_start.checked_add(layout.size()).ok_or(AllocError)?;
        if aligned_end > SIZE {
            return Err(AllocError);
        }
        let slice = unsafe {
            let slice = (*stack)
                .get_mut(aligned_start..aligned_end)
                .unwrap_unchecked();
            NonNull::new_unchecked(ptr::addr_of_mut!(*slice))
        };
        self.idx.set(aligned_end);
        Ok(slice)
    }

    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        let idx = self.idx.get();
        let alloc_start = as_usize(ptr);
        let alloc_end = alloc_start.saturating_add(layout.size());
        if alloc_end == idx {
            self.idx.set(alloc_start)
        }
    }

    // TODO: optimize default implementations where applicable
}

// TODO: test owns
impl<const SIZE: usize> ArenaAllocator for Stack<SIZE> {
    fn contains(&self, ptr: NonNull<u8>, layout: Layout) -> bool {
        let stack_start = self.stack.get() as usize;
        let stack_end = stack_start.saturating_add(SIZE);
        let alloc_start = as_usize(ptr);
        let alloc_end = alloc_start.saturating_add(layout.size());
        stack_start <= alloc_start && stack_end >= alloc_end
    }
}

/// Re-rexport of [`bumpalo::Bump`](https://docs.rs/bumpalo/latest/bumpalo/struct.Bump.html).
#[cfg(feature = "bumpalo")]
pub use bumpalo::Bump;

#[cfg(feature = "bumpalo")]
impl ArenaAllocator for &Bump {
    fn contains(&self, ptr: NonNull<u8>, layout: Layout) -> bool {
        unsafe {
            self.iter_allocated_chunks_raw()
                .any(|(chunk_ptr, chunk_size)| {
                    let chunk_start = chunk_ptr as usize;
                    let chunk_end = chunk_start.saturating_add(chunk_size);
                    let alloc_start = as_usize(ptr);
                    let alloc_end = alloc_start.saturating_add(layout.size());
                    chunk_start <= alloc_start && chunk_end >= alloc_end
                })
        }
    }
}

#[inline]
fn as_usize<T>(ptr: NonNull<T>) -> usize {
    ptr.as_ptr() as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stack_allocator_aligns_memory() {
        let alloc = Stack::<16>::new();
        let stack_addr = alloc.stack.get() as usize;

        let layout = Layout::new::<u8>();
        let ptr1 = alloc.allocate(layout).unwrap().cast::<u8>();
        assert_eq!(alloc.idx.get(), 1);
        assert_eq!(as_usize(ptr1), stack_addr);

        let layout = Layout::new::<u32>();
        let ptr1_align_offset = ptr1.align_offset(layout.align());
        let ptr2 = alloc.allocate(layout).unwrap().cast::<u8>();
        let ptr3 = alloc.allocate(layout).unwrap().cast::<u8>();
        assert_eq!(alloc.idx.get(), 4 + 4 + ptr1_align_offset + 1);
        assert_eq!(as_usize(ptr2), 1 + ptr1_align_offset + as_usize(ptr1));
        assert_eq!(as_usize(ptr3), 4 + as_usize(ptr2));
    }

    #[test]
    fn stack_allocator_allocates_zst() {
        let alloc = Stack::<16>::new();
        let stack_addr = alloc.stack.get() as usize;

        let layout = Layout::new::<()>();
        let ptr = alloc.allocate(layout).unwrap().cast::<u8>();
        assert_eq!(alloc.idx.get(), 0);
        assert_eq!(as_usize(ptr), stack_addr);
    }

    #[test]
    fn stack_allocator_handles_out_of_memory() {
        let alloc = Stack::<4>::new();

        let layout = Layout::new::<u8>();
        let _ = alloc.allocate(layout).unwrap();
        assert_eq!(alloc.idx.get(), 1);

        let layout = Layout::new::<u32>();
        let _ptr = alloc.allocate(layout).unwrap_err();
        assert_eq!(alloc.idx.get(), 1);
    }

    #[test]
    fn vec_with_stack_allocator_runs_drop() {
        use allocator_api2::vec::Vec;

        std::thread_local! {
            static COUNTER: Cell<u32> = const { Cell::new(0) };
        }

        struct AddOnDrop(u32);
        impl Drop for AddOnDrop {
            fn drop(&mut self) {
                COUNTER.set(COUNTER.get() + self.0)
            }
        }

        let alloc = Stack::<16>::new();
        let mut v = Vec::new_in(&alloc);
        v.push(AddOnDrop(1));
        v.push(AddOnDrop(2));
        assert_eq!(COUNTER.get(), 0);
        v.pop();
        assert_eq!(COUNTER.get(), 2);
        v.pop();
        assert_eq!(COUNTER.get(), 3);
    }

    #[test]
    fn vec_with_stack_allocator_fails_oob_allocation() {
        use allocator_api2::vec::Vec;

        let alloc = Stack::<8>::new();
        let mut v: Vec<u32, _> = Vec::new_in(&alloc);
        v.try_reserve(3).unwrap_err();
    }

    #[cfg(feature = "bumpalo")]
    #[test]
    fn bumpalo_is_aware_of_its_allocations() {
        use crate::Allocandrescu as _;
        use std::ptr::addr_of;

        let bump = &Bump::with_capacity(8);
        let alloc = bump
            .cond(|layout| layout.size() <= 8)
            .fallback(std::alloc::System);
        let layout = std::alloc::Layout::new::<u8>();

        let v1 = allocator_api2::vec![in &alloc; 0u8; 8];
        assert!(bump.contains(NonNull::new(addr_of!(v1[0]).cast_mut()).unwrap(), layout));
        assert!(bump.contains(NonNull::new(addr_of!(v1[7]).cast_mut()).unwrap(), layout));

        let v2 = allocator_api2::vec![in &alloc; 0u8; 8];
        assert!(bump.contains(NonNull::new(addr_of!(v2[0]).cast_mut()).unwrap(), layout));
        assert!(bump.contains(NonNull::new(addr_of!(v2[7]).cast_mut()).unwrap(), layout));

        let v3 = allocator_api2::vec![in &alloc; 0u8; 9];
        assert!(!bump.contains(NonNull::new(addr_of!(v3[0]).cast_mut()).unwrap(), layout));
        assert!(!bump.contains(NonNull::new(addr_of!(v3[8]).cast_mut()).unwrap(), layout));
    }
}
