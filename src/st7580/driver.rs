use super::globals;
use hal::{gpio::*, pac};
use stm32f4xx_hal as hal;

pub struct Driver {
    resetn: PA8<Output<PushPull>>,
}

impl Driver {
    pub fn new(resetn: PA8<Output<PushPull>>) -> Self {
        let resetn = resetn.internal_resistor(Pull::None).speed(Speed::High);
        Self { resetn }
    }

    pub fn init() {}
}
