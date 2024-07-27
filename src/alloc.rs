//! Basic allocators.

use crate::AwareAllocator;
use allocator_api2::alloc::{AllocError, Allocator};
use core::{
    alloc::Layout,
    cell::{Cell, UnsafeCell},
    ptr::{self, NonNull},
};

/// Failing Allocator.
pub struct Failing;

unsafe impl Allocator for Failing {
    fn allocate(&self, _layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        Err(AllocError)
    }

    unsafe fn deallocate(&self, _ptr: NonNull<u8>, _layout: Layout) {}
}

impl AwareAllocator for Failing {
    fn owns(&self, _ptr: NonNull<u8>, _layout: Layout) -> bool {
        false
    }
}

/// Stack allocator.
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
impl<const SIZE: usize> AwareAllocator for Stack<SIZE> {
    fn owns(&self, ptr: NonNull<u8>, layout: Layout) -> bool {
        let stack_start = self.stack.get() as usize;
        let stack_end = stack_start.saturating_add(SIZE);
        let alloc_start = as_usize(ptr);
        let alloc_end = alloc_start.saturating_add(layout.size());
        stack_start <= alloc_start && stack_end >= alloc_end
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
}
