#![deny(unsafe_code)]
#![no_main]
#![no_std]

use panic_rtt_target as _;
use rtic::app;

#[app(device = hal::pac, peripherals = true, dispatchers = [SPI1])]
mod app {
    use stm32f4xx_hal as hal;

    use rtt_target::{rprintln, rtt_init_print};
    use systick_monotonic::Systick;

    #[shared]
    struct Shared {}

    #[local]
    struct Local {}

    #[monotonic(binds = SysTick, default = true)]
    type Tonic = Systick<1000>;

    #[init]
    fn init(ctx: init::Context) -> (Shared, Local, init::Monotonics) {
        rtt_init_print!();
        rprintln!("init");

        let mono = Systick::new(ctx.core.SYST, 48_000_000);

        (Shared {}, Local {}, init::Monotonics(mono))
    }
}
