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

    use tunnel_firmware::{dbg, usbd_ether, util};
    use usbd_ether::{
        CdcEemClass, CDC_PROTOCOL_EEM, CDC_SUBCLASS_EEM, USB_CDC_CODE,
    };

    #[shared]
    struct Shared {}

    #[local]
    struct Local {
        usb_dev: UsbDevice<'static, UsbBusType>,
        usb_comm: CdcEemClass<'static, UsbBusType>,
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
        let clocks = rcc.cfgr.use_hse(8.MHz()).sysclk(100.MHz()).freeze();

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
        let usb_comm = CdcEemClass::new(usb_bus);
        let usb_dev = UsbDeviceBuilder::new(usb_bus, UsbVidPid(0x0000, 0x6969))
            .manufacturer("TUNNEL Team")
            .product("TUNNEL Device")
            .serial_number("eem-v01")
            // .device_class(USB_CDC_CODE)
            // .device_sub_class(CDC_SUBCLASS_EEM)
            // .device_protocol(CDC_PROTOCOL_EEM)
            // .device_release(0x0110)
            // .max_packet_size_0(64)
            // .composite_with_iads()
            .self_powered(true)
            .build();

        // ping::spawn_after(1.secs()).unwrap();
        // usb::spawn().unwrap();

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
        // usb::spawn_after(1.millis()).unwrap();

        if !usb_dev.poll(&mut [usb_comm]) {
            return;
        }

        if !usb_comm.enable {
            return;
        }

        // usb_comm.read_endpoint.stall();
        let mut buf = [0; 64];
        match usb_comm.read_endpoint.read(&mut buf) {
            Ok(count) => {
                dbg::println!("poll read success of length {:?}", count);
                dbg::println!("{:?}", &buf[..count]);
            }
            Err(UsbError::WouldBlock) => {}
            Err(e) => {
                dbg::println!("poll read error: {:?}", e);
            }
        }

        // match usb_comm.write_endpoint.write("yeet".as_bytes()) {
        //     Ok(count) => {
        //         dbg::println!("poll write success of length {:?}", count);
        //     }
        //     Err(UsbError::WouldBlock) => {}
        //     Err(e) => {
        //         dbg::println!("poll write error: {:?}", e);
        //     }
        // }
    }

    #[task]
    fn ping(_ctx: ping::Context) {
        rtic::pend(pac::interrupt::OTG_FS);
        ping::spawn_after(1.millis()).unwrap();
    }
}
