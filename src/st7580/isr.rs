use cortex_m::prelude::*;
use hal::{
    gpio::{Alternate, PA10, PA9},
    serial::{Event, RxISR, Serial1, TxISR},
};
use stm32f4xx_hal as hal;

use super::{constants::*, frame::*, globals, types::*};

pub struct InterruptHandler {
    ic_timeout: Timeout,
    rx_state: RxIrqStatus,
    rx_cksum: u16,
    rx_frame: Frame,

    ind_frame_queue: globals::FrameProducer<{ globals::QUEUE_SIZE }>,
    cnf_frame_queue: globals::FrameProducer<2>,
    tx_frame_queue: globals::FrameConsumer<2>,

    ack_tx_value: Option<bool>,

    tx_state: TxIrqStatus,
    tx_cur_idx: u8,
    tx_frame: Frame,
}

impl InterruptHandler {
    pub(super) fn new() -> Self {
        unsafe { globals::SERIAL_PLM.as_mut() }
            .unwrap()
            .listen(Event::Rxne);
        Self {
            ic_timeout: Default::default(),
            rx_state: RxIrqStatus::FirstByte,
            rx_cksum: 0,
            rx_frame: Default::default(),
            ind_frame_queue: unsafe { globals::FRAME_QUEUE.split() }.0,
            cnf_frame_queue: unsafe { globals::CONFIRM_FRAME.split() }.0,
            tx_frame_queue: unsafe { globals::TX_FRAME.split() }.1,
            ack_tx_value: None,
            tx_state: TxIrqStatus::SendStx,
            tx_cur_idx: 0,
            tx_frame: Default::default(),
        }
    }

    fn rx(
        &mut self,
        serial: &mut Serial1<(PA9<Alternate<7>>, PA10<Alternate<7>>), u8>,
    ) {
        // First check whether a timeout is expired or not
        if self.ic_timeout.is_expired() {
            self.rx_state = RxIrqStatus::FirstByte;
        }

        // Get received character
        let Ok(c) = serial.read() else { return };

        match self.rx_state {
            RxIrqStatus::FirstByte => match c {
                ACK | NAK if globals::WAIT_ACK.take_signal() => {
                    globals::ACK_RX_VALUE.enqueue(c).unwrap();
                }
                ACK | NAK => {
                    globals::WAIT_STATUS.clear();
                }
                STX_02 | STX_03 => {
                    self.rx_frame.stx = c;
                    self.ic_timeout.set(IC_TMO);
                    self.rx_state = RxIrqStatus::Length;
                }
                STX_STATUS if globals::WAIT_STATUS.take_signal() => {
                    self.ic_timeout.set(IC_TMO);
                    self.rx_state = RxIrqStatus::StatusValue;
                }
                STX_STATUS => {
                    globals::WAIT_ACK.clear();
                }
                _ => {
                    globals::WAIT_STATUS.clear();
                    globals::WAIT_ACK.clear();
                }
            },
            RxIrqStatus::StatusValue => {
                globals::STATUS_VALUE.enqueue(c).unwrap();
                self.ic_timeout.clear();
                self.rx_state = RxIrqStatus::FirstByte;
            }
            RxIrqStatus::Length => {
                self.rx_frame.length = c;
                self.rx_cksum = c as u16;
                self.ic_timeout.set(IC_TMO);
                self.rx_state = RxIrqStatus::Command;
            }
            RxIrqStatus::Command => {
                self.rx_frame.command = c;
                self.rx_cksum += c as u16;
                self.rx_frame.data.clear();
                self.ic_timeout.set(IC_TMO);
                self.rx_state = if self.rx_frame.length == 0 {
                    RxIrqStatus::ChecksumLsb
                } else {
                    RxIrqStatus::Data
                };
            }
            RxIrqStatus::Data => {
                self.rx_frame.data.push(c).unwrap();
                self.rx_cksum += c as u16;
                self.ic_timeout.set(IC_TMO);
                if self.rx_frame.length == self.rx_frame.data.len() as _ {
                    self.rx_state = RxIrqStatus::ChecksumLsb;
                }
            }
            RxIrqStatus::ChecksumLsb => {
                self.rx_frame.checksum = c as u16;
                self.ic_timeout.set(IC_TMO);
                self.rx_state = RxIrqStatus::ChecksumMsb;
            }
            RxIrqStatus::ChecksumMsb => {
                self.rx_frame.checksum |= (c as u16) << 8;

                let valid_cksum = self.rx_frame.checksum == self.rx_cksum;

                self.ack_tx_value = Some(valid_cksum);

                if !valid_cksum {
                    crate::dbg::println!("Invalid cksum {:?}", &self.rx_frame);
                } else if self.rx_frame.command.is_indication() {
                    if matches!(self.rx_frame.command, CMD_RESET_IND)
                        || unsafe { globals::READY_TO_RECEIVE }
                    {
                        self.ind_frame_queue
                            .enqueue(self.rx_frame.clone())
                            .unwrap();
                    }
                } else {
                    self.cnf_frame_queue
                        .enqueue(self.rx_frame.clone())
                        .unwrap();
                }

                self.ic_timeout.clear();
                globals::TX_ACTIVE.set_signal();
                serial.listen(Event::Txe);
                self.rx_state = RxIrqStatus::FirstByte;
            }
        }
    }

    fn tx(
        &mut self,
        serial: &mut Serial1<(PA9<Alternate<7>>, PA10<Alternate<7>>), u8>,
    ) {
        if let Some(ack_tx) = self.ack_tx_value {
            debug_assert!(matches!(self.tx_state, TxIrqStatus::SendStx));
            debug_assert!(matches!(self.tx_cur_idx, 0));
            serial.write(ack_tx.to_ack()).unwrap();
            serial.unlisten(Event::Txe);
            self.ack_tx_value = None;
            return;
        }

        if !matches!(self.tx_state, TxIrqStatus::TxDone) {
            globals::TX_ACTIVE.set_signal();
        }

        match self.tx_state {
            TxIrqStatus::SendStx => {
                self.tx_frame = self
                    .tx_frame_queue
                    .dequeue()
                    .expect("entered TX ISR without TX frame queued");
                serial.write(self.tx_frame.stx).unwrap();
                self.tx_state = TxIrqStatus::SendLength;
            }
            TxIrqStatus::SendLength => {
                unsafe { globals::T_REQ_PIN.as_mut() }.unwrap().set_high();
                serial.write(self.tx_frame.length).unwrap();
                self.tx_state = TxIrqStatus::SendCommand;
            }
            TxIrqStatus::SendCommand => {
                serial.write(self.tx_frame.command).unwrap();
                self.tx_state = TxIrqStatus::SendData;
            }
            TxIrqStatus::SendData => {
                serial
                    .write(self.tx_frame.data[self.tx_cur_idx as usize])
                    .unwrap();
                self.tx_cur_idx += 1;
                if self.tx_frame.length == self.tx_cur_idx {
                    self.tx_state = TxIrqStatus::SendChecksumLsb;
                }
            }
            TxIrqStatus::SendChecksumLsb => {
                serial.write((self.tx_frame.checksum & 0xff) as u8).unwrap();
                self.tx_state = TxIrqStatus::SendChecksumMsb;
            }
            TxIrqStatus::SendChecksumMsb => {
                serial.write((self.tx_frame.checksum >> 8) as u8).unwrap();
                self.tx_state = TxIrqStatus::TxDone;
            }
            TxIrqStatus::TxDone => {
                serial.unlisten(Event::Txe);
                globals::LOCAL_FRAME_TX.set_signal();
                self.tx_state = TxIrqStatus::SendStx;
                self.tx_cur_idx = 0;
            }
        }
    }

    pub fn handle(&mut self) {
        let serial = unsafe { globals::SERIAL_PLM.as_mut() }.unwrap();

        if serial.is_rx_not_empty() {
            self.rx(serial);
        }

        if serial.is_tx_empty() && globals::TX_ACTIVE.take_signal() {
            self.tx(serial);
        }
    }
}
