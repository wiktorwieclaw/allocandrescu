use allocandrescu::{alloc::Stack, prelude::*, AwareAllocator};
use allocator_api2::boxed::Box;
use std::{
    alloc::{Layout, System},
    ptr::{addr_of, NonNull},
};

fn main() {
    let alloc = Stack::<1024>::new().fallback(System);
    let layout = Layout::new::<u8>();

    let v = allocator_api2::vec![in &alloc; 0u8; 1024];
    let ptr = NonNull::new(v.as_ptr().cast_mut()).unwrap();
    assert!(alloc.primary().owns(ptr, layout));
    assert!(alloc.primary().owns(unsafe { ptr.add(1023) }, layout));

    let b = Box::new_in(0, &alloc);
    let ptr = NonNull::new(addr_of!(*b).cast_mut()).unwrap();
    assert!(!alloc.primary().owns(ptr, layout));
}
