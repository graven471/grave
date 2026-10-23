#![allow(unused)]
use core::alloc::Layout;

use limine::memmap;

use crate::{kprintln, memory::map::MemoryMap};

pub struct PhysAddr(u64);

// temp
#[derive(Debug)]
pub struct AllocationError;

pub trait PhysicalAllocator {
    type Error;

    // hhdm_offset is temp
    fn init(
        memory_map: &MemoryMap,
        hhdm_offset: u64, /* later add some config etc */
    ) -> Result<Self, Self::Error>
    where
        Self: Sized;

    fn alloc(&mut self, layout: Layout) -> Result<PhysAddr, Self::Error>;
    fn free(&mut self, ptr: PhysAddr, layout: Layout);
}

const MIN_SIZE: usize = 4096;
// fixme: this is temp max order will be 18
const MAX_ORDER: usize = 18;

struct FreeBlock {
    next: *mut FreeBlock,
}

impl FreeBlock {
    pub fn new() -> Self {
        Self {
            next: core::ptr::null_mut(),
        }
    }

    pub unsafe fn push(head: *mut *mut FreeBlock, other: *mut FreeBlock) -> bool {
        unsafe {
            if (head.is_null() || other.is_null()) {
                return false;
            }

            // head -> null
            if (*head).is_null() {
                (*other).next = core::ptr::null_mut();
                *head = other;
                return true;
            }

            // head -> [x] -> null
            if (**head).next.is_null() {
                (**head).next = other;
                return true;
            }

            // general case
            (*other).next = (*head);
            (*head) = other;

            return true;
        }
    }

    pub unsafe fn pop(&self, head: *mut *mut FreeBlock) -> Option<*mut FreeBlock> {
        unsafe {
            if head.is_null() {
                return None;
            }

            // head -> null
            if (*head).is_null() {
                return None;
            }

            // head -> [x] -> [y] -> null
            // head -> [y] -> null

            let node_ptr = *head;
            *head = (*node_ptr).next;

            return Some(node_ptr);
        }
    }

    pub unsafe fn remove_by_address(
        &mut self,
        head: *mut *mut FreeBlock,
        addr: *const FreeBlock,
    ) -> Option<*mut FreeBlock> {
        unsafe {
            if head.is_null() {
                return None;
            }

            if (*head).is_null() {
                return None;
            }

            if core::ptr::eq(*head, addr) {
                return self.pop(head);
            }

            let temp = head;

            while !(*temp).is_null() {
                if core::ptr::eq((**temp).next, addr) {
                    let node_ptr = (**temp).next;
                    (**temp).next = (*node_ptr).next;
                    return Some(node_ptr);
                }
                (*temp) = (**temp).next;
            }
            return None;
        }
    }

    // todo: insert sorted
    pub fn insert_sorted(&self, head: *mut *mut FreeBlock, other: *mut FreeBlock) {
        todo!()
    }
}

pub struct BuddyAllocator {
    // pointer to the first node in the list
    blocks: [*mut FreeBlock; MAX_ORDER + 1],
}

impl BuddyAllocator {
    const fn order_to_size(&self, order: usize) -> usize {
        return MIN_SIZE << order;
    }

    const fn size_to_order(&self, size: usize) -> usize {
        return (size / MIN_SIZE).ilog2() as usize;
    }

    fn round_size(&self, layout: Layout) -> usize {
        return layout
            .size()
            .max(layout.align())
            .next_power_of_two()
            .max(MIN_SIZE);
    }
}

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

/*
 * our smallest block is 4 KiB so,
 * order 0 -> 4 KiB
 * order 1 -> 8 KiB
 * order 2 -> 16 KiB
 * order 3 -> 32 KiB
 * order 4 -> 64 Kib
 * ...
 */

impl PhysicalAllocator for BuddyAllocator {
    type Error = AllocationError;

    fn init(memory_map: &MemoryMap, hhdm_offset: u64) -> Result<Self, Self::Error>
    where
        Self: Sized,
    {
        let mut buddy = Self {
            blocks: [core::ptr::null_mut(); MAX_ORDER + 1],
        };

        for entry in memory_map.usable() {
            let start = entry.base;
            let end = entry.base + entry.length;

            // the address where the unprocessed memory begins
            let mut addr = (start + 4095) & !4095;

            // how much bytes of the region we still have to process
            let mut remaining = end.saturating_sub(addr) & !4095;

            while remaining >= MIN_SIZE as u64 {
                // largest power of two block that fits inside remaining
                let size_limit = 1u64 << remaining.ilog2();
                // largest power of two block that can start at addr without
                // violating alignment
                let alignment_limit = if addr == 0 {
                    u64::MAX
                } else {
                    1u64 << addr.trailing_zeros()
                };

                let mut order = buddy.size_to_order(remaining as usize);
                order = order.min(MAX_ORDER);
                while order > 0 && ((MIN_SIZE as u64) << order) > alignment_limit {
                    order -= 1;
                }

                let block_size = (MIN_SIZE as u64) << order;

                kprintln!(
                    "order = {:#x}, block_size = {:#x}, align_limit = {:#x}, addr = {:#x}, remaining = {:#x}",
                    order,
                    block_size,
                    alignment_limit,
                    addr,
                    remaining
                );

                let block = (addr + hhdm_offset) as *mut FreeBlock;

                unsafe {
                    // buddy.blocks[order] -> [block] -> [block] -> [block] -> null
                    if !FreeBlock::push(&mut buddy.blocks[order], block) {
                        return Err(AllocationError);
                    }
                }

                addr = addr.wrapping_add(block_size);
                remaining = remaining.wrapping_sub(block_size);
            }
        }
        Ok(buddy)
    }

    fn alloc(&mut self, layout: Layout) -> Result<PhysAddr, Self::Error> {
        let size = self.round_size(layout);
        let order = self.size_to_order(size);

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

        // round the request size to next power of two
        // look for that size block in free list
        // if yes then allocate the block and return
        // otherwise pick a bigger block split it using formula n = log2(block/requested_size)
        // store the right side in free list and keep splitting the left side
        //
        // order = log2(size/MIN_SIZE)
        // size = MIN_SIZE * 2^order
        todo!()
    }

    fn free(&mut self, ptr: PhysAddr, layout: Layout) {
        todo!()
    }
}
