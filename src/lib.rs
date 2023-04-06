#![no_std]

#[cfg(feature = "HALT")]
pub use panic_halt as _;
#[cfg(feature = "RTT")]
pub use panic_probe as _;
#[cfg(feature = "QEMU")]
pub use panic_semihosting as _;

pub mod dbg;
pub mod mem;
pub mod plc;
pub mod st7580;
pub mod usb;
pub mod util;

#[cfg(feature = "F446")]
pub const CLOCK_SPEED: u32 = 144;
#[cfg(feature = "F411")]
pub const CLOCK_SPEED: u32 = 96;
