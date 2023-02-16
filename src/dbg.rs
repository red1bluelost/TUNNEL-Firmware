macro_rules! init {
    () => {
        #[cfg(feature = "RTT")]
        rtt_target::rtt_init_print!();
    };
}

macro_rules! println {
    ($($arg:tt)*) => {
        #[cfg(feature = "RTT")]
        rtt_target::rprintln!($($arg)*);

        #[cfg(feature = "QEMU")]
        cortex_m_semihosting:: hprintln!($($arg)*);
    };
}

macro_rules! exit {
    () => {
        #[cfg(feature = "QEMU")]
        cortex_m_semihosting::debug::exit(debug::EXIT_SUCCESS);
    };
}

pub(crate) use {exit, init, println};