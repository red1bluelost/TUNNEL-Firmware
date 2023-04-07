use crate::{st7580, usb};
use stm32f4xx_hal::timer::{self, DelayUs};

pub mod follower;
pub mod leader;

pub use follower::Follower;
pub use leader::Leader;

const PLM_SPACE_USED: usize = 4 + if cfg!(feature = "GAIN_SELECTOR") {
    1
} else {
    0
};
const HEADER_IDX: usize = PLM_SPACE_USED;
const DATA_START: usize = HEADER_IDX + 1;

/// 0 -
const DATA_OPT: u8 = 0b0_010_0_1_0_0;

enum Header {
    Idle = 0x00,
    Data = 0x01,
    Ping = 0x02,
}

impl From<Header> for u8 {
    fn from(val: Header) -> Self {
        val as u8
    }
}

impl TryFrom<u8> for Header {
    type Error = u8;

    fn try_from(v: u8) -> Result<Self, Self::Error> {
        use Header::*;
        match v {
            0x00 => Ok(Idle),
            0x01 => Ok(Data),
            0x02 => Ok(Ping),
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

    driver
        .mib_write(st7580::MIB_PHY_CONF, &st7580::PHY_CONFIG)
        .and_then(|tag| sender.enqueue(tag))
        .and_then(|d| nb::block!(d.process()))
        .unwrap();
    driver.set_ready_to_receive();
}
