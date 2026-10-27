#![feature(abi_x86_interrupt, likely_unlikely)]
#![no_std]
#![no_main]
#![allow(unsafe_op_in_unsafe_fn)]
use core::{mem::MaybeUninit, panic::PanicInfo};

use crate::{
    boot::BootInfo,
    collections::intrusive_list::ListHead,
    drivers::serial::{self},
    memory::{
        hhdm,
        physical::{PhysicalAllocator, buddy::BuddyAllocator},
        slab::KMemCache,
    },
    x86_64::cpu::{gdt::load_gdt, idt::load_idt},
};

#[macro_use]
pub mod macros;

pub mod boot;
pub mod collections;
pub mod drivers;
pub mod memory;
pub mod sync;
pub mod x86_64;

unsafe extern "C" {
    static __top_init_kernel_stack: u8;
}

#[unsafe(naked)]
#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.boot")]
pub extern "C" fn _start() -> ! {
    core::arch::naked_asm!(
        "lea rsp, [rip + {stack_top}]",
        "and rsp, -16",
        "call {kernel_main}",
        stack_top = sym __top_init_kernel_stack,
        kernel_main = sym kernel_main,
    );
}

static mut CACHE: MaybeUninit<KMemCache<32>> = MaybeUninit::uninit();

#[unsafe(no_mangle)]
pub extern "C" fn kernel_main() -> ! {
    serial::init();

    for _ in 0..5 {
        kprintln!("");
    }

    kprintln!("kernel started");

    load_gdt();
    kprintln!("GDT loaded successfully");

    load_idt();
    kprintln!("IDT loaded successfully");

    let boot_info = BootInfo::new();
    unsafe {
        hhdm::init(boot_info.hhdm_offset);
    }

    let mut frame_allocator =
        BuddyAllocator::init(&boot_info.memory_map).expect("failed to init buddy allocator");
    kprintln!("buddy allocator init successfully");

    // probably the worst code i have ever written ewwww
    unsafe {
        #![allow(static_mut_refs)]
        CACHE.write(KMemCache::new());
        let cache = CACHE.assume_init_mut();
        ListHead::init(&mut cache.list as *mut _);

        let ptr = cache
            .alloc(&mut frame_allocator)
            .expect("slab failed to allocate");
        let p = ptr.as_mut_ptr::<u32>();
        *p = 0x1234;
        kprintln!("p is {:#x}", *p);

        let ptr = cache
            .alloc(&mut frame_allocator)
            .expect("slab failed to allocate");

        let p = ptr.as_mut_ptr::<u32>();
        *p = 0x15678;
        kprintln!("p is {:#x}", *p);
    }

    for y in 0..boot_info.framebuffer.height {
        for x in 0..boot_info.framebuffer.width {
            unsafe {
                boot_info
                    .framebuffer
                    .addr
                    .add(y * boot_info.framebuffer.width + x)
                    .write_volatile(0x00FF00FF);
            }
        }
    }

    loop {
        core::hint::spin_loop();
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    kprintln!("KERNEL PANIC: {}", info);

    loop {
        core::hint::spin_loop();
    }
}
