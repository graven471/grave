use core::alloc::Layout;

use crate::{
    kprintln,
    memory::{
        address::{TranslateAddr, VirtAddr},
        buddy::{BuddyAllocator, PhysicalAllocator},
        map::MemoryMap,
    },
};

pub fn test_buddy_allocator_init(memory_map: &MemoryMap) {
    kprintln!("entered test_buddy_allocator_init");
    let mut allocator = BuddyAllocator::init(memory_map);
    kprintln!("result is : {:?}", allocator.as_mut().err());
    assert!(allocator.is_ok());

    let mut allocator = allocator.unwrap();
    // Consume a block from the smallest order you want to test.
    let small = allocator.alloc(Layout::from_size_align(4, 4).unwrap());
    assert!(small.is_ok());

    // Request a larger block, which should require splitting.
    let large = allocator.alloc(Layout::from_size_align(1024, 1024).unwrap());
    assert!(large.is_ok());

    kprintln!("small allocation: {:?}", small.unwrap().as_64());
    kprintln!("large allocation: {:?}", large.unwrap().as_64());
}
