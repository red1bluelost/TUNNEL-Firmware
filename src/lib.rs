#![no_std]

pub mod dbg;
pub mod signal;
pub mod st7580;

#[cfg(feature = "RTT")]
pub use panic_rtt_target as _;

#[cfg(feature = "HALT")]
pub use panic_halt as _;

#[cfg(feature = "QEMU")]
pub use panic_semihosting as _;
