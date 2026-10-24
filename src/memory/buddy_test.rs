#![allow(unused)]
use core::{
    alloc::Layout,
    arch::x86_64::{__rdtscp, _rdtsc},
};

use crate::{
    kprintln,
    memory::{
        address::{PhysAddr, TranslateAddr, VirtAddr},
        buddy::{BuddyAllocator, PhysicalAllocator},
        map::MemoryMap,
    },
};

const TEST_MIN_SIZE: usize = 4096;
const TEST_SLOTS: usize = 128;
const TEST_ROUNDS: usize = 512;

struct TestRng(u64);

impl TestRng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    fn range(&mut self, upper: usize) -> usize {
        assert!(upper != 0);
        (self.next() as usize) % upper
    }
}

#[inline]
fn test_layout(size: usize, align: usize) -> Layout {
    Layout::from_size_align(size, align).unwrap()
}

// Expected buddy block size, assuming order zero is 4 KiB
// and allocation order is based on max(size, alignment).
#[inline]
fn expected_block_size(layout: Layout) -> usize {
    core::cmp::max(TEST_MIN_SIZE, core::cmp::max(layout.size(), layout.align())).next_power_of_two()
}

// pub fn test_buddy_allocator_stress(memory_map: &MemoryMap) {
//     kprintln!("=== buddy allocator stress tests ===");
//
//     let mut allocator =
//         BuddyAllocator::init(memory_map).expect("buddy allocator initialization failed");
//
//     // --------------------------------------------------------
//     // Test 1: Minimum-size allocation and alignment.
//     // --------------------------------------------------------
//     for &(size, align) in &[
//         (1, 1),
//         (4, 4),
//         (64, 8),
//         (4095, 1),
//         (4096, 4096),
//         (4097, 1),
//         (4096, 8192),
//         (8192, 4096),
//         (16384, 16384),
//     ] {
//         let layout = test_layout(size, align);
//         let allocation = allocator
//             .alloc(layout)
//             .expect("valid allocation unexpectedly failed");
//
//         let virt = allocation
//             .translate()
//             .expect("allocation translation failed");
//         let ptr = virt.as_mut_ptr::<u8>();
//         let addr = ptr as usize;
//
//         assert_eq!(
//             addr % align,
//             0,
//             "misaligned allocation: size={}, align={}, addr={:#x}",
//             size,
//             align,
//             addr
//         );
//
//         // Verify that the requested bytes are writable.
//         unsafe {
//             ptr.write(0xA5);
//             assert_eq!(ptr.read(), 0xA5);
//         }
//
//         allocator
//             .free(allocation, layout)
//             .expect("free failed after basic allocation test");
//     }
//
//     kprintln!("[PASS] minimum sizes, alignment, translation, read/write");
//
//     // --------------------------------------------------------
//     // Test 2: Split a larger block with mixed allocation sizes.
//     // --------------------------------------------------------
//     let layouts = [
//         test_layout(4096, 4096),
//         test_layout(4096, 4096),
//         test_layout(8192, 8192),
//         test_layout(16384, 4096),
//         test_layout(1024, 1024),
//         test_layout(32768, 32768),
//         test_layout(4096, 4096),
//     ];
//
//     // Inferred as an array of optional allocation handles.
//     let mut slots = core::array::from_fn::<_, 7, _>(|_| None);
//     let mut addresses = [0usize; 7];
//     let mut block_sizes = [0usize; 7];
//
//     for i in 0..layouts.len() {
//         let layout = layouts[i];
//         kprintln!(
//             "before alloc: size={}, align={}",
//             layout.size(),
//             layout.align()
//         );
//         let allocation = allocator.alloc(layout).expect("mixed allocation failed");
//         kprintln!("after alloc");
//
//         let virt = allocation.translate().unwrap();
//         let addr = virt.as_mut_ptr::<u8>() as usize;
//
//         assert_eq!(addr % layout.align(), 0);
//
//         addresses[i] = addr;
//         block_sizes[i] = expected_block_size(layout);
//         slots[i] = Some((allocation, layout));
//     }
//
//     // The live buddy blocks must not overlap.
//     for i in 0..layouts.len() {
//         for j in (i + 1)..layouts.len() {
//             let a0 = addresses[i];
//             let a1 = a0 + block_sizes[i];
//             let b0 = addresses[j];
//             let b1 = b0 + block_sizes[j];
//
//             assert!(
//                 a1 <= b0 || b1 <= a0,
//                 "overlapping live blocks: slot {} [{:#x}, {:#x}), \
//                  slot {} [{:#x}, {:#x})",
//                 i,
//                 a0,
//                 a1,
//                 j,
//                 b0,
//                 b1
//             );
//         }
//     }
//
//     // Free in a non-LIFO order to exercise buddy merging.
//     for i in [3, 0, 5, 2, 6, 1, 4] {
//         let (allocation, layout) = slots[i].take().unwrap();
//         kprintln!(
//             "before free: size={}, align={}",
//             layout.size(),
//             layout.align()
//         );
//         allocator
//             .free(allocation, layout)
//             .expect("free failed in non-LIFO sequence");
//         kprintln!("after free");
//     }
//
//     // If the allocator tracks free lists correctly, it should
//     // still support a subsequent large allocation.
//     let layout = test_layout(65536, 65536);
//     let allocation = allocator
//         .alloc(layout)
//         .expect("allocation failed after freeing mixed blocks");
//
//     allocator
//         .free(allocation, layout)
//         .expect("free failed after mixed allocation test");
//
//     kprintln!("[PASS] splitting, non-overlap, non-LIFO freeing");
//
//     // --------------------------------------------------------
//     // Test 3: Repeated allocation/free cycles.
//     // --------------------------------------------------------
//     for round in 0..TEST_ROUNDS {
//         let size = match round % 8 {
//             0 => 1,
//             1 => 64,
//             2 => 4095,
//             3 => 4096,
//             4 => 4097,
//             5 => 8192,
//             6 => 16384,
//             _ => 32768,
//         };
//
//         let align = match round % 5 {
//             0 => 1,
//             1 => 8,
//             2 => 4096,
//             3 => 8192,
//             _ => 16384,
//         };
//
//         let layout = test_layout(size, align);
//         let allocation = allocator
//             .alloc(layout)
//             .expect("allocation failed during repeated cycles");
//
//         let addr = allocation.translate().unwrap().as_mut_ptr::<u8>() as usize;
//
//         assert_eq!(addr % align, 0);
//
//         allocator
//             .free(allocation, layout)
//             .expect("free failed during repeated cycles");
//     }
//
//     kprintln!("[PASS] {} repeated allocation/free cycles", TEST_ROUNDS);
//
//     // --------------------------------------------------------
//     // Test 4: Multiple simultaneous allocations and fragmentation.
//     // --------------------------------------------------------
//     let mut live = core::array::from_fn::<_, TEST_SLOTS, _>(|_| None);
//     let mut live_addr = [0usize; TEST_SLOTS];
//     let mut live_size = [0usize; TEST_SLOTS];
//
//     // Fill the slots with a varied mix of sizes.
//     for i in 0..TEST_SLOTS {
//         let size = match i % 6 {
//             0 => 4096,
//             1 => 8192,
//             2 => 16384,
//             3 => 32768,
//             4 => 65536,
//             _ => 4096,
//         };
//
//         let align = size;
//         let layout = test_layout(size, align);
//
//         let allocation = allocator
//             .alloc(layout)
//             .expect("allocator failed during simultaneous allocations");
//
//         let addr = allocation.translate().unwrap().as_mut_ptr::<u8>() as usize;
//
//         assert_eq!(addr % align, 0);
//
//         live_addr[i] = addr;
//         live_size[i] = expected_block_size(layout);
//         live[i] = Some((allocation, layout));
//     }
//
//     // Every live allocation must occupy a disjoint address range.
//     for i in 0..TEST_SLOTS {
//         for j in (i + 1)..TEST_SLOTS {
//             let a0 = live_addr[i];
//             let a1 = a0 + live_size[i];
//             let b0 = live_addr[j];
//             let b1 = b0 + live_size[j];
//
//             assert!(
//                 a1 <= b0 || b1 <= a0,
//                 "live allocations overlap: {} and {}",
//                 i,
//                 j
//             );
//         }
//     }
//
//     // Free alternating blocks first, then the remaining blocks.
//     for parity in [0usize, 1usize] {
//         for i in 0..TEST_SLOTS {
//             if i % 2 == parity {
//                 let (allocation, layout) = live[i].take().unwrap();
//                 allocator
//                     .free(allocation, layout)
//                     .expect("free failed in fragmentation test");
//             }
//         }
//     }
//
//     // The allocator must remain usable after all blocks are freed.
//     let layout = test_layout(1024 * 1024, 1024 * 1024);
//     let allocation = allocator
//         .alloc(layout)
//         .expect("large allocation failed after fragmentation test");
//
//     allocator
//         .free(allocation, layout)
//         .expect("large allocation free failed");
//
//     kprintln!("[PASS] simultaneous allocations, fragmentation, recovery");
//     kprintln!("=== buddy allocator stress tests passed ===");
// }
//

// pub fn test_buddy_allocator_stress(memory_map: &MemoryMap) {
//     let mut allocator =
//         BuddyAllocator::init(memory_map).expect("buddy allocator initialization failed");
//
//     let layouts = [
//         Layout::from_size_align(4096, 4096).unwrap(),
//         Layout::from_size_align(4096, 4096).unwrap(),
//         Layout::from_size_align(8192, 8192).unwrap(),
//         Layout::from_size_align(16384, 4096).unwrap(),
//         Layout::from_size_align(1024, 1024).unwrap(),
//     ];
//
//     kprintln!("alloc 1: 4 KiB");
//     let a = allocator.alloc(layouts[0]).unwrap();
//
//     kprintln!("alloc 2: 4 KiB");
//     let b = allocator.alloc(layouts[1]).unwrap();
//
//     kprintln!("alloc 3: 8 KiB");
//     let c = allocator.alloc(layouts[2]).unwrap();
//
//     kprintln!("alloc 4: 16 KiB");
//     let d = allocator.alloc(layouts[3]).unwrap();
//
//     kprintln!("alloc 5: 1 KiB");
//     let e = allocator.alloc(layouts[4]).unwrap();
//
//     kprintln!("all allocations succeeded");
//
//     kprintln!("free 3: 8 KiB");
//     allocator.free(c, layouts[2]).expect("free c failed");
//
//     kprintln!("free 1: 4 KiB");
//     allocator.free(a, layouts[0]).expect("free a failed");
//
//     kprintln!("free 5: 1 KiB");
//     allocator.free(e, layouts[4]).expect("free e failed");
//
//     kprintln!("free 4: 16 KiB");
//     allocator.free(d, layouts[3]).expect("free d failed");
//
//     kprintln!("free 2: 4 KiB");
//     allocator.free(b, layouts[1]).expect("free b failed");
//
//     kprintln!("all frees succeeded");
//
//     // Verify the allocator remains usable after freeing everything.
//     let large_layout = Layout::from_size_align(1024 * 1024, 1024 * 1024).unwrap();
//
//     kprintln!("alloc 6: 1 MiB after frees");
//     let large = allocator
//         .alloc(large_layout)
//         .expect("1 MiB allocation failed after freeing all blocks");
//
//     kprintln!("free 6: 1 MiB");
//     allocator
//         .free(large, large_layout)
//         .expect("free large allocation failed");
//
//     kprintln!("buddy allocator alloc/free test passed");
// }

use core::sync::atomic::{AtomicBool, Ordering};

const EXHAUSTION_SLOTS: usize = 2048;
const PAGE_SIZE: usize = 4096;

static TEST_RUNNING: AtomicBool = AtomicBool::new(false);

static mut ALLOCATIONS: [Option<PhysAddr>; EXHAUSTION_SLOTS] = [const { None }; EXHAUSTION_SLOTS];

static mut ORDER: [usize; EXHAUSTION_SLOTS] = [0; EXHAUSTION_SLOTS];

pub fn test_buddy_allocator_stress(memory_map: &MemoryMap) {
    // Prevent concurrent invocations from sharing the static arrays.
    assert!(
        TEST_RUNNING
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_ok(),
        "buddy allocator stress test is already running"
    );

    kprintln!("=== buddy allocator exhaustion test ===");

    let mut allocator =
        BuddyAllocator::init(memory_map).expect("buddy allocator initialization failed");

    let page_layout = Layout::from_size_align(PAGE_SIZE, PAGE_SIZE).unwrap();

    let mut count = 0usize;

    // This test exclusively owns the static arrays while TEST_RUNNING is true.
    unsafe {
        let allocations_ptr = core::ptr::addr_of_mut!(ALLOCATIONS).cast::<Option<PhysAddr>>();

        for i in 0..EXHAUSTION_SLOTS {
            allocations_ptr.add(i).write(None);
        }

        let order_ptr = core::ptr::addr_of_mut!(ORDER).cast::<usize>();

        for i in 0..EXHAUSTION_SLOTS {
            order_ptr.add(i).write(i);
        }
    }

    kprintln!("phase 1: allocating 4 KiB blocks until failure");

    while count < EXHAUSTION_SLOTS {
        match allocator.alloc(page_layout) {
            Ok(allocation) => {
                let addr = allocation
                    .translate()
                    .expect("translation failed")
                    .as_mut_ptr::<u8>() as usize;

                assert_eq!(
                    addr % PAGE_SIZE,
                    0,
                    "misaligned page allocation at {:#x}",
                    addr
                );

                // Write a per-allocation signature.
                unsafe {
                    (addr as *mut u64).write((count as u64) ^ 0xDEAD_BEEF_CAFE_BABE);
                    ALLOCATIONS[count] = Some(allocation);
                }

                count += 1;

                if count % 256 == 0 {
                    kprintln!("allocated {} pages", count);
                }
            }
            Err(_) => {
                kprintln!("allocator exhausted after {} allocations", count);
                break;
            }
        }
    }

    assert!(
        count > 0,
        "allocator could not satisfy one 4 KiB allocation"
    );

    if count == EXHAUSTION_SLOTS {
        kprintln!(
            "[INFO] Test recording capacity reached ({} allocations); allocator exhaustion not verified",
            count
        );
    }

    // Verify all live allocations still contain their signatures.
    for i in 0..count {
        let allocation = unsafe { ALLOCATIONS[i].as_ref().unwrap() };

        let addr = allocation
            .translate()
            .expect("translation failed during verification")
            .as_mut_ptr::<u8>() as usize;

        let actual = unsafe { (addr as *const u64).read() };
        let expected = (i as u64) ^ 0xDEAD_BEEF_CAFE_BABE;

        assert_eq!(
            actual, expected,
            "payload corruption in allocation {} at {:#x}",
            i, addr
        );
    }

    kprintln!("[PASS] exhaustion and payload integrity");

    // Deterministic Fisher-Yates shuffle using static storage.
    let mut rng = ExhaustionRng(0x9E37_79B9_7F4A_7C15);

    unsafe {
        let order_ptr = core::ptr::addr_of_mut!(ORDER).cast::<usize>();

        for i in (1..count).rev() {
            let j = rng.range(i + 1);
            let a = order_ptr.add(i);
            let b = order_ptr.add(j);
            core::ptr::swap(a, b);
        }
    }

    kprintln!("phase 2: freeing {} allocations in shuffled order", count);

    for n in 0..count {
        let i = unsafe { ORDER[n] };
        let allocation = unsafe { ALLOCATIONS[i].take().unwrap() };

        allocator
            .free(allocation, page_layout)
            .expect("free failed during shuffled recovery");

        if (n + 1) % 256 == 0 {
            kprintln!("freed {} pages", n + 1);
        }
    }

    assert!(
        unsafe { ALLOCATIONS[..count].iter().all(|slot| slot.is_none()) },
        "some allocation handles remain live"
    );

    kprintln!("[PASS] all allocations freed");

    // Verify recovery with a large block.
    let large_layout = Layout::from_size_align(1024 * 1024, 1024 * 1024).unwrap();

    kprintln!("phase 3: requesting 1 MiB after recovery");

    let large = allocator
        .alloc(large_layout)
        .expect("1 MiB allocation failed after exhaustion recovery");

    let addr = large
        .translate()
        .expect("large allocation translation failed")
        .as_mut_ptr::<u8>() as usize;

    assert_eq!(addr % large_layout.align(), 0);

    unsafe {
        let ptr = addr as *mut u64;
        ptr.write(0xDEAD_BEEF_CAFE_BABE);
        assert_eq!(ptr.read(), 0xDEAD_BEEF_CAFE_BABE);
    }

    allocator
        .free(large, large_layout)
        .expect("free of recovered 1 MiB block failed");

    kprintln!("[PASS] 1 MiB allocation and free after recovery");

    kprintln!(
        "=== exhaustion test passed: {} pages allocated and recovered ===",
        count
    );

    TEST_RUNNING.store(false, Ordering::Release);
}

pub fn test_buddy_allocator_benchmark(memory_map: &MemoryMap) {
    let mut allocator =
        BuddyAllocator::init(memory_map).expect("buddy allocator initialization failed");

    kprintln!("=== buddy allocator benchmark ===");

    // benchmark_buddy_allocator(&mut allocator);
    benchmark_buddy_allocator_2(&mut allocator);

    kprintln!("=== benchmark complete ===");
}

struct ExhaustionRng(u64);

impl ExhaustionRng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    fn range(&mut self, upper: usize) -> usize {
        assert!(upper > 0);
        (self.next() as usize) % upper
    }
}

#[cfg(target_arch = "x86_64")]
#[inline]
unsafe fn read_tsc() -> u64 {
    unsafe { core::arch::x86_64::_rdtsc() }
}

#[inline(always)]
unsafe fn read_tsc_start() -> u64 {
    unsafe {
        core::arch::asm!("lfence", options(nostack, preserves_flags));
        core::arch::x86_64::_rdtsc()
    }
}

// #[inline(always)]
// unsafe fn read_tsc_end() -> u64 {
//     unsafe {
//         let mut aux: u32 = 0;
//         let ticks = core::arch::x86_64::__rdtscp(&mut aux);
//         core::arch::asm!("lfence", options(nostack, preserves_flags));
//         ticks
//     }
// }
//
use core::arch::asm;

#[inline]
pub unsafe fn read_tsc_serialized() -> u64 {
    unsafe {
        let lo: u32;
        let hi: u32;

        asm!(
            "lfence",
            "rdtsc",
            lateout("eax") lo,
            lateout("edx") hi,
            options(nomem, nostack, preserves_flags),
        );

        ((hi as u64) << 32) | (lo as u64)
    }
}

#[inline(always)]
pub unsafe fn read_tsc_end() -> u64 {
    unsafe {
        let lo: u32;
        let hi: u32;
        let aux: u32;

        asm!(
            "rdtscp",
            "lfence",
            lateout("eax") lo,
            lateout("edx") hi,
            lateout("ecx") aux,
            options(nomem, nostack, preserves_flags),
        );

        let _ = aux;
        ((hi as u64) << 32) | (lo as u64)
    }
}

pub fn benchmark_buddy_allocator(allocator: &mut BuddyAllocator) {
    const ITERATIONS: usize = 1000;

    let layout = Layout::from_size_align(4096, 4096).unwrap();

    let mut alloc_cycles = 0u64;
    let mut free_cycles = 0u64;

    // Warm up first.
    for _ in 0..10_000 {
        let allocation = allocator.alloc(layout).unwrap();
        allocator.free(allocation, layout).unwrap();
    }

    for _ in 0..ITERATIONS {
        // Benchmark allocation
        let start = unsafe { read_tsc_serialized() };
        let allocation = allocator.alloc(layout).expect("allocation failed");
        let end = unsafe { read_tsc_end() };

        alloc_cycles += end - start;

        // Benchmark free
        let start = unsafe { read_tsc_serialized() };
        allocator.free(allocation, layout).expect("free failed");
        let end = unsafe { read_tsc_end() };

        free_cycles += end - start;
    }

    kprintln!(
        "alloc average: {} TSC ticks",
        alloc_cycles / ITERATIONS as u64
    );

    kprintln!(
        "free average: {} TSC ticks",
        free_cycles / ITERATIONS as u64
    );
}

const ITERATIONS: usize = 10000;

static mut ALLOC_LATENCIES: [u64; ITERATIONS] = [0; ITERATIONS];
static mut FREE_LATENCIES: [u64; ITERATIONS] = [0; ITERATIONS];

pub fn benchmark_buddy_allocator_2(allocator: &mut BuddyAllocator) {
    let layout = Layout::from_size_align(4096, 4096).unwrap();

    // Warm up before collecting measurements.
    for _ in 0..100 {
        let ptr = allocator.alloc(layout).expect("allocation failed");
        allocator.free(ptr, layout).expect("free failed");
    }

    let alloc_ptr = core::ptr::addr_of_mut!(ALLOC_LATENCIES).cast::<u64>();
    let free_ptr = core::ptr::addr_of_mut!(FREE_LATENCIES).cast::<u64>();

    for i in 0..ITERATIONS {
        let start = unsafe { read_tsc_start() };
        let allocation = allocator.alloc(layout).expect("allocation failed");
        let end = unsafe { read_tsc_end() };

        unsafe {
            alloc_ptr.add(i).write(end - start);
        }

        let start = unsafe { read_tsc_start() };
        allocator.free(allocation, layout).expect("free failed");
        let end = unsafe { read_tsc_end() };

        unsafe {
            free_ptr.add(i).write(end - start);
        }
    }

    unsafe {
        report_latency("alloc 4 KiB", alloc_ptr, ITERATIONS);
        report_latency("free 4 KiB", free_ptr, ITERATIONS);
    }
}

pub fn benchmark_buddy_allocator_2_split(allocator: &mut BuddyAllocator) {
    const ALLOC_COUNT: usize = ITERATIONS;
    let layout = Layout::from_size_align(4096, 4096).unwrap();

    static mut ALLOCATIONS: [*mut u8; ITERATIONS] = [core::ptr::null_mut(); ITERATIONS];

    let alloc_ptr = core::ptr::addr_of_mut!(ALLOC_LATENCIES).cast::<u64>();
    let free_ptr = core::ptr::addr_of_mut!(FREE_LATENCIES).cast::<u64>();
    let allocations_ptr = core::ptr::addr_of_mut!(ALLOCATIONS).cast::<*mut u8>();

    // Keep every allocation live to prevent immediate reuse.
    for i in 0..ALLOC_COUNT {
        let start = unsafe { read_tsc_start() };
        let ptr = allocator.alloc(layout).expect("allocation failed");
        let end = unsafe { read_tsc_end() };

        unsafe {
            alloc_ptr.add(i).write(end - start);
            allocations_ptr.add(i).write(ptr.as_mut_ptr::<u8>());
        }
    }

    for i in 0..ALLOC_COUNT {
        let ptr = unsafe { allocations_ptr.add(i).read() };

        let start = unsafe { read_tsc_start() };
        allocator
            .free(unsafe { PhysAddr::new_unchecked(ptr as u64) }, layout)
            .expect("free failed");
        let end = unsafe { read_tsc_end() };
        unsafe {
            free_ptr.add(i).write(end - start);
        }
    }
    unsafe {
        report_latency("alloc 4 KiB", alloc_ptr, ITERATIONS);
        report_latency("free 4 KiB", free_ptr, ITERATIONS);
    }
}

unsafe fn report_latency(label: &str, values: *mut u64, len: usize) {
    // Sort in place to calculate distribution statistics.
    for i in 1..len {
        let value = unsafe { *values.add(i) };
        let mut j = i;

        while j > 0 && unsafe { *values.add(j - 1) } > value {
            unsafe {
                *values.add(j) = *values.add(j - 1);
            }
            j -= 1;
        }

        unsafe {
            *values.add(j) = value;
        }
    }

    let mut total = 0u64;
    for i in 0..len {
        total += unsafe { *values.add(i) };
    }

    kprintln!(
        "{}: min={} median={} avg={} max={} TSC ticks",
        label,
        unsafe { *values },
        unsafe { *values.add(len / 2) },
        total / len as u64,
        unsafe { *values.add(len - 1) },
    );
}
