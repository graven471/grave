#![allow(unused)]
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
* slub[32] -> 0x1000 -> 0x1020 -> 0x1040 -> 0x1080 ....... -> NULL
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
* SLUB metadata will be embedded initially
* page.as_64()
    |
    v
+-------------------------+
| Slub<N> metadata        |
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

use core::alloc::Layout;

use crate::memory::{
    address::{PhysAddr, VirtAddr},
    physical::{PhysicalAllocator, buddy::BuddyAllocator, free_list::FreeBlock},
};

#[inline(always)]
pub const fn align_up(value: usize, align: usize) -> usize {
    (value + align - 1) & !(align - 1)
}

#[repr(u8)]
enum State {
    Empty,
    Partial,
    Full,
}

struct Slub<const N: usize> {
    // one contigious page
    page: PhysAddr,
    // blocks linked together
    // next -> [] -> [] -> [] -> [] -> null
    next: *mut FreeBlock,
    free_count: usize,
    state: State,
}

struct KMemCache<const N: usize> {
    // Head<N> -> Slub<N> -> Slub<N> -> Slub<N> -> NULL
    head: *mut Slub<N>,
}

// temp
pub struct SlubError;

const PAGE_SIZE: usize = 4096;

impl<const N: usize> KMemCache<N> {
    const ASSERT_POW2: () = assert!(N.is_power_of_two(), "N must needs to be power of two");
    // how many slots of N can we pack in a page
    const SLOTS: u64 = (PAGE_SIZE >> N.ilog2()) as u64;
    // todo: remove buddy from arg either make it global or anything
    pub fn new() -> Self {
        let _ = Self::ASSERT_POW2;
        Self {
            head: core::ptr::null_mut(),
        }
    }

    #[inline(always)]
    pub const fn slots(&self, offset: usize) -> usize {
        (PAGE_SIZE - offset) >> N.trailing_zeros()
    }

    pub fn alloc<T>(&mut self, phys_allocator: &mut T) -> Result<VirtAddr, SlubError>
    where
        T: PhysicalAllocator,
    {
        // SAFETY: we already verified N is power of two and later i will add more contraints
        let layout = unsafe { Layout::from_size_align_unchecked(N, N) };
        let page = phys_allocator.alloc(layout).map_err(|_| SlubError)?;

        // slub will be embedded
        // [slub_metadata] head -> [obj1] -> [obj2] -> null
        let slub = page.as_mut_ptr::<Slub<N>>();

        // skip slub metadata and align up with respect to N
        let header_size = size_of::<Slub<N>>();
        let start = page.as_64() + header_size as u64;
        let start = align_up(start as usize, N) as u64;
        let offset = start - page.as_64();

        let slots = self.slots(offset as usize);

        let mut head: Slub<N> = Slub {
            next: core::ptr::null_mut(),
            page,
            free_count: slots,
            state: State::Empty,
        };

        for i in (0..slots).rev() {
            let slot_addr = start + (i * N) as u64;
            if unsafe { !FreeBlock::push(&mut head.next, slot_addr as *mut FreeBlock) } {
                return Err(SlubError);
            }
        }
        unsafe {
            core::ptr::write(page.as_mut_ptr(), head);
        };
        self.head = page.as_mut_ptr::<Slub<N>>();
        // this will update head each this if
        // head -> Slub_a after that it will head -> Slub_b _a is lost
        // we want
        // head -> slub_a -> slub_b -> slub_c -> slub_d

        todo!()
    }
}

fn test() {
    let cache: KMemCache<64> = KMemCache::new();
}
