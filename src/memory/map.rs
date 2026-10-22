use limine::memmap::{self, Entry};

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
