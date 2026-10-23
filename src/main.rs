#![feature(abi_x86_interrupt)]
#![no_std]
#![no_main]

use core::panic::PanicInfo;

use crate::{
    boot::BootInfo,
    drivers::serial::{self},
    x86_64::cpu::{gdt::load_gdt, idt::load_idt},
};

pub mod boot;
pub mod drivers;
pub mod macros;
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

    unsafe {
        let val: u64;
        core::arch::asm!("mov {}, [0x12345678]", out(reg) val,)
    }

    let boot_info = BootInfo::new();

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
fn panic(_info: &PanicInfo) -> ! {
    loop {
        core::hint::spin_loop();
    }
}
