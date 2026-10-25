#[inline(always)]
#[cfg(target_arch = "x86_64")]
pub unsafe fn prefetch_read_t0(ptr: *const u8) {
    unsafe {
        core::arch::x86_64::_mm_prefetch(ptr.cast::<i8>(), core::arch::x86_64::_MM_HINT_T0);
    }
}
