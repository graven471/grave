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
        physical::{PhysicalAllocator, free_list::FreeBlock},
    },
};

#[cfg(target_arch = "x86_64")]
use crate::x86_64::utils::prefetch_read_t0;

use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuddyError {
    /// No suitable free block exists for the requested allocation.
    OutOfMemory,

    /// The requested layout has an invalid size or alignment.
    InvalidLayout,

    /// The requested allocation exceeds the allocator's maximum order.
    OrderTooLarge,

    /// The supplied physical address is null, unmapped, or invalid.
    InvalidAddress,

    /// The address does not correspond to a valid allocated block.
    InvalidFree,

    /// The corresponding buddy block is missing from the free list.
    CorruptedFreeList,

    /// The memory map contains no usable region large enough to initialize.
    NoUsableMemory,

    /// Address arithmetic overflowed while calculating a block boundary.
    AddressOverflow,
}

impl fmt::Display for BuddyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::OutOfMemory => "buddy allocator: out of memory",
            Self::InvalidLayout => "buddy allocator: invalid allocation layout",
            Self::OrderTooLarge => "buddy allocator: allocation exceeds maximum order",
            Self::InvalidAddress => "buddy allocator: invalid physical address",
            Self::InvalidFree => "buddy allocator: invalid or double free",
            Self::CorruptedFreeList => "buddy allocator: free-list invariant violated",
            Self::NoUsableMemory => "buddy allocator: no usable memory available",
            Self::AddressOverflow => "buddy allocator: address arithmetic overflow",
        })
    }
}

impl core::error::Error for BuddyError {}

pub struct Bitmap {
    value: u16,
}

impl Bitmap {
    #[inline(always)]
    pub const fn new() -> Self {
        Self { value: 0 }
    }

    #[inline(always)]
    pub const fn value(&self) -> u16 {
        self.value
    }

    #[inline(always)]
    pub const fn set_bit(&mut self, index: u16) {
        self.value |= 1u16 << index;
    }

    #[inline(always)]
    pub const fn clear_bit(&mut self, index: u16) {
        self.value &= !(1u16 << index);
    }

    #[inline(always)]
    pub const fn check_bit(&self, index: u16) -> bool {
        (self.value & (1u16 << index)) != 0
    }

    #[inline(always)]
    pub const fn lowest_set_bit(&self) -> u32 {
        self.value.trailing_zeros()
    }
}

const MIN_SIZE: usize = 4096;
const MAX_ORDER: usize = 11;

pub struct BuddyAllocator {
    blocks: [*mut FreeBlock; MAX_ORDER + 1],
    bitmap: Bitmap,
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

impl PhysicalAllocator for BuddyAllocator {
    type Error = BuddyError;

    fn init(memory_map: &MemoryMap) -> Result<Self, Self::Error>
    where
        Self: Sized,
    {
        let mut buddy = Self {
            blocks: [core::ptr::null_mut(); MAX_ORDER + 1],
            bitmap: Bitmap::new(),
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
                let phys = PhysAddr::new(addr).ok_or(BuddyError::InvalidAddress)?;

                let block = phys
                    .translate()
                    .ok_or(BuddyError::InvalidAddress)?
                    .as_mut_ptr::<FreeBlock>();

                unsafe {
                    // buddy.blocks[order] -> [block] -> [block] -> [block] -> null
                    if unlikely(!FreeBlock::push(&mut buddy.blocks[order], block)) {
                        return Err(BuddyError::CorruptedFreeList);
                    }
                }

                buddy.bitmap.set_bit(order as u16);

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

        if likely(!self.blocks[requested_order].is_null()) {
            let block = unsafe { FreeBlock::pop(&raw mut self.blocks[requested_order]) };
            debug_assert!(!block.is_null(), "pop block is null");
            let virt = unsafe { VirtAddr::new_unchecked(block as u64) };
            let phys_addr = virt.as_64() - hhdm::offset();
            debug_assert!(
                PhysAddr::new(phys_addr).is_some(),
                "invalid physical address derived from buddy free-list node"
            );
            // todo: check address invariant
            return unsafe { Ok(PhysAddr::new_unchecked(phys_addr)) };
        }

        let mut actual_order: usize = usize::MAX;
        for order in (requested_order)..=MAX_ORDER {
            if likely(!self.blocks[order].is_null()) {
                actual_order = order;
                break;
            }
        }
        if unlikely(actual_order == usize::MAX) {
            return Err(BuddyError::NoUsableMemory);
        }

        let mut splits_remaining = actual_order - requested_order;
        size = self.order_to_size(actual_order);
        let mut block: *mut FreeBlock = core::ptr::null_mut();

        unsafe {
            #[cfg(target_arch = "x86_64")]
            prefetch_read_t0(self.blocks[actual_order].cast());
        }

        while likely(splits_remaining > 0) {
            debug_assert_eq!(size, self.order_to_size(actual_order));
            debug_assert!(actual_order > requested_order);

            let sh = (size >> 1) as u64;
            let lower_order = actual_order - 1;

            block = unsafe { FreeBlock::pop(&mut self.blocks[actual_order]) };
            debug_assert!(!block.is_null());

            let block_addr = block as u64;
            let right_start = block_addr + sh;

            if unlikely(!unsafe {
                FreeBlock::push(&mut self.blocks[lower_order], right_start as *mut FreeBlock)
            }) {
                return Err(BuddyError::CorruptedFreeList);
            }

            actual_order = lower_order;
            size = sh as usize;
            splits_remaining -= 1;
        }

        let virt = VirtAddr::new(block as u64).ok_or(BuddyError::InvalidAddress)?;
        return virt.translate().ok_or(BuddyError::InvalidAddress);
    }

    #[inline(always)]
    fn free(&mut self, ptr: PhysAddr, layout: Layout) -> Result<(), Self::Error> {
        debug_assert!(!ptr.as_ptr::<u64>().is_null(), "free ptr is null");

        let size = self.round_size(layout);
        let mut order = self.size_to_order(size);
        let mut curr_addr = ptr.as_64();

        loop {
            if order == MAX_ORDER {
                break;
            }

            let block_size = self.order_to_size(order) as u64;
            let buddy_addr = curr_addr ^ block_size;

            let buddy_virt: VirtAddr = unsafe {
                PhysAddr::new_unchecked(buddy_addr)
                    .translate()
                    .ok_or(BuddyError::InvalidAddress)?
            };

            let removed = unsafe {
                FreeBlock::remove_by_address(
                    &mut self.blocks[order],
                    buddy_virt.as_mut_ptr::<FreeBlock>(),
                )
            };

            if removed.is_null() {
                break;
            }

            // Both blocks are now represented by their parent block.
            curr_addr = curr_addr.min(buddy_addr);
            order += 1;
        }

        let curr_virt = unsafe {
            PhysAddr::new_unchecked(curr_addr)
                .translate()
                .ok_or(BuddyError::InvalidAddress)?
        };

        unsafe {
            if !FreeBlock::push(&mut self.blocks[order], curr_virt.as_mut_ptr::<FreeBlock>()) {
                return Err(BuddyError::CorruptedFreeList);
            }
        }
        return Ok(());
    }
}
