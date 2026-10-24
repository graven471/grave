use core::{
    alloc::Layout,
    hint::{likely, unlikely},
};

use crate::{
    kprintln,
    memory::{
        address::{PhysAddr, TranslateAddr, VirtAddr},
        hhdm,
        map::MemoryMap,
    },
};

// temp
#[derive(Debug)]
pub struct AllocationError;

pub trait PhysicalAllocator {
    type Error;

    // hhdm_offset is temp
    fn init(memory_map: &MemoryMap, /* later add some config etc */) -> Result<Self, Self::Error>
    where
        Self: Sized;

    fn alloc(&mut self, layout: Layout) -> Result<PhysAddr, Self::Error>;
    fn free(&mut self, ptr: PhysAddr, layout: Layout) -> Result<(), Self::Error>;
}

const MIN_SIZE: usize = 4096;
const MAX_ORDER: usize = 11;

struct FreeBlock {
    next: *mut FreeBlock,
}

impl FreeBlock {
    #[allow(unused)]
    pub const fn new() -> Self {
        Self {
            next: core::ptr::null_mut(),
        }
    }

    pub unsafe fn push(head: *mut *mut FreeBlock, other: *mut FreeBlock) -> bool {
        if head.is_null() || other.is_null() {
            return false;
        }
        unsafe {
            (*other).next = *head;
            *head = other;
        }
        return true;
    }

    pub unsafe fn pop(head: *mut *mut FreeBlock) -> Option<*mut FreeBlock> {
        unsafe {
            if head.is_null() {
                return None;
            }

            let node = *head;

            if node.is_null() {
                return None;
            }

            if (node as usize) % core::mem::align_of::<FreeBlock>() != 0 {
                kprintln!("CORRUPT FREE LIST: node_ptr={:#x}", node as usize);
                return None;
            }

            *head = (*node).next;
            (*node).next = core::ptr::null_mut();

            return Some(node);
        }
    }

    #[allow(unused)]
    pub unsafe fn remove_by_address(
        head: *mut *mut FreeBlock,
        addr: *const FreeBlock,
    ) -> Option<*mut FreeBlock> {
        unsafe {
            if head.is_null() || addr.is_null() {
                return None;
            }

            if (*head).is_null() {
                return None;
            }

            if core::ptr::eq(*head, addr) {
                return Self::pop(head);
            }

            let temp = head;

            while !(*temp).is_null() {
                if core::ptr::eq((**temp).next, addr) {
                    let node_ptr = (**temp).next;
                    (**temp).next = (*node_ptr).next;
                    return Some(node_ptr);
                }
                *temp = (**temp).next;
            }
            return None;
        }
    }

    // todo: insert sorted
    // pub fn insert_sorted(&self, head: *mut *mut FreeBlock, other: *mut FreeBlock) {
    //     todo!()
    // }
}

pub struct BuddyAllocator {
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

impl PhysicalAllocator for BuddyAllocator {
    type Error = AllocationError;

    fn init(memory_map: &MemoryMap) -> Result<Self, Self::Error>
    where
        Self: Sized,
    {
        let mut buddy = Self {
            blocks: [core::ptr::null_mut(); MAX_ORDER + 1],
        };

        for entry in memory_map.usable() {
            let start = entry.base;
            let end = entry.base + entry.length;
            let min_size = (MIN_SIZE - 1) as u64;

            // the address where the unprocessed memory begins
            let mut addr = (start + min_size) & !min_size;
            // how much bytes of the region we still have to process
            let mut remaining = end.saturating_sub(addr) & !min_size;

            // we have three constraints:
            // 1. block must fit inside remaining.
            // 2. block start address must be aligned to size.
            // 3. maximum order the block must fit in the free-list array.
            while remaining >= MIN_SIZE as u64 {
                // c2: alignment it must be aligned
                let alignment_limit = if unlikely(addr == 0) {
                    u64::MAX
                } else {
                    1u64 << addr.trailing_zeros()
                };

                // c3: order must not be bigger than MAX_ORDER
                let mut order = buddy.size_to_order(remaining as usize);
                order = order.min(MAX_ORDER);
                while likely(order > 0) && (buddy.order_to_size(order) as u64) > alignment_limit {
                    order -= 1;
                }

                // block_size(o) = MIN_SIZE * 2^o
                let block_size = buddy.order_to_size(order) as u64;

                let phys = PhysAddr::new(addr).ok_or(AllocationError)?;

                let block = phys
                    .translate()
                    .ok_or(AllocationError)?
                    .as_mut_ptr::<FreeBlock>();

                unsafe {
                    // buddy.blocks[order] -> [block] -> [block] -> [block] -> null
                    if unlikely(!FreeBlock::push(&mut buddy.blocks[order], block)) {
                        return Err(AllocationError);
                    }
                }

                addr += block_size;
                remaining -= block_size;
            }
        }
        Ok(buddy)
    }

    fn alloc(&mut self, layout: Layout) -> Result<PhysAddr, Self::Error> {
        let mut size = self.round_size(layout);
        let requested_order = self.size_to_order(size);

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
        */

        let mut actual_order: Option<usize> = None;

        for order in (requested_order)..=MAX_ORDER {
            if !self.blocks[order].is_null() {
                actual_order = Some(order);
                break;
            }
        }

        let Some(mut actual_order) = actual_order else {
            return Err(AllocationError);
        };

        size = self.order_to_size(actual_order);
        let mut splits_remaining = actual_order - requested_order;

        // fast path
        if splits_remaining == 0 {
            let block = unsafe { FreeBlock::pop(&raw mut self.blocks[requested_order]) }
                .filter(|b| !b.is_null())
                .ok_or(AllocationError)?;
            let virt = VirtAddr::new(block as u64).ok_or(AllocationError)?;
            kprintln!("allocated address = {:#x}", (block as u64) - hhdm::offset());
            return virt.translate().ok_or(AllocationError);
        }

        let mut block: *mut FreeBlock = core::ptr::null_mut();

        while splits_remaining > 0 {
            debug_assert_eq!(size, self.order_to_size(actual_order));
            debug_assert!(actual_order > requested_order);

            // let A = [A + A + S), let S = l
            // S_half = S/2
            // A_left = [A, A + S_half)
            // A_right = [A + S_half, A + S)
            let sh = (size >> 1) as u64;

            let lower_order = actual_order - 1;

            block = unsafe { FreeBlock::pop(&mut self.blocks[actual_order]) }
                .filter(|b| !b.is_null())
                .ok_or(AllocationError)?;

            let block_addr = block as u64;

            let left_start = block_addr;
            let left_end = block_addr + sh;
            let right_start = block_addr + sh;
            let right_end = block_addr + (size as u64);

            kprintln!(
                "req_order = {}, req_size: {}, actual_size: {}, actual_order = {}, lower_order = {}, left_start = {:#x}, left_end = {:#x}, right_start = {:#x}, right_end = {:#x}, left_block: {:#x}",
                requested_order,
                layout.size(),
                size,
                actual_order,
                lower_order,
                left_start,
                left_end,
                right_start,
                right_end,
                block_addr
            );

            unsafe {
                if !FreeBlock::push(&mut self.blocks[lower_order], right_start as *mut FreeBlock) {
                    return Err(AllocationError);
                }
            }

            actual_order = lower_order;
            size = sh as usize;
            splits_remaining -= 1;
        }
        kprintln!("allocated address = {:#x}", (block as u64) - hhdm::offset());
        let virt = VirtAddr::new(block as u64).ok_or(AllocationError)?;
        return virt.translate().ok_or(AllocationError);
    }

    fn free(&mut self, ptr: PhysAddr, layout: Layout) -> Result<(), Self::Error> {
        if ptr.as_ptr::<u64>().is_null() {
            return Err(AllocationError);
        }

        let size = self.round_size(layout);
        let mut order = self.size_to_order(size);
        let mut curr_block_addr: u64;

        loop {
            let block_size = self.order_to_size(order) as u64;
            let buddy_addr_phys = ptr.as_64() ^ block_size;
            let buddy_addr_virt = ptr.translate().ok_or(AllocationError)?;

            let block = unsafe {
                FreeBlock::remove_by_address(
                    &mut self.blocks[order],
                    buddy_addr_virt.as_mut_ptr::<FreeBlock>(),
                )
            };

            let Some(_) = block else {
                unsafe {
                    FreeBlock::push(
                        &mut self.blocks[order],
                        ptr.translate()
                            .ok_or(AllocationError)?
                            .as_mut_ptr::<FreeBlock>(),
                    );
                };
                return Ok(());
            };

            curr_block_addr = ptr.as_64().min(buddy_addr_phys);

            if order == MAX_ORDER {
                break;
            }

            order += 1;
        }

        unsafe {
            FreeBlock::push(
                &mut self.blocks[order],
                PhysAddr::new_unchecked(curr_block_addr)
                    .translate()
                    .ok_or(AllocationError)?
                    .as_mut_ptr::<FreeBlock>(),
            );
        }

        return Ok(());
    }
}
