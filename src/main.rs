#![feature(abi_x86_interrupt)]
#![no_std]
#![no_main]

use core::panic::PanicInfo;

use core::fmt::Write;

use crate::{
    boot::BootInfo,
    drivers::serial::{Serial, SerialWriter, uart16550::Uart},
    x86_64::{gdt::load_gdt, idt::load_idt},
};

pub mod boot;
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

#[unsafe(no_mangle)]
pub extern "C" fn kernel_main() -> ! {
    let uart = Uart::init().expect("failed to init UART");
    let mut writer = SerialWriter(uart);
    writeln!(writer, "kernel started").unwrap();

    for _ in 0..5 {
        // this is to prevent qemu firmware print cloberring our logs
        writeln!(writer, "").unwrap();
    }

    load_gdt();
    writeln!(writer, "GDT loaded successfully").unwrap();

    load_idt();
    writeln!(writer, "IDT loaded successfully").unwrap();

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
