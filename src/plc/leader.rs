use super::Channels;
use crate::{st7580, usb};
use stm32f4xx_hal::timer::{self, DelayUs};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum State {
    PingSend,
    WaitPing,
    Send,
    Receive,
}

pub struct Leader {
    state: State,
    driver: st7580::Driver,
    sender: st7580::DSender,
    channels: Channels,
}

impl Leader {
    pub fn new(
        driver: st7580::Driver,
        sender: st7580::DSender,
        in_producer: usb::UsbProducer,
        out_consumer: usb::UsbConsumer,
    ) -> Self {
        Self {
            state: State::PingSend,
            driver,
            sender,
            channels: Channels {
                in_producer,
                out_consumer,
            },
        }
    }

    pub fn init<TIM: timer::Instance>(&mut self, delay: &mut DelayUs<TIM>) {
        super::shared_init(delay, &mut self.driver, &mut self.sender)
    }

    pub fn process(&mut self) {
        todo!()
    }
}
