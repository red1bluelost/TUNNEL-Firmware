//! Code relating to the ST7580 chip

pub mod constants;
pub mod driver;
pub mod frame;
mod globals;
pub mod isr;
mod types;

/// Code use from the HAL
use hal::{
    gpio::*,
    pac,
    prelude::*,
    rcc,
    serial::{config, Serial},
    time,
};
use stm32f4xx_hal as hal;

/// All the re-exports
pub use constants::*;
pub use driver::*;
pub use frame::*;
pub use isr::*;

pub struct Builder {
    pub t_req: PA5<Output<PushPull>>,
    pub resetn: PA8<Output<PushPull>>,
    pub tx_on: PC0<Input>,
    pub rx_on: PC1<Input>,
    pub usart: pac::USART1,
    pub usart_tx: PA9<Alternate<7>>,
    pub usart_rx: PA10<Alternate<7>>,
    pub tim3: pac::TIM3,
    pub tim5: pac::TIM5,
}

impl Builder {
    pub fn split(self, clocks: &rcc::Clocks) -> (Driver, InterruptHandler) {
        let Self {
            t_req,
            resetn,
            tx_on,
            rx_on,
            usart,
            usart_tx,
            usart_rx,
            tim3,
            tim5,
        } = self;

        let t_req = t_req.internal_resistor(Pull::None).speed(Speed::High);
        unsafe { globals::T_REQ_PIN.replace(t_req) };

        let serial_plm = Serial::new(
            usart,
            (
                usart_tx
                    .internal_resistor(Pull::None)
                    .speed(Speed::VeryHigh),
                usart_rx
                    .internal_resistor(Pull::None)
                    .speed(Speed::VeryHigh),
            ),
            config::Config::default()
                .wordlength_8()
                .baudrate(time::Bps(57600))
                .stopbits(config::StopBits::STOP1)
                .parity_none(),
            clocks,
        )
        .unwrap();
        unsafe { globals::SERIAL_PLM.replace(serial_plm) };

        let counter = tim3.counter(clocks);
        unsafe { globals::COUNTER.replace(counter) };

        let isr = InterruptHandler::new();

        let driver = Driver::new(resetn, tx_on, rx_on, tim5, clocks);

        (driver, isr)
    }
}
