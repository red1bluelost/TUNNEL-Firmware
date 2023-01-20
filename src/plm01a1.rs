//! Code relating to the plm01a1 breakout board

use hal::{
    gpio::*,
    pac, rcc,
    serial::{config, Event, Serial, Serial1},
    time,
};
use stm32f4xx_hal as hal;

pub struct PLM {
    t_req: PA5<Output<PushPull>>,
    resetn: PA8<Output<PushPull>>,
    tx_on: PC0<Input>,
    rx_on: PC1<Input>,
    serial_plm: Serial1<(PA9<Alternate<7>>, PA10<Alternate<7>>), u8>,
}

impl PLM {
    pub fn new(
        t_req: PA5<Output<PushPull>>,
        resetn: PA8<Output<PushPull>>,
        tx_on: PC0<Input>,
        rx_on: PC1<Input>,
        usart: pac::USART1,
        usart_tx: PA9<Alternate<7>>,
        usart_rx: PA10<Alternate<7>>,
        clocks: &rcc::Clocks,
    ) -> Self {
        let t_req = t_req.internal_resistor(Pull::None).speed(Speed::High);
        let resetn = resetn.internal_resistor(Pull::None).speed(Speed::High);
        let tx_on = tx_on.internal_resistor(Pull::None);
        let rx_on = rx_on.internal_resistor(Pull::None);
        let mut serial_plm = Serial::new(
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
        serial_plm.listen(Event::Rxne);

        Self {
            t_req,
            resetn,
            tx_on,
            rx_on,
            serial_plm,
        }
    }
}
