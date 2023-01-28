//! Code relating to the ST7580 chip

mod cmd;
mod constants;
mod globals;
mod types;

use core::borrow::{Borrow, BorrowMut};

use constants::*;
use cortex_m::prelude::_embedded_hal_serial_Read;
use hal::{
    gpio::*,
    pac, rcc,
    serial::{config, Event, RxISR, Serial, Serial1, TxISR},
    time,
    timer::{CounterMs, Delay, ExtU32, TimerExt},
};
use stm32f4xx_hal as hal;
use types::*;

#[repr(C)]
#[derive(Debug, Clone)]
pub struct Frame {
    stx: u8,
    length: u8,
    command: u8,
    data: [u8; 255],
    checksum: u16,
}

impl Default for Frame {
    fn default() -> Self {
        Self {
            stx: Default::default(),
            length: Default::default(),
            command: Default::default(),
            data: [0; 255],
            checksum: Default::default(),
        }
    }
}

impl Frame {
    fn calc_checksum(command: u8, length: u8, data: &[u8]) -> u16 {
        assert_eq!(length as usize, data.len());
        data.iter().fold(
            u16::wrapping_add(command.into(), length.into()),
            |acc, &val| u16::wrapping_add(acc, val.into()),
        )
    }

    fn checksum(&self) -> u16 {
        Self::calc_checksum(
            self.command,
            self.length,
            &self.data[..self.length as _],
        )
    }

    fn clear(&mut self) {
        *self = Default::default();
    }
}

#[derive(Default, Debug, Clone, Copy)]
struct Timeout {
    tmo: u32,
    tmo_start_time: u32,
}

impl Timeout {
    fn is_expired(&self, now: u32) -> bool {
        let Timeout {
            tmo,
            tmo_start_time,
        } = *self;
        let elapse = if now >= tmo_start_time {
            now - tmo_start_time
        } else {
            now + (u32::MAX - tmo_start_time)
        };
        elapse >= tmo
    }

    fn set(&mut self, tmo: u32, now: u32) {
        let tmo_start_time = now;
        *self = Timeout {
            tmo,
            tmo_start_time,
        };
    }
}

pub fn split(
    usart: pac::USART1,
    usart_tx: PA9<Alternate<7>>,
    usart_rx: PA10<Alternate<7>>,
    tim3: pac::TIM3,
    clocks: &rcc::Clocks,
) -> ((), InterruptHandler) {
    let serial_plm = Serial::new(
        usart,
        (
            usart_tx.internal_resistor(Pull::None).speed(Speed::High),
            usart_rx.internal_resistor(Pull::None).speed(Speed::High),
        ),
        config::Config::default()
            .wordlength_8()
            .baudrate(time::Bps(57600))
            .stopbits(config::StopBits::STOP1)
            .parity_none()
            .dma(config::DmaConfig::TxRx),
        clocks,
    )
    .unwrap();
    unsafe { globals::SERIAL_PLM.replace(serial_plm) };

    let counter = tim3.counter(clocks);
    unsafe { globals::COUNTER.replace(counter) };

    todo!()
}

fn now() -> u32 {
    unsafe { globals::COUNTER.as_ref() }.unwrap().now().ticks()
}

pub struct InterruptHandler {
    ic_timeout: Timeout,
    rx_state: RxIrqStatus,
    rx_cur_idx: u8,
    rx_cksum: u16,
    rx_frame: Frame,

    tx_state: TxIrqStatus,
}

impl InterruptHandler {
    fn rx(
        &mut self,
        serial: &mut Serial1<(PA9<Alternate<7>>, PA10<Alternate<7>>), u8>,
    ) {
        // First check whether a timeout is expired or not
        if self.ic_timeout.is_expired(now()) {
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
                    self.ic_timeout.set(IC_TMO, now());
                    self.rx_state = RxIrqStatus::Length;
                }
                STX_STATUS => {
                    if globals::WAIT_STATUS.check() {
                        self.ic_timeout.set(IC_TMO, now());
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
                self.ic_timeout.set(IC_TMO, now());
                self.rx_state = RxIrqStatus::Command;
            }
            RxIrqStatus::Command => {
                self.rx_frame.command = c;
                self.rx_cksum += c as u16;
                self.rx_cur_idx = 0;
                self.ic_timeout.set(IC_TMO, now());
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
                self.ic_timeout.set(IC_TMO, now());
                if self.rx_frame.length == self.rx_cur_idx {
                    self.rx_state = RxIrqStatus::ChecksumLsb;
                }
            }
            RxIrqStatus::ChecksumLsb => {
                self.rx_frame.checksum = c as u16;
                self.ic_timeout.set(IC_TMO, now());
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
                    globals::ACK_TX_VALUE.enqueue(ACK).unwrap();
                } else {
                    globals::ACK_TX_VALUE.enqueue(NAK).unwrap();
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
