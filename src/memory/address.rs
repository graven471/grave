use crate::memory::hhdm;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct PhysAddr(u64);

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct VirtAddr(u64);

impl PhysAddr {
    #[inline]
    pub fn new(addr: u64) -> Option<Self> {
        // lower 12 bits needs to be 0
        if (addr & ((1u64 << 12) - 1)) != 0 || (addr >> 52) != 0 {
            return None;
        }
        return Some(Self { 0: addr });
    }

    #[inline]
    pub unsafe fn new_unchecked(addr: u64) -> Self {
        Self { 0: addr }
    }

    #[inline]
    pub const fn as_64(&self) -> u64 {
        self.0
    }

    #[inline]
    pub const fn as_ptr<T>(self) -> *const T {
        self.0 as *const T
    }

    #[inline]
    pub const fn as_mut_ptr<T>(self) -> *mut T {
        self.0 as *mut T
    }
}

impl VirtAddr {
    #[inline]
    pub fn new(addr: u64) -> Option<Self> {
        // 4-level paging: bits 63..48 must sign-extend bit 47.
        let upper = addr >> 48;
        let sign = (addr >> 47) & 1;

        if (sign == 0 && upper != 0) || (sign == 1 && upper != 0xFFFF) {
            return None;
        }

        Some(Self(addr))
    }

    #[inline]
    pub unsafe fn new_unchecked(addr: u64) -> Self {
        Self(addr)
    }

    #[inline]
    pub const fn as_64(&self) -> u64 {
        self.0
    }

    #[inline]
    pub const fn as_ptr<T>(self) -> *const T {
        self.0 as *const T
    }

    #[inline]
    pub const fn as_mut_ptr<T>(self) -> *mut T {
        self.0 as *mut T
    }
}

// #[inline(always)]
// pub fn phys_to_virt(phys: PhysAddr) -> u64 {
//     return phys.as_64() + hhdm::offset();
// }
//
// pub const fn virt_to_phys(_virt: u64) -> u64 {
//     todo!()
// }

pub trait TranslateAddr<T> {
    fn translate(self) -> Option<T>;
}

impl TranslateAddr<VirtAddr> for PhysAddr {
    #[inline]
    fn translate(self) -> Option<VirtAddr> {
        let addr = self.0.checked_add(crate::memory::hhdm::offset())?;
        VirtAddr::new(addr)
    }
}

impl TranslateAddr<PhysAddr> for VirtAddr {
    #[inline]
    fn translate(self) -> Option<PhysAddr> {
        let addr = self.0.checked_sub(crate::memory::hhdm::offset())?;
        PhysAddr::new(addr)
    }
}
