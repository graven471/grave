#![allow(unused)]

use crate::{kprintln, x86_64::interrupts::frame::InterruptStackFrame};
use core::arch::asm;

pub(crate) extern "x86-interrupt" fn do_general_protection_fault(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    kprintln!(
        "GP fault: CS = {:#x}, RIP = {:#x}, RSP = {:#x}",
        stack_frame.cs,
        stack_frame.rip,
        stack_frame.rsp
    );
    unsafe { asm!("hlt") }
}

struct PageFaultErrorCode(u32);
impl PageFaultErrorCode {
    #[inline]
    pub const fn present(&self) -> bool {
        return (self.0 & 0x01) != 0;
    }

    #[inline]
    pub const fn write_fault(&self) -> bool {
        return (self.0 & 0x02) != 0;
    }

    #[inline]
    pub const fn user_mode(&self) -> bool {
        return (self.0 & 0x04) != 0;
    }

    #[inline]
    pub const fn rsvd(&self) -> bool {
        return (self.0 & 0x08) != 0;
    }

    #[inline]
    pub const fn instruction_fetch(&self) -> bool {
        return (self.0 & 0x10) != 0;
    }
}

pub(crate) extern "x86-interrupt" fn do_page_fault(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    let cr2: u64;

    unsafe {
        asm!(
            "mov {}, cr2",
            out(reg) cr2,
            options(nostack, preserves_flags)
        )
    }

    // we can cast error_code to u32 since x64 only used 32 bits for error_code
    // refer to AMD APM vol. 2 page 327
    let err = PageFaultErrorCode(error_code as u32);

    kprintln!("CR2 = {:#x}", cr2);

    kprintln!(
        "present ? {}, write_fault ? {}, user_mode ? {}",
        err.present(),
        err.write_fault(),
        err.user_mode()
    );

    kprintln!("CS = {:#x}, RIP = {:#x}", stack_frame.cs, stack_frame.rip);

    unsafe { asm!("hlt") }
}
