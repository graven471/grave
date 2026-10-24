static mut HHDM_OFFSET: u64 = 0;

#[cold]
pub unsafe fn init(offset: u64) {
    unsafe {
        HHDM_OFFSET = offset;
    }
}

#[inline(always)]
pub fn offset() -> u64 {
    unsafe { HHDM_OFFSET }
}
