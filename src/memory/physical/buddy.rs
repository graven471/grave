use core::{
    alloc::Layout,
    hint::{likely, unlikely},
};

use crate::memory::{
    address::{PhysAddr, TranslateAddr, VirtAddr},
    map::MemoryMap,
    physical::{PhysicalAllocator, free_list::FreeBlock},
};

#[cfg(target_arch = "x86_64")]
use crate::x86_64::utils::prefetch_read_t0;

// temp
#[derive(Debug)]
pub struct AllocationError;

const MIN_SIZE: usize = 4096;
const MAX_ORDER: usize = 11;

pub struct BuddyAllocator {
    blocks: [*mut FreeBlock; MAX_ORDER + 1],
}

impl BuddyAllocator {
    #[inline(always)]
    const fn order_to_size(&self, order: usize) -> usize {
        return MIN_SIZE << order;
    }

    #[inline(always)]
    const fn size_to_order(&self, size: usize) -> usize {
        return size.ilog2() as usize - 12;
    }

    #[inline(always)]
    fn round_size(&self, layout: Layout) -> usize {
        return layout
            .size()
            .max(layout.align())
            .next_power_of_two()
            .max(MIN_SIZE);
    }

    #[inline(always)]
    #[allow(unused)]
    fn is_order_empty(&self, order: usize) -> bool {
        self.blocks[order].is_null()
    }
}

/*
 * core rules of buddy
 * S(k) = 2^k          (block size)
 * B(i, k) = i XOR 2^k (buddy index) where, i = block index, k = order
 * P(i, k) = i & ~2^k  (parent index)
 *
 */
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

    #[inline(always)]
    fn alloc(&mut self, layout: Layout) -> Result<PhysAddr, Self::Error> {
        let mut size = self.round_size(layout);
        let requested_order = self.size_to_order(size);
        let mut actual_order: usize = usize::MAX;

        for order in (requested_order)..=MAX_ORDER {
            if likely(!self.blocks[order].is_null()) {
                actual_order = order;
                break;
            }
        }

        if unlikely(actual_order == usize::MAX) {
            return Err(AllocationError);
        }

        let mut splits_remaining = actual_order - requested_order;
        if likely(splits_remaining == 0) {
            let block = unsafe { FreeBlock::pop(&raw mut self.blocks[requested_order]) }
                .ok_or(AllocationError)?;

            let virt = VirtAddr::new(block as u64).ok_or(AllocationError)?;
            // kprintln!("allocated address = {:#x}", (block as u64) - hhdm::offset());
            return virt.translate().ok_or(AllocationError);
        }

        size = self.order_to_size(actual_order);
        let mut block: *mut FreeBlock = core::ptr::null_mut();

        unsafe {
            #[cfg(target_arch = "x86_64")]
            prefetch_read_t0(self.blocks[actual_order].cast());
        }

        while likely(splits_remaining > 0) {
            debug_assert_eq!(size, self.order_to_size(actual_order));
            debug_assert!(actual_order > requested_order);

            // let A = [A + A + S), let S = l
            // S_half = S/2
            // A_left = [A, A + S_half)
            // A_right = [A + S_half, A + S)
            let sh = (size >> 1) as u64;
            let lower_order = actual_order - 1;

            block =
                unsafe { FreeBlock::pop(&mut self.blocks[actual_order]) }.ok_or(AllocationError)?;

            let block_addr = block as u64;
            let right_start = block_addr + sh;

            unsafe {
                if unlikely(!FreeBlock::push(
                    &mut self.blocks[lower_order],
                    right_start as *mut FreeBlock,
                )) {
                    return Err(AllocationError);
                }
            }

            actual_order = lower_order;
            size = sh as usize;
            splits_remaining -= 1;
        }
        // kprintln!("allocated address = {:#x}", (block as u64) - hhdm::offset());
        let virt = VirtAddr::new(block as u64).ok_or(AllocationError)?;
        return virt.translate().ok_or(AllocationError);
    }

    #[inline(always)]
    fn free(&mut self, ptr: PhysAddr, layout: Layout) -> Result<(), Self::Error> {
        if unlikely(ptr.as_ptr::<u64>().is_null()) {
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
