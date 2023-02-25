#![no_std]

pub mod dbg;
pub mod mem;
pub mod st7580;

#[cfg(feature = "RTT")]
pub use panic_probe as _;

#[cfg(feature = "HALT")]
pub use panic_halt as _;

#[cfg(feature = "QEMU")]
pub use panic_semihosting as _;
