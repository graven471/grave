#[macro_export]
macro_rules! kprint {
    ($($arg:tt)*) => {
        $crate::drivers::serial::_print(
            core::format_args!($($arg)*)
        )
    };
}

#[macro_export]
macro_rules! kprintln {
    () => {
        $crate::kprint!("\n")
    };
    ($($arg:tt)*) => {
        $crate::kprint!("{}\n", core::format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! virt {
    ($addr:expr) => {
        $addr + $crate::memory::hhdm::offset()
    };
}

#[macro_export]
macro_rules! phys {
    ($addr:expr) => {
        $addr - $crate::memory::hhdm::offset()
    };
}
