#[macro_export(crate)]
macro_rules! init {
    () => {
        #[cfg(feature = "RTT")]
        rtt_target::rtt_init_print!();
    };
}

#[macro_export(crate)]
macro_rules! println {
    ($($arg:tt)*) => {
        #[cfg(feature = "RTT")]
        rtt_target::rprintln!($($arg)*);

        #[cfg(feature = "QEMU")]
        cortex_m_semihosting:: hprintln!($($arg)*);
    };
}

pub use {init, println};
