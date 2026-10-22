use core::arch::asm;

pub struct Port {
    base: u16,
}

impl Port {
    // in/inb: read a byte from I/O port
    // out/outb: write a byte to I/O port
    // examples:
    // in al, dx -> read a byte from I/O port DX into AL
    // out dx, al -> write AL to I/O port DX
    pub fn new(base: u16) -> Self {
        Self { base }
    }

    #[inline]
    pub unsafe fn read(&self) -> u8 {
        let val: u8;
        unsafe {
            asm!(
                "in al, dx",
                // make the rust value self.base available in dx register
                in("dx") self.base,
                // copy the value of al into val
                out("al") val,
                options(nomem, nostack, preserves_flags),
            )
        }
        return val;
    }

    #[inline]
    pub unsafe fn write(&self, val: u8) {
        unsafe {
            asm!(
                "out dx, al",
                in("dx") self.base,
                in("al") val,
                options(nomem, nostack, preserves_flags),
            );
        }
    }
}
