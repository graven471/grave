#![no_std]
#![no_main]

use core::panic::PanicInfo;

use limine::BaseRevision;

#[used]
#[unsafe(link_section = ".requests")]
static BASE_REVISION: BaseRevision = BaseRevision::new();

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    unsafe {
        let port = 0x3F8 as *mut u8;
        for byte in b"Grave: reached _start\n" {
            port.write_volatile(*byte);
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
