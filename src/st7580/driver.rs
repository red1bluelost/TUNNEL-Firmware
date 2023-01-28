use super::{constants::*, frame::*, globals, types::*};
use hal::{
    gpio::*,
    pac, rcc,
    timer::{DelayMs, ExtU32, TimerExt},
};
use stm32f4xx_hal as hal;

pub struct Driver {
    resetn: PA8<Output<PushPull>>,
    delay: DelayMs<pac::TIM5>,
    ind_frame_queue: globals::FrameConsumer<{ globals::QUEUE_SIZE }>,
}

impl Driver {
    pub fn new(
        resetn: PA8<Output<PushPull>>,
        tim5: pac::TIM5,
        clocks: &rcc::Clocks,
    ) -> Self {
        let resetn = resetn.internal_resistor(Pull::None).speed(Speed::High);
        let delay = tim5.delay_ms(clocks);
        let ind_frame_queue = unsafe { globals::FRAME_QUEUE.split().1 };
        Self {
            resetn,
            delay,
            ind_frame_queue,
        }
    }

    pub fn init(&mut self) {
        self.resetn.set_low();
        self.delay.delay(1500.millis());
        self.resetn.set_high();

        loop {
            self.delay.delay(100.millis());
            if self
                .ind_frame_queue
                .dequeue()
                .map_or(false, |f| f.command == CMD_RESET_IND)
            {
                return;
            }
        }
    }
}
