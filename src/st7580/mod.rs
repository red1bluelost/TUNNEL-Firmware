//! Code relating to the ST7580 chip

mod constants;
pub mod driver;
pub mod frame;
mod globals;
pub mod isr;
mod types;

/// Code use from the HAL
use hal::{
    gpio::*,
    pac, rcc,
    serial::{config, Serial},
    time,
    timer::{CounterMs, TimerExt},
};
use stm32f4xx_hal as hal;

/// All the re-exports
pub use driver::*;
pub use frame::*;
pub use isr::*;

#[derive(Debug, Default)]
pub struct Builder {
    t_req: Option<PA5<Output<PushPull>>>,
    resetn: Option<PA8<Output<PushPull>>>,
    tx_on: Option<PC0<Input>>,
    rx_on: Option<PC1<Input>>,
    usart: Option<pac::USART1>,
    usart_tx: Option<PA9<Alternate<7>>>,
    usart_rx: Option<PA10<Alternate<7>>>,
    tim3: Option<pac::TIM3>,
    tim5: Option<pac::TIM5>,
}

macro_rules! setter {
    ($n:ident: $t: ty) => {
        #[inline(always)]
        pub fn $n(self, $n: $t) -> Self {
            assert!(self.$n.is_none());
            let $n = Some($n);
            Self { $n, ..self }
        }
    };
}

impl Builder {
    setter!(t_req: PA5<Output<PushPull>>);
    setter!(resetn: PA8<Output<PushPull>>);
    setter!(tx_on: PC0<Input>);
    setter!(rx_on: PC1<Input>);
    setter!(usart: pac::USART1);
    setter!(usart_tx: PA9<Alternate<7>>);
    setter!(usart_rx: PA10<Alternate<7>>);
    setter!(tim3: pac::TIM3);
    setter!(tim5: pac::TIM5);

    pub fn split(self, clocks: &rcc::Clocks) -> (Driver, InterruptHandler) {
        let t_req = self.t_req.unwrap();
        let resetn = self.resetn.unwrap();
        let tx_on = self.tx_on.unwrap();
        let rx_on = self.rx_on.unwrap();
        let usart = self.usart.unwrap();
        let usart_tx = self.usart_tx.unwrap();
        let usart_rx = self.usart_rx.unwrap();
        let tim3 = self.tim3.unwrap();
        let tim5 = self.tim5.unwrap();

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

        let driver = Driver::new(resetn, tx_on, rx_on, tim5, clocks);

        (driver, isr)
    }
}
