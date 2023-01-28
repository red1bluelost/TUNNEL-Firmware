use super::*;
use crate::signal::Signal;
use hal::{gpio::*, serial::Serial1};
use heapless::{
    mpmc::Q2,
    spsc::{Consumer, Producer, Queue},
};
use stm32f4xx_hal as hal;

pub const QUEUE_SIZE: usize = 8;
pub static mut FRAME_QUEUE: Queue<Frame, QUEUE_SIZE> = Queue::new();
pub type FrameConsumer<const SIZE: usize> = Consumer<'static, Frame, SIZE>;
pub type FrameProducer<const SIZE: usize> = Producer<'static, Frame, SIZE>;
pub static mut CONFIRM_FRAME: Queue<Frame, 2> = Queue::new();

pub static mut TX_FRAME: Queue<Frame, 2> = Queue::new();

type SerialPlm = Option<Serial1<(PA9<Alternate<7>>, PA10<Alternate<7>>), u8>>;
pub static mut SERIAL_PLM: SerialPlm = None;
pub static mut T_REQ_PIN: Option<PA5<Output<PushPull>>> = None;

pub static mut COUNTER: Option<CounterMs<pac::TIM3>> = None;
pub fn now() -> u32 {
    unsafe { COUNTER.as_ref() }.unwrap().now().ticks()
}

pub static STATUS_VALUE: Q2<u8> = Q2::new();
pub static ACK_RX_VALUE: Q2<u8> = Q2::new();

pub static WAIT_ACK: Signal = Signal::new();
pub static WAIT_STATUS: Signal = Signal::new();
pub static LOCAL_FRAME_TX: Signal = Signal::new();
