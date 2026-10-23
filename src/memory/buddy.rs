#![allow(unused)]
use core::{alloc::Layout, ptr::NonNull};

use crate::{kprintln, memory::map::MemoryMap};

pub struct PhysAddr(u64);

// temp
pub struct AllocationError;

pub trait PhysicalAllocator {
    type Error;

    fn init(memory_map: &MemoryMap, /* later add some config etc */) -> Result<Self, Self::Error>
    where
        Self: Sized;

    fn alloc(&mut self, layout: Layout) -> Result<PhysAddr, Self::Error>;
    fn free(&mut self, ptr: PhysAddr, layout: Layout);
}

pub struct BuddyAllocator {/* stuffs */}

/*
* core rules of buddy
* S(k) = 2^k          (block size)
* B(i, k) = i XOR 2^k (buddy index) where, i = block index, k = order
* P(i, k) = i & ~2^k  (parent index)
*
*  General splitting formula for any S > 1:
 let A = [A + A + S), let S = l
 S_half = S/2
 A_left = [A, A + S_half)
 A_right = [A + S_half, A + S)

 * If the available block has size S and the requested block size is R, repeatedly halve S
 * until it equals R.

  n = log2(S/R)

  example:
  here caller requested 32 bytes, and available block is 256
  n = log2(256/32) = log2(8) = 3, (order = 3)

  we keep splitting until n = 0

  n = 3
  splits 1 -> block size 128
  n = 2
  splits 2 -> block size 64
  n = 1
  splits 3 -> block size 32

*/

impl PhysicalAllocator for BuddyAllocator {
    type Error = AllocationError;

    fn init(memory_map: &MemoryMap, /* later add some config etc */) -> Result<Self, Self::Error>
    where
        Self: Sized,
    {
        for region in memory_map.usable() {}
        todo!()
    }

    fn alloc(&mut self, layout: Layout) -> Result<PhysAddr, Self::Error> {
        /*
        * initially i have a big blob of usable memory
        *  ___________________________________________
        * |               0x100 - 0x1FF              |
        * |                 256 bytes                |
        * |__________________________________________|
        *
        * i want 64 bytes alloc(64)
        *
        * split out big blob into two equal pieces
        * BLOCK A                          BLOCK B
        *  ____________________           ___________________
        * |     0x100-0x17F   |           |   0x180-0x1FF   |
        * |     128-bytes     |           |   128 bytes     |
        * |___________________|           |_________________|

        *    ____________________           ___________________
          * |     0x100-0x12F   |           |   0x140-0x17F   |
          * |      64 bytes     |           |    64 bytes     |
          * |___________________|           |_________________|
          *
          * return the first block to user
          * we have 0x100-0x12F -> allocated (64 B)
          * 0x140 -> free (64 B)
          * 0x180 -> free (128 B)
        */
        todo!()
    }

    fn free(&mut self, ptr: PhysAddr, layout: Layout) {
        todo!()
    }
}
