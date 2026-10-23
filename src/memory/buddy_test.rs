use crate::{
    kprintln,
    memory::{
        buddy::{BuddyAllocator, PhysicalAllocator},
        map::MemoryMap,
    },
};

pub fn test_buddy_allocator_init(memory_map: &MemoryMap, hddm_off: u64) {
    kprintln!("entered test_buddy_allocator_init");
    let mut allocator = BuddyAllocator::init(memory_map, hddm_off);
    kprintln!("result is : {:?}", allocator.as_mut().err());
    assert!(allocator.is_ok());
}
