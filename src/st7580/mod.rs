//! Code relating to the ST7580 chip

use fugit::Instant;
use hal::{
    gpio::{
        Alternate, Input, Output, Pull, PushPull, Speed, PA10, PA5, PA8, PA9,
        PC0, PC1,
    },
    pac, rcc,
    serial::{config, Serial},
    time,
};
use stm32f4xx_hal as hal;

/// All the re-exports
pub use constants::*;
pub use driver::*;
pub use frame::*;
pub use isr::*;

pub mod constants;
pub mod driver;
pub mod frame;
mod globals;
pub mod isr;
mod signal;
mod types;

pub struct Builder {
    pub t_req: PA5<Output<PushPull>>,
    pub resetn: PA8<Output<PushPull>>,
    pub tx_on: PC0<Input>,
    pub rx_on: PC1<Input>,
    pub usart: pac::USART1,
    pub usart_tx: PA9<Alternate<7>>,
    pub usart_rx: PA10<Alternate<7>>,
    pub now: fn() -> Instant<u32, 1, 1000000>,
}

impl Builder {
    pub fn split(
        self,
        clocks: &rcc::Clocks,
    ) -> (Driver, DSender, InterruptHandler) {
        cortex_m::singleton!(:bool = false).expect("May only call split once");

        let Self {
            t_req,
            resetn,
            tx_on,
            rx_on,
            usart,
            usart_tx,
            usart_rx,
            now,
        } = self;

        let mut t_req = t_req.internal_resistor(Pull::None).speed(Speed::High);
        t_req.set_high();
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

        globals::set_now_fn(now);

        let isr = InterruptHandler::new();

        let driver = Driver::new(resetn, tx_on, rx_on);
        let dsender = DSender::new();

        (driver, dsender, isr)
    }
}
