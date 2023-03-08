mod header;

use usb_device::{class_prelude::*, Result};

/// Communication Class Code for Communication Device Class
pub const USB_CDC_CODE: u8 = 0x02;

/// Communication Subclass code for EEM
pub const CDC_SUBCLASS_EEM: u8 = 0x0c;

/// Communication Class Protocol code for EEM
pub const CDC_PROTOCOL_EEM: u8 = 0x07;

///
const CDC_DATA_IF_CLASS: u8 = 0x0a;

const CDC_DATA_IF_SUBCLASS: u8 = 0x00;

const CDC_DATA_IF_PROTOCOL_NONE: u8 = 0x00;

const CS_INTERFACE: u8 = 0x24;
const CDC_TYPE_HEADER: u8 = 0x00;
const CDC_TYPE_CALL_MANAGEMENT: u8 = 0x01;
const CDC_TYPE_UNION: u8 = 0x06;

const REQ_SEND_ENCAPSULATED_COMMAND: u8 = 0x00;
#[allow(unused)]
const REQ_GET_ENCAPSULATED_COMMAND: u8 = 0x01;

pub struct CdcEemClass<'a, B: UsbBus> {
    // communication_interface: InterfaceNumber,
    // communication_endpoint: EndpointIn<'a, B>,
    data_interface: InterfaceNumber,
    pub read_endpoint: EndpointOut<'a, B>,
    pub write_endpoint: EndpointIn<'a, B>,
    pub enable: bool,
}

impl<B: UsbBus> CdcEemClass<'_, B> {
    pub fn new(alloc: &UsbBusAllocator<B>) -> CdcEemClass<'_, B> {
        let max_packet_size = 4;
        let tmp = CdcEemClass {
            // communication_interface: alloc.interface(),
            // communication_endpoint: alloc.interrupt(8, 255),
            data_interface: alloc.interface(),
            read_endpoint: alloc.bulk(max_packet_size),
            write_endpoint: alloc.bulk(max_packet_size),
            enable: false,
        };
        // crate::dbg::println!(
        //     "eem class comm = {:?} data = {:?} ce = {:?} re = {:?} we = {:?}\n",
        //     u8::from(tmp.communication_interface),
        //     u8::from(tmp.data_interface),
        //     tmp.communication_endpoint.address(),
        //     tmp.read_endpoint.address(),
        //     tmp.write_endpoint.address(),
        // );
        crate::dbg::println!(
            "eem class data = {:?} re = {:?} we = {:?}\n",
            u8::from(tmp.data_interface),
            tmp.read_endpoint.address(),
            tmp.write_endpoint.address(),
        );
        tmp
    }

    fn max_packet_size(&self) -> u16 {
        self.read_endpoint.max_packet_size()
    }
}

impl<B: UsbBus> UsbClass<B> for CdcEemClass<'_, B> {
    fn get_configuration_descriptors(
        &self,
        writer: &mut DescriptorWriter,
    ) -> Result<()> {
        crate::dbg::println!("Calling get_configuration_descriptors");

        // writer.iad(
        //     self.communication_interface,
        //     2,
        //     0xff,
        //     0xff,
        //     CDC_PROTOCOL_EEM,
        // )?;
        // writer.interface(
        //     self.communication_interface,
        //     0xff,
        //     0xff,
        //     0xff,
        // )?;

        // writer.write(
        //     CS_INTERFACE,
        //     &[
        //         CDC_TYPE_UNION,                      // bDescriptorSubtype
        //         self.communication_interface.into(), // bControlInterface
        //         self.data_interface.into(),          // bSubordinateInterface
        //     ],
        // )?;
        // writer.write(
        //     CS_INTERFACE,
        //     &[
        //         CDC_TYPE_CALL_MANAGEMENT,   // bDescriptorSubtype
        //         0x00,                       // bmCapabilities
        //         self.data_interface.into(), // bDataInterface
        //     ],
        // )?;
        // writer.endpoint(&self.communication_endpoint)?;

        writer.interface(
            self.data_interface,
            CDC_DATA_IF_CLASS,
            CDC_DATA_IF_SUBCLASS,
            CDC_DATA_IF_PROTOCOL_NONE,
        )?;
        writer.endpoint(&self.read_endpoint)?;
        writer.endpoint(&self.write_endpoint)?;
        writer.write(
            CS_INTERFACE,
            &[
                CDC_TYPE_HEADER, // bDescriptorSubtype
                0x10,
                0x01, // bcdCDC (1.10)
            ],
        )?;
        writer.write(
            CS_INTERFACE,
            &[
                CDC_TYPE_CALL_MANAGEMENT,   // bDescriptorSubtype
                0x00,                       // bmCapabilities
                self.data_interface.into(), // bDataInterface
            ],
        )?;
        Ok(())
    }

    fn get_bos_descriptors(&self, writer: &mut BosWriter) -> Result<()> {
        Ok(())
    }

    fn control_in(&mut self, xfer: ControlIn<B>) {
        let req = xfer.request();
        crate::dbg::println!("Control In for someone {:?}", req);

        // if !(req.request_type == control::RequestType::Class
        //     && req.recipient == control::Recipient::Interface
        //     && req.index == u8::from(self.communication_interface) as u16)
        // {
        //     return;
        // }

        // crate::dbg::println!("Control In for me?");
    }

    fn control_out(&mut self, xfer: ControlOut<B>) {
        let req = xfer.request();
        crate::dbg::println!("Control Out for someone {:?}", req);

        if (req.request_type == control::RequestType::Standard
            && req.recipient == control::Recipient::Interface
            && req.request == 11)
        {
            crate::dbg::println!("Enabling usb");
            xfer.accept().unwrap();
            self.enable = true;
            return;
        }

        if !(req.request_type == control::RequestType::Class
            && req.recipient == control::Recipient::Interface
            && req.index == u8::from(self.data_interface) as u16)
        {
            return;
        }

        // crate::dbg::println!("Control Out for me?");

        // match req.request {
        //     REQ_SEND_ENCAPSULATED_COMMAND => {
        //         // We don't actually support encapsulated commands but pretend
        //         // we do for standards compatibility.
        //         xfer.accept().ok();
        //     }
        //     _ => {}
        // }
        // xfer.reject().ok();
    }

    fn endpoint_setup(&mut self, addr: EndpointAddress) {
        crate::dbg::println!("Endpoint setup for {:?}", addr);
    }

    fn endpoint_out(&mut self, addr: EndpointAddress) {
        crate::dbg::println!("Endpoint out for {:?}", addr);
    }

    fn endpoint_in_complete(&mut self, addr: EndpointAddress) {
        crate::dbg::println!("Endpoint in complete for {:?}", addr);
    }

    fn reset(&mut self) {
        crate::dbg::println!("USB Reset");
    }

    fn get_string(&self, index: StringIndex, lang_id: u16) -> Option<&str> {
        crate::dbg::println!(
            "Get String for {:?} {:?}",
            u8::from(index),
            lang_id
        );
        None
    }
}
