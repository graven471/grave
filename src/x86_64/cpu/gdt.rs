// refer to Intel SDM (page 97) or Amd programming manual vol. 2 (page 146)
//==========================================================
// for kernel code we need to set following:
// DPL = 0 (we are the kernel)
// P = 1 (present)
// L = 1 (this is a 64-bit kernel)
// S = 1 (0 = system; 1 = code or data)
// Type = 0b1010 (code only runs at DPL = 0, Readable = 1)
// ======================================================
// kernel data:
// DPL = 0
// P = 1
// L = 0
// S = 1
// Type = 0b0010 (Read/Write)
// =====================================================

use core::arch::{asm, naked_asm};

const KERNEL_CODE_DESCRIPTOR: u64 = 0x00AF9A000000FFFF;
const KERNEL_DATA_DESCRIPTOR: u64 = 0x00CF92000000FFFF;

pub(super) const KERNEL_CODE_SELECTOR: u16 = 0x08;
pub(super) const KERNEL_DATA_SELECTOR: u16 = 0x10;

const MAX_GDT_ENTRIES: usize = 7;

// system software should align the GDT on a quadword boundary to avoid the potential performance penalties associated
// with accessing unaligned data
#[repr(align(8))]
struct Gdt([u64; MAX_GDT_ENTRIES]);

// later setup User code and data and TSS
static GDT: Gdt = Gdt([
    0x00u64,
    KERNEL_CODE_DESCRIPTOR,
    KERNEL_DATA_DESCRIPTOR,
    0x0,
    0x0,
    0x0,
    0x0,
]);

#[repr(packed)]
pub struct Gdtr {
    pub limit: u16,
    pub base_address: u64,
}

#[unsafe(naked)]
unsafe extern "C" fn reload_cs(cs: u64) {
    // we can't just mov cs, 0x10 cause changing CS chages CPL etc
    // Reload CS with a far control transfer, not MOV.
    naked_asm!(
        "pop rax",  // pop return address
        "push rdi", // push new CS (first arg in rdi)
        "push rax", // push return address as RIP
        "retfq",
    );
}

#[unsafe(naked)]
unsafe extern "C" fn reload_ds(ds: u16) {
    naked_asm!("mov ds, di", "mov es, di", "mov ss, di", "ret",);
}

pub fn load_gdt() {
    let gdtr = Gdtr {
        limit: (MAX_GDT_ENTRIES as u16 * 8) - 1,
        base_address: GDT.0.as_ptr() as u64,
    };

    unsafe {
        asm!(
            "lgdt [{}]",
            in(reg) &gdtr as *const Gdtr,
            options(readonly, nostack, preserves_flags)
        );
        reload_ds(KERNEL_DATA_SELECTOR);
        reload_cs(KERNEL_CODE_SELECTOR as u64);
    };
}
