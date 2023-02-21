#![no_main]
#![no_std]

#[cfg(feature = "RTT")]
use panic_rtt_target as _;

#[cfg(feature = "HALT")]
use panic_halt as _;

#[cfg(feature = "QEMU")]
use panic_semihosting as _;

mod dbg;
pub mod signal;
pub mod st7580;

#[rtic::app(
    device = hal::pac,
    peripherals = true,
    dispatchers = [SPI1, SPI2, SPI3]
)]
mod app {
    use crate::dbg;
    use crate::st7580;
    use hal::{
        gpio::*,
        otg_fs::{UsbBus, UsbBusType, USB},
        pac,
        prelude::*,
        timer,
    };
    use stm32f4xx_hal as hal;
    use usb_device::prelude::*;
    use usbd_serial::SerialPort;

    #[shared]
    struct Shared {}

    #[local]
    struct Local {
        usb_dev: UsbDevice<'static, UsbBusType>,
        usb_comm: SerialPort<'static, UsbBusType>,
        st7580_interrupt_handler: st7580::InterruptHandler,
        st7580_driver: st7580::Driver,
    }

    #[monotonic(binds = TIM2, default = true)]
    type MicrosecMono = timer::MonoTimerUs<pac::TIM2>;

    #[init]
    fn init(ctx: init::Context) -> (Shared, Local, init::Monotonics) {
        dbg::init!();
        dbg::println!("init");

        static mut EP_MEMORY: [u32; 1024] = [0; 1024];

        let dp = ctx.device;

        let rcc = dp.RCC.constrain();
        let clocks = rcc.cfgr.use_hse(8.MHz()).sysclk(100.MHz()).freeze();

        let mono = dp.TIM2.monotonic_us(&clocks);

        let gpioa = dp.GPIOA.split();
        let gpioc = dp.GPIOC.split();

        let (st7580_driver, st7580_interrupt_handler) = st7580::Builder {
            t_req: gpioa.pa5.into_push_pull_output(),
            resetn: gpioa.pa8.into_push_pull_output(),
            tx_on: gpioc.pc0,
            rx_on: gpioc.pc1,
            usart: dp.USART1,
            usart_tx: gpioa.pa9.into_alternate(),
            usart_rx: gpioa.pa10.into_alternate(),
            tim3: dp.TIM3,
            tim5: dp.TIM5,
        }
        .split(&clocks);

        let usb = USB {
            usb_global: dp.OTG_FS_GLOBAL,
            usb_device: dp.OTG_FS_DEVICE,
            usb_pwrclk: dp.OTG_FS_PWRCLK,
            pin_dm: gpioa.pa11.into_alternate(),
            pin_dp: gpioa.pa12.into_alternate(),
            hclk: clocks.hclk(),
        };

        static mut USB_BUS: Option<
            usb_device::bus::UsbBusAllocator<UsbBusType>,
        > = None;
        unsafe { USB_BUS.replace(UsbBus::new(usb, &mut EP_MEMORY)) };
        let usb_bus = unsafe { USB_BUS.as_ref() }.unwrap();
        let usb_comm = SerialPort::new(usb_bus);
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
                st7580_interrupt_handler,
                st7580_driver,
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

    #[task(priority = 1, local = [st7580_driver, should_init: bool = true])]
    fn plm(ctx: plm::Context) {
        let plm::LocalResources {
            st7580_driver: driver,
            should_init,
        } = ctx.local;

        // We must perform the initialization stage here due to the `init`
        // task being interrupt free.
        if *should_init {
            dbg::println!("plm init");
            driver.init();

            driver
                .mib_write(st7580::MIB_MODEM_CONF, &st7580::MODEM_CONFIG)
                .unwrap();
            driver.delay.delay(500.millis());

            driver
                .mib_write(st7580::MIB_PHY_CONF, &st7580::PHY_CONFIG)
                .unwrap();
            driver.delay.delay(500.millis());

            *should_init = false;
        }

        plm::spawn_after(1.secs()).unwrap();
    }

    #[task(binds = USART1, priority = 2, local = [st7580_interrupt_handler])]
    fn usart1(ctx: usart1::Context) {
        ctx.local.st7580_interrupt_handler.handle();
    }
}
