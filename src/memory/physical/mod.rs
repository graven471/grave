pub mod buddy;
pub mod buddy_test;
pub mod free_list;

use core::alloc::Layout;

use crate::memory::{address::PhysAddr, map::MemoryMap};

pub trait PhysicalAllocator {
    type Error;

    fn init(memory_map: &MemoryMap) -> Result<Self, Self::Error>
    where
        Self: Sized;

    fn alloc(&mut self, layout: Layout) -> Result<PhysAddr, Self::Error>;
    fn free(&mut self, ptr: PhysAddr, layout: Layout) -> Result<(), Self::Error>;
}
