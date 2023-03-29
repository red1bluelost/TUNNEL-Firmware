#![no_main]
#![no_std]

#[rtic::app(
    device = hal::pac,
    peripherals = true,
    dispatchers = [SPI1, SPI2, SPI3]
)]
mod app {
    use hal::{
        gpio::{Output, PA5},
        pac,
        prelude::*,
        timer,
    };
    use stm32f4xx_hal as hal;

    use tunnel_firmware::dbg;

    #[shared]
    struct Shared {}

    #[local]
    struct Local {
        led: PA5<Output>,
    }

    #[monotonic(binds = TIM2, default = true)]
    type MicrosecMono = timer::MonoTimerUs<pac::TIM2>;

    #[init]
    fn init(ctx: init::Context) -> (Shared, Local, init::Monotonics) {
        dbg::init!();
        dbg::println!("init");

        let dp = ctx.device;

        let rcc = dp.RCC.constrain();
        let clocks = rcc.cfgr.use_hse(8.MHz()).sysclk(96.MHz()).freeze();

        let mono = dp.TIM2.monotonic_us(&clocks);

        // Configure PA5 pin to blink LED
        let gpioa = dp.GPIOA.split();
        let mut led = gpioa.pa5.into_push_pull_output();
        led.set_high(); // Turn off

        blink::spawn().unwrap();

        dbg::println!("init end");
        (Shared {}, Local { led }, init::Monotonics(mono))
    }

    #[task(local = [led])]
    fn blink(ctx: blink::Context) {
        let led = ctx.local.led;
        if led.is_set_high() {
            led.set_low();
        } else {
            led.set_high();
        }
        blink::spawn_after(1000.millis()).unwrap();
    }
}
