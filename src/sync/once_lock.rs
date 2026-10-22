use core::{
    cell::UnsafeCell,
    mem::MaybeUninit,
    sync::atomic::{AtomicBool, Ordering},
};

pub struct OnceLock<T> {
    flag: AtomicBool,
    value: UnsafeCell<MaybeUninit<T>>,
}

unsafe impl<T> Send for OnceLock<T> {}
unsafe impl<T> Sync for OnceLock<T> {}

impl<T> OnceLock<T> {
    pub const fn new() -> Self {
        Self {
            flag: AtomicBool::new(false),
            value: UnsafeCell::new(MaybeUninit::uninit()),
        }
    }
    pub fn get_or_init<F>(&self, f: F) -> &T
    where
        F: FnOnce() -> T,
    {
        // todo: for now use relaxed if we need SMP then add proper acquire and release
        if self.flag.load(Ordering::Relaxed) == false {
            // init the value and return reference
            let ptr = self.value.get();
            unsafe {
                (*ptr).write(f());
            }
            self.flag.store(true, Ordering::Relaxed);
        }

        return unsafe { self.value.as_ref_unchecked().assume_init_ref() };
    }
}
