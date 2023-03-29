#[macro_export(crate)]
macro_rules! init {
    () => {
        #[cfg(feature = "RTT")]
        rtt_target::rtt_init_print!();
    };
}

#[macro_export(crate)]
macro_rules! print {
    ($($arg:tt)*) => {
        #[cfg(feature = "RTT")]
        rtt_target::rprint!($($arg)*);

        #[cfg(feature = "QEMU")]
        cortex_m_semihosting::hprint!($($arg)*);
    };
}

#[macro_export(crate)]
macro_rules! println {
    ($($arg:tt)*) => {
        #[cfg(feature = "RTT")]
        rtt_target::rprintln!($($arg)*);

        #[cfg(feature = "QEMU")]
        cortex_m_semihosting::hprintln!($($arg)*);
    };
}

pub fn dump_buffer(buffer: &[u8]) {
    for (cnk_idx, cnk) in buffer.chunks(16).enumerate() {
        print!("{:08x}:", cnk_idx);
        for bit_idx in 0..16 {
            if bit_idx % 2 == 0 {
                print!(" ");
            }
            match cnk.get(bit_idx) {
                Some(&e) => {
                    print!("{:02x}", e);
                }
                None => {
                    print!("  ");
                }
            }
        }
        print!("  ");
        for &c in cnk {
            if c.is_ascii() && !c.is_ascii_control() {
                print!("{}", c as char);
            } else {
                print!(" ");
            }
        }
        println!();
    }
}

pub use {init, print, println};
