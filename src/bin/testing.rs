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
    use usbd_serial::SerialPort;

    use tunnel_firmware::{dbg, mem, st7580, util};

    const USB_BUF_SIZE: usize = 512;
    type UsbBuf = [u8; USB_BUF_SIZE];

    #[shared]
    struct Shared {}

    #[local]
    struct Local {
        usb_dev: UsbDevice<'static, UsbBusType>,
        usb_comm: SerialPort<'static, UsbBusType, UsbBuf, UsbBuf>,
        delay: DelayUs<pac::TIM3>,
        st7580_interrupt_handler: st7580::InterruptHandler,
        st7580_driver: st7580::Driver,
        st7580_dsender: st7580::DSender,
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
        let clocks = rcc.cfgr.use_hse(8.MHz()).sysclk(100.MHz()).freeze();

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
        let usb_comm =
            SerialPort::new_with_store(usb_bus, util::zeros(), util::zeros());
        let usb_dev = UsbDeviceBuilder::new(usb_bus, UsbVidPid(0x0000, 0x6969))
            .manufacturer("TUNNEL Team")
            .product("TUNNEL Device")
            .serial_number("deadbeef")
            .device_class(0xff)
            .self_powered(true)
            .build();

        plm::spawn().unwrap();

        dbg::println!("init end");
        (
            Shared {},
            Local {
                usb_dev,
                usb_comm,
                delay,
                st7580_interrupt_handler,
                st7580_driver,
                st7580_dsender,
            },
            init::Monotonics(mono),
        )
    }

    #[task(binds = OTG_FS, priority = 2, local = [usb_dev, usb_comm])]
    fn usb(ctx: usb::Context) {
        let usb::LocalResources { usb_dev, usb_comm } = ctx.local;

        if !usb_dev.poll(&mut [usb_comm]) {
            return;
        }

        const DATA: &[u8] = "test string\n".as_bytes();
        match usb_comm.write(DATA) {
            Ok(count) => {
                dbg::println!(
                    "*** USB wrote {} bytes out of {}",
                    count,
                    DATA.len()
                );
            }
            Err(err) => {
                dbg::println!("USB write failed: {:?}", err);
            }
        }
    }

    #[task(
        priority = 1,
        local = [
            delay, st7580_driver, st7580_dsender, should_init: bool = true
        ]
    )]
    fn plm(ctx: plm::Context) {
        let plm::LocalResources {
            delay,
            st7580_driver: driver,
            st7580_dsender: dsender,
            should_init,
        } = ctx.local;

        // We must perform the initialization stage here due to the `init`
        // task being interrupt free.
        if *should_init {
            dbg::println!("plm init");
            driver.init(delay);

            dbg::println!("plm modem conf");
            driver
                .mib_write(st7580::MIB_MODEM_CONF, &st7580::MODEM_CONFIG)
                .and_then(|tag| dsender.enqueue(tag))
                .and_then(|d| nb::block!(d.process()))
                .unwrap();
            delay.delay(500.millis());

            dbg::println!("plm phy conf");
            driver
                .mib_write(st7580::MIB_PHY_CONF, &st7580::PHY_CONFIG)
                .and_then(|tag| dsender.enqueue(tag))
                .and_then(|d| nb::block!(d.process()))
                .unwrap();
            delay.delay(500.millis());

            *should_init = false;
        }

        let buf = mem::alloc_from_slice("hello st7580".as_bytes()).unwrap();
        driver
            .ping(buf)
            .and_then(|tag| dsender.enqueue(tag))
            .and_then(|d| nb::block!(d.process()))
            .unwrap();
        dbg::println!("successfully pinged the st7580");

        plm::spawn_after(5.secs()).unwrap();
    }

    #[task(binds = USART1, priority = 2, local = [st7580_interrupt_handler])]
    fn usart1(ctx: usart1::Context) {
        ctx.local.st7580_interrupt_handler.handle();
    }
}
