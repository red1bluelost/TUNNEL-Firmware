use crate::{st7580, usb};
use stm32f4xx_hal::timer::{self, DelayUs, ExtU32};

pub mod follower;
pub mod leader;

pub use follower::Follower;
pub use leader::Leader;

enum Header {
    Ack = 0x00,
    Nack = 0x01,
    Idle = 0x02,
    Send = 0x03,
    Data = 0x04,
}

impl Into<u8> for Header {
    fn into(self) -> u8 {
        self as u8
    }
}

impl TryFrom<u8> for Header {
    type Error = u8;

    fn try_from(v: u8) -> Result<Self, Self::Error> {
        use Header::*;
        match v {
            0x00 => Ok(Ack),
            0x01 => Ok(Nack),
            0x02 => Ok(Idle),
            0x03 => Ok(Send),
            0x04 => Ok(Data),
            v => Err(v),
        }
    }
}

struct Channels {
    in_producer: usb::UsbProducer,
    out_consumer: usb::UsbConsumer,
}

fn shared_init<TIM: timer::Instance>(
    delay: &mut DelayUs<TIM>,
    driver: &mut st7580::Driver,
    sender: &mut st7580::DSender,
) {
    driver.init(delay);

    driver
        .mib_write(st7580::MIB_MODEM_CONF, &st7580::MODEM_CONFIG)
        .and_then(|tag| sender.enqueue(tag))
        .and_then(|d| nb::block!(d.process()))
        .unwrap();
    delay.delay(500.millis());

    driver
        .mib_write(st7580::MIB_PHY_CONF, &st7580::PHY_CONFIG)
        .and_then(|tag| sender.enqueue(tag))
        .and_then(|d| nb::block!(d.process()))
        .unwrap();
    delay.delay(500.millis());
}
