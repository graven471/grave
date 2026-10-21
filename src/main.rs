#![allow(unused)]
#![no_std]
#![no_main]

/*
* core::arch::asm! primer
* in(reg) -> input
* out(reg) -> output
* inout(reg) -> read then write
* in("rax") -> specific register
* out("rax") -> specific register output
*
* options:
* nomem -> dosen't touch memory
* nostack -> dosen't touch stack
* noreturn -> never returns (for hlt loops etc)
* preserve_flags -> dosen't clobber RFLAGS
*
* examples:
* writing:
* unsafe {
    asm!("mov cr3, {}", in(reg) value);
}
reading:
let cr3: u64;
unsafe {
    asm!("mov {}, cr3", out(reg) cr3);
}
*/

use core::panic::PanicInfo;

use core::arch::asm;

use limine::{
    BaseRevision,
    memmap::{self, Entry},
    request::{FramebufferRequest, HhdmRequest, MemmapRequest, RsdpRequest},
};

pub mod x86_64;

#[used]
#[unsafe(link_section = ".requests")]
static BASE_REVISION: BaseRevision = BaseRevision::new();

#[used]
#[unsafe(link_section = ".requests")]
static FB_REQUEST: FramebufferRequest = FramebufferRequest::new();

#[used]
#[unsafe(link_section = ".requests")]
static HHDM_RESQUEST: HhdmRequest = HhdmRequest::new();

#[used]
#[unsafe(link_section = ".requests")]
static MEMORY_MAP_REQUEST: MemmapRequest = MemmapRequest::new();

#[used]
#[unsafe(link_section = ".requests")]
static RDSP_REQUEST: RsdpRequest = RsdpRequest::new();

pub struct BootInfo {
    pub framebuffer: FrameBuffer,
    pub memory_map: MemoryMap,
    hhdm_offset: u64,
    rsdp_addr: u64,
}

pub struct FrameBuffer {
    pub addr: *mut u32,
    pub width: usize,
    pub height: usize,
    pub pitch: usize,
}

pub struct MemoryMap {
    pub entries: &'static [&'static memmap::Entry],
}

impl MemoryMap {
    #[inline]
    pub fn usable(&self) -> impl Iterator<Item = &&Entry> {
        self.entries
            .iter()
            .filter(|&&e| e.type_ == memmap::MEMMAP_USABLE)
    }

    #[inline]
    pub fn reserved(&self) -> impl Iterator<Item = &&Entry> {
        self.entries
            .iter()
            .filter(|&&e| e.type_ == memmap::MEMMAP_RESERVED)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MemoryMapKind {
    Usable,
    AcpiReclaimable,
    AcpiNvs,
    BadMemory,
    BootloaderReclaimable,
    ExeAndModule,
    FrameBuffer,
    Reserved,
}

impl TryFrom<u64> for MemoryMapKind {
    type Error = ();

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        match value {
            memmap::MEMMAP_USABLE => Ok(Self::Usable),
            memmap::MEMMAP_RESERVED => Ok(Self::Reserved),
            memmap::MEMMAP_ACPI_RECLAIMABLE => Ok(Self::AcpiReclaimable),
            memmap::MEMMAP_BAD_MEMORY => Ok(Self::BadMemory),
            memmap::MEMMAP_BOOTLOADER_RECLAIMABLE => Ok(Self::BootloaderReclaimable),
            memmap::MEMMAP_EXECUTABLE_AND_MODULES => Ok(Self::ExeAndModule),
            memmap::MEMMAP_FRAMEBUFFER => Ok(Self::FrameBuffer),
            _ => Err(()),
        }
    }
}

pub fn get_boot_info() -> BootInfo {
    let fb_resp = FB_REQUEST.response().unwrap();
    let fb = fb_resp.framebuffers()[0];
    let mm_resp = MEMORY_MAP_REQUEST.response().unwrap();
    let hhdm_resp = HHDM_RESQUEST.response().unwrap();
    let rsdp_resp = RDSP_REQUEST.response().unwrap();

    let framebuffer = FrameBuffer {
        addr: fb.address() as *mut u32,
        width: fb.width as usize,
        height: fb.height as usize,
        pitch: fb.pitch as usize,
    };

    let mem_map = MemoryMap {
        entries: mm_resp.entries(),
    };

    return BootInfo {
        framebuffer,
        memory_map: mem_map,
        hhdm_offset: hhdm_resp.offset,
        rsdp_addr: rsdp_resp.address as u64,
    };
}

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

pub fn init_uart_driver() {
    // first serial port
    const COM_1_PORT: u16 = 0x3F8;
    let port = Port::new(COM_1_PORT);

    // 16550 register:
    // let base = COM_1_PORT
    // base = RBR/THR/DLL
    // base + 1 = IER/DLM
    // base + 2 = IIR/FCR
    // base + 3 = LCR
    // base + 4 = MCR
    // base + 5 = LSR
    // base + 6 = MSR
    // base + 7 = SCR

    todo!()
}

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    assert!(BASE_REVISION.is_supported());

    let boot_info = get_boot_info();

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
