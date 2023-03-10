#![no_main]
#![no_std]

#[rtic::app(
    device = hal::pac,
    peripherals = true,
    dispatchers = [SPI1, SPI2, SPI3]
)]
mod app {
    use hal::otg_fs::{UsbBus, UsbBusType, USB};
    use hal::{
        pac,
        prelude::*,
        timer::{self, DelayUs},
    };
    use heapless::pool::singleton::Pool;
    use stm32f4xx_hal as hal;
    use usb_device::{bus::UsbBusAllocator, prelude::*};

    #[cfg(feature = "LEADER")]
    use plc::Follower as PlcDriver;
    #[cfg(feature = "FOLLOWER")]
    use plc::Leader as PlcDriver;
    use tunnel_firmware::{dbg, mem, plc, st7580, usb, util};

    #[shared]
    struct Shared {}

    #[local]
    struct Local {
        usb_device: UsbDevice<'static, UsbBusType>,
        usb_manager: usb::UsbManager,
        st7580_interrupt_handler: st7580::InterruptHandler,
        delay: DelayUs<pac::TIM3>,
        driver: PlcDriver,
    }

    #[monotonic(binds = TIM2, default = true)]
    type MicrosecMono = timer::MonoTimerUs<pac::TIM2>;

    #[init(
        local = [
            stbuf: [u8; 1 << 12] = util::zeros(),
            ep_memory: [u32; 1024] = util::zeros(),
            usb_bus: Option<UsbBusAllocator<UsbBusType>> = None,
        ]
    )]
    fn init(ctx: init::Context) -> (Shared, Local, init::Monotonics) {
        dbg::init!();
        dbg::println!("init");

        let init::LocalResources {
            stbuf,
            ep_memory,
            usb_bus,
        } = ctx.local;

        let dp = ctx.device;

        let rcc = dp.RCC.constrain();
        let clocks = rcc.cfgr.use_hse(8.MHz()).sysclk(96.MHz()).freeze();

        let mono = dp.TIM2.monotonic_us(&clocks);

        let gpioa = dp.GPIOA.split();
        let gpioc = dp.GPIOC.split();

        mem::POOL::grow(stbuf);

        let (st7580_driver, st7580_dsender, st7580_interrupt_handler) =
            st7580::Builder {
                t_req: gpioa.pa5.into_push_pull_output(),
                resetn: gpioa.pa8.into_push_pull_output(),
                tx_on: gpioc.pc0,
                rx_on: gpioc.pc1,
                usart: dp.USART1,
                usart_tx: gpioa.pa9.into_alternate(),
                usart_rx: gpioa.pa10.into_alternate(),
                now: monotonics::now,
            }
            .split(&clocks);
        let delay = dp.TIM3.delay(&clocks);

        let usb = USB {
            usb_global: dp.OTG_FS_GLOBAL,
            usb_device: dp.OTG_FS_DEVICE,
            usb_pwrclk: dp.OTG_FS_PWRCLK,
            pin_dm: gpioa.pa11.into_alternate(),
            pin_dp: gpioa.pa12.into_alternate(),
            hclk: clocks.hclk(),
        };

        usb_bus.replace(UsbBus::new(usb, ep_memory));
        let usb_bus = usb_bus.as_mut().unwrap();

        let usb::UsbSplit {
            usb_manager,
            in_producer,
            out_consumer,
        } = usb::split(usb_bus);

        let usb_device =
            UsbDeviceBuilder::new(usb_bus, UsbVidPid(0x0000, 0x6969))
                .manufacturer("TUNNEL Team")
                .product("TUNNEL Device")
                .serial_number("deadbeef")
                .self_powered(true)
                .build();

        let driver = PlcDriver::new(
            st7580_driver,
            st7580_dsender,
            in_producer,
            out_consumer,
        );

        plm::spawn().unwrap();

        dbg::println!("init end");
        (
            Shared {},
            Local {
                usb_device,
                usb_manager,
                st7580_interrupt_handler,
                delay,
                driver,
            },
            init::Monotonics(mono),
        )
    }

    #[task(
        binds = OTG_FS,
        priority = 2,
        local = [
            usb_device,
            usb_manager,
        ]
    )]
    fn otg_fs(ctx: otg_fs::Context) {
        let otg_fs::LocalResources {
            usb_device,
            usb_manager,
        } = ctx.local;

        if !usb_device.poll(&mut [usb_manager.class()]) {
            return;
        }

        usb_manager.poll().unwrap();
    }

    #[task(
        priority = 1,
        local = [
            delay,
            driver,
            should_init: bool = true
        ]
    )]
    fn plm(ctx: plm::Context) {
        let plm::LocalResources {
            delay,
            driver,
            should_init,
        } = ctx.local;

        // We must perform the initialization stage here due to the `init`
        // task being interrupt free.
        if *should_init {
            dbg::println!("plm init");
            driver.init(delay);
            *should_init = false;
        }

        driver.process();

        plm::spawn().unwrap();
    }

    #[task(binds = USART1, priority = 2, local = [st7580_interrupt_handler])]
    fn usart1(ctx: usart1::Context) {
        ctx.local.st7580_interrupt_handler.handle();
    }
}
