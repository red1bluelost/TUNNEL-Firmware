use super::constants::*;
use super::frame::*;
use super::globals;
use super::types::*;
use core::borrow::BorrowMut;
use cortex_m::prelude::*;
use hal::{
    gpio::*,
    pac, rcc,
    serial::{config, Event, RxISR, Serial, Serial1, TxISR},
    time,
    timer::{CounterMs, Delay, ExtU32, TimerExt},
};
use stm32f4xx_hal as hal;

pub struct InterruptHandler {
    ic_timeout: Timeout,
    rx_state: RxIrqStatus,
    rx_cur_idx: u8,
    rx_cksum: u16,
    rx_frame: Frame,

    ack_tx_value: Option<u8>,

    tx_state: TxIrqStatus,
    tx_cur_idx: u8,
    tx_frame: Frame,
}

impl InterruptHandler {
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
                ACK | NAK => {
                    if globals::WAIT_ACK.check() {
                        globals::ACK_RX_VALUE.enqueue(c).unwrap();
                        globals::WAIT_ACK.reset();
                    } else {
                        globals::WAIT_STATUS.reset();
                    }
                }
                STX_02 | STX_03 => {
                    self.rx_frame.stx = c;
                    self.ic_timeout.set(IC_TMO);
                    self.rx_state = RxIrqStatus::Length;
                }
                STX_STATUS => {
                    if globals::WAIT_STATUS.check() {
                        self.ic_timeout.set(IC_TMO);
                        self.rx_state = RxIrqStatus::StatusValue;
                    } else {
                        globals::WAIT_ACK.reset();
                    }
                }
                _ => {
                    globals::WAIT_STATUS.reset();
                    globals::WAIT_ACK.reset();
                }
            },
            RxIrqStatus::StatusValue => {
                globals::STATUS_VALUE.enqueue(c).unwrap();
                globals::WAIT_STATUS.reset();
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
                self.rx_cur_idx = 0;
                self.ic_timeout.set(IC_TMO);
                self.rx_state = if self.rx_frame.length == 0 {
                    RxIrqStatus::ChecksumLsb
                } else {
                    RxIrqStatus::Data
                };
            }
            RxIrqStatus::Data => {
                self.rx_frame.data[self.rx_cur_idx as usize] = c;
                self.rx_cur_idx += 1;
                self.rx_cksum += c as u16;
                self.ic_timeout.set(IC_TMO);
                if self.rx_frame.length == self.rx_cur_idx {
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
                if self.rx_frame.checksum == self.rx_cksum {
                    if self.rx_frame.command.is_indication() {
                        unsafe { globals::FRAME_QUEUE.borrow_mut() }
                            .enqueue(self.rx_frame.clone())
                            .unwrap();
                    } else {
                        unsafe { globals::CONFIRM_FRAME.borrow_mut() }
                            .enqueue(self.rx_frame.clone())
                            .unwrap();
                    }
                    self.ack_tx_value = Some(ACK);
                } else {
                    self.ack_tx_value = Some(NAK);
                }
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
            serial.write(ack_tx).unwrap();
            self.tx_state = TxIrqStatus::TxDone;
        }

        match self.tx_state {
            TxIrqStatus::SendStx => {
                self.tx_frame = unsafe { globals::TX_FRAME.borrow_mut() }
                    .dequeue()
                    .unwrap();
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
                if self.ack_tx_value.is_some() {
                    self.ack_tx_value = None;
                } else {
                    globals::LOCAL_FRAME_TX.check();
                }
                self.tx_state = TxIrqStatus::SendStx;
                self.tx_cur_idx = 0;
            }
        }
    }

    pub fn handle(&mut self) {
        let serial = unsafe { globals::SERIAL_PLM.as_mut().unwrap() };

        if serial.is_rx_not_empty() {
            self.rx(serial);
        }

        if serial.is_tx_empty() {
            self.tx(serial);
        }
    }
}
