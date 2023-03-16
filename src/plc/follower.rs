use super::{Channels, Header, DATA_OPT, DATA_START, HEADER_IDX};
use crate::{mem, st7580, usb};
use stm32f4xx_hal::timer::{self, DelayUs};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum State {
    Wait,
    Send,
}

pub struct Follower {
    state: State,
    driver: st7580::Driver,
    sender: st7580::DSender,
    channels: Channels,
}

impl Follower {
    pub fn new(
        driver: st7580::Driver,
        sender: st7580::DSender,
        in_producer: usb::UsbProducer,
        out_consumer: usb::UsbConsumer,
    ) -> Self {
        Self {
            state: State::Wait,
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
        match self.state {
            State::Wait => {
                let Some(f) = self.driver.receive_frame() else { return };
                debug_assert!(matches!(f.stx, st7580::STX_03 | st7580::STX_02));
                let header = f.data[HEADER_IDX].try_into().unwrap();
                match header {
                    Header::Idle => panic!("Unexpected Idle from leader"),
                    Header::Data => {
                        let len = f.length as usize;
                        let mut data = f.data;
                        data.copy_within(DATA_START..len, 0);
                        data.truncate(len - DATA_START);
                        self.channels.in_producer.enqueue(data).unwrap();
                    }
                    Header::Ping => {
                        let receive_opt = self.channels.out_consumer.dequeue();
                        let send_buf = match receive_opt {
                            Some(mut send_buf) => {
                                send_buf.push(Header::Data.into()).unwrap();
                                send_buf.rotate_right(1);
                                send_buf
                            }
                            None => {
                                mem::alloc_from_slice(&[Header::Idle.into()])
                                    .unwrap()
                            }
                        };
                        let tag =
                            self.driver.dl_data(DATA_OPT, send_buf).unwrap();
                        self.sender.enqueue(tag).unwrap();
                        self.state = State::Send;
                    }
                }
            }
            State::Send => match self.sender.process() {
                Ok(()) => self.state = State::Wait,
                Err(st7580::NbStErr::WouldBlock) => {}
                Err(st7580::NbStErr::Other(e)) => {
                    panic!("Ping processing error: {:?}", e)
                }
            },
        }
    }
}
