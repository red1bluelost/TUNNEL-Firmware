#![no_main]
#![no_std]

use stm32f4xx_hal::otg_fs::UsbBusType;
use tunnel_firmware::dbg;
use usb_device::{bus::UsbBusAllocator, class_prelude::*};
use usbd_serial::{Result, SerialPort};

#[rtic::app(
    device = hal::pac,
    peripherals = true,
    dispatchers = [SPI1, SPI2, SPI3]
)]
mod app {
    use super::SerialPortTrace;
    use hal::otg_fs::{UsbBus, UsbBusType, USB};
    use hal::{pac, prelude::*, timer};
    use stm32f4xx_hal as hal;
    use usb_device::{bus::UsbBusAllocator, prelude::*};
    use usbd_serial::{self};

    use tunnel_firmware::{dbg, util};

    #[shared]
    struct Shared {}

    #[local]
    struct Local {
        usb_dev: UsbDevice<'static, UsbBusType>,
        usb_comm: SerialPortTrace,
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
        let usb_comm = SerialPortTrace::new(usb_bus);
        let usb_dev = UsbDeviceBuilder::new(usb_bus, UsbVidPid(0x0000, 0x6968))
            .manufacturer("TUNNEL Team")
            .product("TUNNEL Device")
            .serial_number("deadbeef")
            // .device_class(0x02)
            // .device_sub_class(0x0c)
            // .device_protocol(0x07)
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
            counter: u64 = 0,
        ]
    )]
    fn usb(ctx: usb::Context) {
        let usb::LocalResources {
            usb_dev,
            usb_comm,
            buffer,
            counter,
        } = ctx.local;

        if !usb_dev.poll(&mut [usb_comm]) {
            // return;
        }

        match usb_comm.read(buffer) {
            Ok(count) => {
                dbg::println!("received {} bytes", count);
                dbg::dump_buffer(&buffer[..count]);
                dbg::println!("dump over\n");
            }
            Err(usbd_serial::UsbError::WouldBlock) => {
                // dbg::println!("attempting read {}", counter);
                *counter += 1;
            }
            Err(err) => {
                dbg::println!("USB read failed: {:?}", err);
            }
        }

        #[cfg(any())]
        {
            let tmp = "yeet".as_bytes();
            buffer[..tmp.len()].clone_from_slice(tmp);
            match usb_comm.write(&buffer[..tmp.len()]) {
                Ok(count) => {
                    dbg::println!("wrote {} bytes", count);
                    dbg::dump_buffer(&buffer[..count]);
                    dbg::println!("dump over\n");
                }
                Err(usbd_serial::UsbError::WouldBlock) => {}
                Err(err) => {
                    dbg::println!("USB write failed: {:?}", err);
                }
            }
        }
    }
}
pub struct SerialPortTrace(SerialPort<'static, UsbBusType>);

impl SerialPortTrace {
    pub fn new(alloc: &'static UsbBusAllocator<UsbBusType>) -> Self {
        SerialPortTrace(SerialPort::new(alloc))
    }

    pub fn read(&mut self, data: &mut [u8]) -> Result<usize> {
        self.0.read(data)
    }

    pub fn write(&mut self, data: &[u8]) -> Result<usize> {
        self.0.write(data)
    }
}

impl UsbClass<UsbBusType> for SerialPortTrace {
    fn get_configuration_descriptors(
        &self,
        writer: &mut DescriptorWriter,
    ) -> Result<()> {
        dbg::println!("Calling get_configuration_descriptors");
        self.0.get_configuration_descriptors(writer)
    }

    fn get_bos_descriptors(&self, writer: &mut BosWriter) -> Result<()> {
        dbg::println!("Calling get_bos_descriptors");
        self.0.get_bos_descriptors(writer)
    }

    fn control_in(&mut self, xfer: ControlIn<UsbBusType>) {
        let req = xfer.request();
        dbg::println!("Control In for someone {:?}", req);
        self.0.control_in(xfer)
    }

    fn control_out(&mut self, xfer: ControlOut<UsbBusType>) {
        let req = xfer.request();
        dbg::println!("Control Out for someone {:?}", req);
        self.0.control_out(xfer)
    }

    fn endpoint_setup(&mut self, addr: EndpointAddress) {
        // dbg::println!("Endpoint setup for {:?}", addr);
        self.0.endpoint_setup(addr)
    }

    fn endpoint_out(&mut self, addr: EndpointAddress) {
        // dbg::println!("Endpoint out for {:?}", addr);
        self.0.endpoint_out(addr)
    }

    fn endpoint_in_complete(&mut self, addr: EndpointAddress) {
        // dbg::println!("Endpoint in complete for {:?}", addr);
        self.0.endpoint_in_complete(addr)
    }

    fn get_string(&self, index: StringIndex, lang_id: u16) -> Option<&str> {
        // dbg::println!("Get String for {:?} {:?}", u8::from(index), lang_id);
        self.0.get_string(index, lang_id)
    }

    fn poll(&mut self) {
        // dbg::println!("poll");
        self.0.poll()
    }

    fn reset(&mut self) {
        // dbg::println!("USB Reset");
        self.0.reset()
    }
}
