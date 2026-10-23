use crate::{drivers::serial::uart16550::Uart, sync::once_lock::OnceLock};

pub mod uart16550;

pub trait Serial {
    type Error;

    fn init() -> Result<Self, Self::Error>
    where
        Self: Sized;

    fn write_byte(&mut self, byte: u8);
    fn read_byte(&self) -> u8;
}

pub struct SerialWriter<T: Serial>(pub T);

impl<T: Serial> SerialWriter<T> {
    pub const fn new(serial: T) -> Self {
        Self(serial)
    }
}

impl<T: Serial> core::fmt::Write for SerialWriter<T> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for b in s.bytes() {
            self.0.write_byte(b);
        }
        Ok(())
    }
}

// TODO: see if we can do this in a better way
// this is a ticking bomb

static WRITER: OnceLock<UnsafeCell<SerialWriter<Uart>>> = OnceLock::new();

pub fn init() {
    WRITER.get_or_init(|| {
        let uart = Uart::init().expect("UART initialization failed");
        UnsafeCell::new(SerialWriter::new(uart))
    });
}

use core::{cell::UnsafeCell, fmt::Write};

#[doc(hidden)]
pub fn _print(args: core::fmt::Arguments<'_>) {
    let flags: u64;

    unsafe {
        core::arch::asm!(
            "pushfq",
            "pop {}",
            "cli",
            out(reg) flags,
            options(preserves_flags),
        );
    }

    let writer = WRITER.get_or_init(|| {
        let uart = Uart::init().expect("UART initialization failed");
        UnsafeCell::new(SerialWriter::new(uart))
    });

    unsafe {
        (*writer.get())
            .write_fmt(args)
            .expect("serial write failed");
    }

    if flags & (1 << 9) != 0 {
        unsafe {
            core::arch::asm!("sti", options(nomem, nostack));
        }
    }
}
