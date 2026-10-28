/*
* core idea
* let P = 4096
* let B = 0x1000 (base address buddy will give us)
* let A = 4 (alignment)
* let R = 32 (object size)
* S = align_up(R, A)
* N = 4096/32 = 128 slots can fit in this page
* [0, 1, 2, 3, 4, 5, 6, ......... 127]
* size of each slot is ai = B + i*s where i is the index and i < N
* alignment test = ai mod A == 0
* our free list will look like this
* Slab[32] -> 0x1000 -> 0x1020 -> 0x1040 -> 0x1080 ....... -> NULL
* during init
* for i in 0..N
*    addr = B + i*s
*    free_list.push(addr)
*
* states are:
* Empty -> in_use == 0 (all slots are free)
* Partial -> 0 < in_use < object_count
* Full -> in_use == object_size
*
* slab blocks are:
* 16 bytes
* 32 bytes
* 64 bytes
* 128 bytes
*
* if size > 128 bytes then use buddy directly
*
* CACHE the key idea:
* slab is the top level it looks like this
* it needs SLAB_HEAD so it can travese it
* CACHE<64> -> SLAB_A(page = 0x1000) -> SLAB_B(page = 0x2000) -> SLAB_C(page = 0x3000) -> NULL
* note: there is two next one is CACHE next which links SLAB
* and for each SLAB there is a next which points to the free block
* conceptually:
* SLAB_A -> 0x1000 -> 0x1020 -> 0x1040 -> NULL
* SLAB_B -> 0x1060 -> 0x1080 -> 0x2000 -> NULL
*
* Slab metadata will be embedded initially
* page.as_64()
    |
    v
+-------------------------+
| Slab<N> metadata        |
+-------------------------+
| Alignment padding       |
+-------------------------+ <-- start (slot 0)
| Slot 0        (N bytes) |
+-------------------------+ <-- start + N
| Slot 1        (N bytes) |
+-------------------------+ <-- start + 2*N
| Slot 2        (N bytes) |
+-------------------------+
| ...                     |
+-------------------------+


kmem_cache = cache metadata
slab = one memory pool for that cache
freelist = list of free objects inside a slab

struct kmem_cache does not hold a direct single next_slab pointer in this header.
Instead, slabs are kept in lists managed by the allocator.
each slab has:
struct kmem_cache *slab_cache
struct list_head slab_list
*/

use crate::memory::address::TranslateAddr;
use core::alloc::Layout;
use core::hint::{likely, unlikely};

use crate::{
    collections::intrusive_list::ListHead,
    list_entry, list_for_each, list_for_each_entry, list_head_ptr,
    memory::{
        address::{PhysAddr, VirtAddr},
        physical::{PhysicalAllocator, free_list::FreeBlock},
    },
};

#[inline(always)]
pub const fn align_up(value: usize, align: usize) -> usize {
    (value + align - 1) & !(align - 1)
}

#[repr(u8)]
#[allow(unused)]
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
enum State {
    Empty,
    Partial,
    Full,
}

struct Slab<const N: usize> {
    page: PhysAddr,
    next: *mut FreeBlock,
    slab_list: ListHead,
    free_count: usize,
    in_use: usize,
    object_count: usize,
    state: State,
}

impl<const N: usize> Slab<N> {
    pub const fn get_state(&self) -> State {
        match self.in_use {
            0 => State::Empty,
            n if likely(n > 0 && n < self.object_count) => State::Partial,
            n if n == self.object_count => State::Full,
            _ => unreachable!(),
        }
    }

    #[inline]
    pub fn update_state(&mut self) {
        self.state = self.get_state();
    }

    pub fn take_block(&mut self) -> *mut FreeBlock {
        if unlikely(self.state == State::Full) {
            return core::ptr::null_mut();
        }

        let block = unsafe { FreeBlock::pop(&mut self.next) };
        debug_assert!(!block.is_null(), "addr is null");
        debug_assert!(block.is_aligned(), "addr is not aligned");
        self.free_count -= 1;
        self.in_use += 1;
        self.update_state();
        return block;
    }
}

pub struct KMemCache<const N: usize> {
    pub list: ListHead, // list of slab caches
}

// temp
#[derive(Debug)]
pub struct SlabError;

const PAGE_SIZE: usize = 4096;

impl<const N: usize> KMemCache<N> {
    const ASSERT_POW2: () = assert!(N.is_power_of_two(), "N must needs to be power of two");
    const HEADER: usize = align_up(size_of::<Slab<N>>(), N);
    const SLOTS: usize = (PAGE_SIZE - Self::HEADER) >> N.trailing_zeros();

    pub fn new() -> Self {
        let _ = Self::ASSERT_POW2;
        let mut cache = Self {
            list: ListHead::new(),
        };
        unsafe {
            ListHead::init(&mut cache.list as *mut _);
        }
        return cache;
    }

    #[inline(always)]
    pub const fn slots(&self, offset: usize) -> usize {
        (PAGE_SIZE - offset) >> N.trailing_zeros()
    }

    #[inline]
    unsafe fn add_slab(&mut self, slab: *mut Slab<N>) {
        (*slab).slab_list.add(&mut self.list);
    }

    #[allow(unused)]
    #[inline]
    unsafe fn add_slab_tail(&mut self, slab: *mut Slab<N>) {
        (*slab).slab_list.add_tail(&mut self.list);
    }

    unsafe fn alloc_slab<T>(&mut self, phys_allocator: &mut T) -> Result<VirtAddr, SlabError>
    where
        T: PhysicalAllocator,
    {
        let layout = Layout::from_size_align_unchecked(N, N);
        let page = phys_allocator.alloc(layout).map_err(|_| SlabError)?;
        let virt = page.translate().ok_or(SlabError)?;

        // skip Slab metadata and align up with respect to N
        // let header_size = size_of::<Slab<N>>();
        // let start = virt.as_64() + header_size as u64;
        // let start = align_up(start as usize, N) as u64;
        // let offset = start - virt.as_64();
        // let slots = self.slots(offset as usize);

        let start = virt.as_64() + Self::HEADER as u64;
        let slots = Self::SLOTS;

        kprintln!("slots = {}, start = {:#x}", slots, start,);

        let slab: Slab<N> = Slab {
            free_count: slots,
            next: core::ptr::null_mut(),
            page,
            slab_list: ListHead::new(),
            in_use: 0,
            object_count: slots,
            state: State::Empty,
        };
        core::ptr::write(virt.as_mut_ptr(), slab);

        let slab_ptr = virt.as_mut_ptr::<Slab<N>>();
        debug_assert!(!slab_ptr.is_null(), "slab ptr is null");

        ListHead::init(&mut (*slab_ptr).slab_list as *mut _);

        for i in (0..slots).rev() {
            // Pi = P + iQ, where P is the base and Q is the block size
            let slot_addr = start + (i * N) as u64;
            if !FreeBlock::push(&mut (*slab_ptr).next, slot_addr as *mut FreeBlock) {
                return Err(SlabError);
            }
        }
        // link new slab
        self.add_slab(slab_ptr);

        // todo: replace with error handling
        let addr = (*slab_ptr).take_block();
        debug_assert!(!addr.is_null(), "take block returns null state is full");

        return Ok(VirtAddr::new_unchecked(addr as u64));
    }

    pub fn alloc<T>(&mut self, phys_allocator: &mut T) -> Result<VirtAddr, SlabError>
    where
        T: PhysicalAllocator,
    {
        // (todo): refill slab if its empty
        // (todo): handle partial case
        list_for_each_entry!(slab, self.list, Slab<N>, slab_list, {
            if slab.get_state() == State::Empty {
                continue;
            }
            return unsafe { Ok(VirtAddr::new_unchecked((*slab).take_block() as u64)) };
        });
        return unsafe { self.alloc_slab(phys_allocator) };
    }
}
