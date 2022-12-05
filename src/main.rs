#![deny(unsafe_code)]
#![no_main]
#![no_std]

use panic_rtt_target as _;

#[rtic::app(device = hal::pac, peripherals = true, dispatchers = [SPI1])]
mod app {
    use cortex_m_semihosting::{debug, hprintln};
    use rtt_target::{rprintln, rtt_init_print};
    use stm32f4xx_hal as hal;
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

        hprintln!("Hello, world!");

        // exit QEMU
        // NOTE do not run this on hardware; it can corrupt OpenOCD state
        debug::exit(debug::EXIT_SUCCESS);

        (Shared {}, Local {}, init::Monotonics(mono))
    }
}
