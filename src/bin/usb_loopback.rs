#![no_main]
#![no_std]

#[rtic::app(
    device = hal::pac,
    peripherals = true,
    dispatchers = [SPI1, SPI2, SPI3]
)]
mod app {
    use hal::otg_fs::{UsbBus, UsbBusType, USB};
    use hal::{pac, prelude::*, timer};
    use stm32f4xx_hal as hal;
    use usb_device::{bus::UsbBusAllocator, prelude::*};
    use usbd_serial::{self, SerialPort};

    use tunnel_firmware::{dbg, util};

    const USB_BUF_SIZE: usize = 512;
    type UsbBuf = [u8; USB_BUF_SIZE];

    #[shared]
    struct Shared {}

    #[local]
    struct Local {
        usb_dev: UsbDevice<'static, UsbBusType>,
        usb_comm: SerialPort<'static, UsbBusType, UsbBuf, UsbBuf>,
    }

    #[monotonic(binds = TIM2, default = true)]
    type MicrosecMono = timer::MonoTimerUs<pac::TIM2>;

    #[init(
        local = [
            ep_memory: [u32; 1024] = util::zeros(),
            usb_bus: Option<UsbBusAllocator<UsbBusType>> = None,
        ]
    )]
    fn init(ctx: init::Context) -> (Shared, Local, init::Monotonics) {
        dbg::init!();
        dbg::println!("init");

        let init::LocalResources { ep_memory, usb_bus } = ctx.local;

        let dp = ctx.device;

        let rcc = dp.RCC.constrain();
        let clocks = rcc.cfgr.use_hse(8.MHz()).sysclk(96.MHz()).freeze();

        let mono = dp.TIM2.monotonic_us(&clocks);

        let gpioa = dp.GPIOA.split();

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
            .device_class(usbd_serial::USB_CLASS_CDC)
            .self_powered(true)
            .build();

        dbg::println!("init end");
        (
            Shared {},
            Local { usb_dev, usb_comm },
            init::Monotonics(mono),
        )
    }

    #[task(
        binds = OTG_FS,
        priority = 2,
        local = [
            usb_dev,
            usb_comm,
            buffer: [u8; 1 << 12] = util::zeros(),
        ]
    )]
    fn usb(ctx: usb::Context) {
        let usb::LocalResources {
            usb_dev,
            usb_comm,
            buffer,
        } = ctx.local;

        if !usb_dev.poll(&mut [usb_comm]) {
            return;
        }

        let count = match usb_comm.read(buffer) {
            Ok(count) => {
                dbg::println!("received {} bytes", count);
                dbg::dump_buffer(&buffer[..count]);
                dbg::println!("dump over\n");
                count
            }
            Err(usbd_serial::UsbError::WouldBlock) => 0,
            Err(err) => {
                dbg::println!("USB read failed: {:?}", err);
                0
            }
        };

        match usb_comm.write(&buffer[..count]) {
            Ok(count) => {
                dbg::println!("wrote {} bytes", count);
                dbg::dump_buffer(&buffer[..count]);
                dbg::println!("dump over\n");
            }
            Err(usbd_serial::UsbError::WouldBlock) => {}
            Err(err) => {
                dbg::println!("USB read failed: {:?}", err);
            }
        }
    }
}
