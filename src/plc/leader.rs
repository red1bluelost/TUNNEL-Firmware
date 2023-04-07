use super::{Channels, Header, DATA_OPT, DATA_START, HEADER_IDX};
use crate::{mem, st7580, usb};
use stm32f4xx_hal::timer::{self, DelayUs};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum State {
    Dispatch,
    SendData,
    SendPing,
    WaitPing,
}

pub struct Leader<const TWO_WAY: bool> {
    state: State,
    driver: st7580::Driver,
    sender: st7580::DSender,
    channels: Channels,
    ping_timeout: st7580::Timeout,
    fail_timeout: st7580::Timeout,
}

impl<const TWO_WAY: bool> Leader<TWO_WAY> {
    pub fn new(
        driver: st7580::Driver,
        sender: st7580::DSender,
        in_producer: usb::UsbProducer,
        out_consumer: usb::UsbConsumer,
    ) -> Self {
        let mut fail_timeout = st7580::Timeout::default();
        fail_timeout.set(1);
        Self {
            state: State::Dispatch,
            driver,
            sender,
            channels: Channels {
                in_producer,
                out_consumer,
            },
            ping_timeout: Default::default(),
            fail_timeout,
        }
    }

    pub fn init<TIM: timer::Instance>(&mut self, delay: &mut DelayUs<TIM>) {
        super::shared_init(delay, &mut self.driver, &mut self.sender)
    }

    pub fn process(&mut self) {
        match self.state {
            State::Dispatch if !self.fail_timeout.is_expired() => {
                // Wait for the plm to get back
            }
            State::Dispatch => {
                let receive_opt = self.channels.out_consumer.dequeue();

                let send_buf = match receive_opt {
                    Some(mut send_buf) => {
                        self.state = State::SendData;
                        send_buf.push(Header::Data.into()).unwrap();
                        send_buf.rotate_right(1);
                        send_buf
                    }
                    None if TWO_WAY => {
                        self.state = State::SendPing;
                        mem::alloc_from_slice(&[Header::Ping.into()]).unwrap()
                    }
                    None => return,
                };

                if let Err(e) = self
                    .driver
                    // .phy_data(DATA_OPT, send_buf)
                    .dl_data(DATA_OPT, send_buf)
                    .and_then(|tag| self.sender.enqueue(tag))
                {
                    crate::dbg::println!("data error {:?}", e);
                    self.fail_timeout.set(100);
                    self.state = State::Dispatch;
                }
            }
            State::SendPing | State::SendData => match self.sender.process() {
                Ok(()) if self.state == State::SendPing => {
                    self.ping_timeout.set(500);
                    self.state = State::WaitPing;
                }
                Ok(()) => self.state = State::Dispatch,
                Err(st7580::NbStErr::WouldBlock) => {}
                Err(st7580::NbStErr::Other(st7580::StErr::TxErrNoStatus)) => {
                    crate::dbg::println!("plm did not return status");
                    self.state = State::Dispatch;
                }
                Err(st7580::NbStErr::Other(st7580::StErr::TxErrAckTmo)) => {
                    crate::dbg::println!("plm ack timed out");
                    self.state = State::Dispatch;
                }
                Err(st7580::NbStErr::Other(st7580::StErr::TxErrBusy)) => {
                    crate::dbg::println!("plm tx busy");
                    self.state = State::Dispatch;
                }
                Err(st7580::NbStErr::Other(st7580::StErr::TxErrNak)) => {
                    crate::dbg::println!("plm tx NAK");
                    self.state = State::Dispatch;
                }
                Err(st7580::NbStErr::Other(e)) => {
                    panic!("{:?} processing error: {:?}", self.state, e)
                }
            },
            State::WaitPing if self.ping_timeout.is_expired() => {
                self.state = State::Dispatch;
            }
            State::WaitPing => {
                let Some(f) = self.driver.receive_frame() else { return };
                debug_assert!(matches!(f.stx, st7580::STX_03 | st7580::STX_02));
                let header = f.data[HEADER_IDX].try_into().unwrap();
                match header {
                    Header::Ping => panic!("Unexpected Ping from follower"),
                    Header::Data => {
                        let len = f.length as usize;
                        let mut data = f.data;
                        data.copy_within(DATA_START..len, 0);
                        data.truncate(len - DATA_START);
                        self.channels.in_producer.enqueue(data).unwrap();
                    }
                    Header::Idle => {}
                }
                self.state = State::Dispatch;
            }
        }
    }
}
