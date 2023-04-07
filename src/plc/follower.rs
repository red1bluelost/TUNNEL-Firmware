use super::{Channels, Header, DATA_OPT, DATA_START, HEADER_IDX};
use crate::{mem, st7580, usb};
use stm32f4xx_hal::timer::{self, DelayUs};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum State {
    Wait,
    Send,
}

pub struct Follower<const TWO_WAY: bool> {
    state: State,
    driver: st7580::Driver,
    sender: st7580::DSender,
    channels: Channels,
}

impl<const TWO_WAY: bool> Follower<TWO_WAY> {
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
                if f.length == 0 {
                    crate::dbg::println!("received zero size packet {:?}", f);
                    return;
                }
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
                    Header::Ping if TWO_WAY => {
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
                        if let Err(e) = self
                            .driver
                            .dl_data(DATA_OPT, send_buf)
                            .and_then(|tag| self.sender.enqueue(tag))
                        {
                            crate::dbg::println!("data error {:?}", e);
                            self.state = State::Wait;
                        }
                        self.state = State::Send;
                    }
                    Header::Ping => panic!("Recieved ping during one-way mode"),
                }
            }
            State::Send if !TWO_WAY => {
                panic!("Reached send during one-way mode")
            }
            State::Send => match self.sender.process() {
                Ok(()) => self.state = State::Wait,
                Err(st7580::NbStErr::WouldBlock) => {}
                Err(st7580::NbStErr::Other(st7580::StErr::TxErrNoStatus)) => {
                    crate::dbg::println!("plm did not return status");
                    self.state = State::Wait;
                }
                Err(st7580::NbStErr::Other(st7580::StErr::TxErrAckTmo)) => {
                    crate::dbg::println!("plm ack timed out");
                    self.state = State::Wait;
                }
                Err(st7580::NbStErr::Other(e)) => {
                    panic!("Ping processing error: {:?}", e)
                }
            },
        }
    }
}
