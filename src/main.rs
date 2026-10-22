#![no_std]
#![no_main]

use core::panic::PanicInfo;

use core::fmt::Write;

use crate::{
    boot::BootInfo,
    drivers::serial::{Serial, SerialWriter, uart16550::Uart},
};

pub mod boot;
pub mod drivers;
pub mod memory;
pub mod x86_64;

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    let boot_info = BootInfo::new();

    let uart = Uart::init().expect("failed to init UART");
    let mut writer = SerialWriter(uart);

    writeln!(writer, "i said, {}", "hey").unwrap();

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
