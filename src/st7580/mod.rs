//! Code relating to the ST7580 chip

mod cmd;
mod constants;
pub mod driver;
pub mod frame;
mod globals;
pub mod isr;
mod types;

use constants::*;
use types::*;

/// Code use from the HAL
use hal::{
    gpio::*,
    pac, rcc,
    serial::{config, Event, RxISR, Serial, Serial1, TxISR},
    time,
    timer::{CounterMs, Delay, ExtU32, TimerExt},
};
use stm32f4xx_hal as hal;

/// All the re-exports
pub use driver::*;
pub use frame::*;
pub use isr::*;

pub fn split(
    t_req: PA5<Output<PushPull>>,
    resetn: PA8<Output<PushPull>>,
    usart: pac::USART1,
    usart_tx: PA9<Alternate<7>>,
    usart_rx: PA10<Alternate<7>>,
    tim3: pac::TIM3,
    tim5: pac::TIM5,
    clocks: &rcc::Clocks,
) -> (Driver, InterruptHandler) {
    let t_req = t_req.internal_resistor(Pull::None).speed(Speed::High);
    unsafe { globals::T_REQ_PIN.replace(t_req) };

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

    let isr = InterruptHandler::new();

    let driver = Driver::new(resetn, tim5, clocks);

    (driver, isr)
}
