use super::{signal::Signal, *};
use fugit::Instant;
use hal::{
    gpio::{Alternate, PA10, PA9},
    serial::Serial1,
};
use heapless::{
    mpmc::Q2,
    spsc::{Consumer, Producer, Queue},
};
use stm32f4xx_hal as hal;

pub(super) const QUEUE_SIZE: usize = 8;
pub(super) static mut FRAME_QUEUE: Queue<Frame, QUEUE_SIZE> = Queue::new();
pub(super) type FrameConsumer<const SIZE: usize> =
    Consumer<'static, Frame, SIZE>;
pub(super) type FrameProducer<const SIZE: usize> =
    Producer<'static, Frame, SIZE>;
pub(super) static mut CONFIRM_FRAME: Queue<Frame, 2> = Queue::new();
pub(super) static mut TX_FRAME: Queue<Frame, 2> = Queue::new();

type SerialPlm = Option<Serial1<(PA9<Alternate<7>>, PA10<Alternate<7>>), u8>>;
pub(super) static mut SERIAL_PLM: SerialPlm = None;
pub(super) static mut T_REQ_PIN: Option<PA5<Output<PushPull>>> = None;

static mut NOW: Option<fn() -> Instant<u32, 1, 1000000>> = None;
pub(super) fn set_now_fn(now: fn() -> Instant<u32, 1, 1000000>) {
    assert!(unsafe { NOW.is_none() });
    unsafe { NOW.replace(now) };
}
pub(super) fn now() -> u32 {
    unsafe { NOW.as_mut() }.unwrap()().ticks() / 1000
}

pub(super) static STATUS_VALUE: Q2<u8> = Q2::new();
pub(super) static ACK_RX_VALUE: Q2<u8> = Q2::new();

pub(super) static LOCAL_FRAME_TX: Signal = Signal::new();
pub(super) static WAIT_ACK: Signal = Signal::new();
pub(super) static WAIT_STATUS: Signal = Signal::new();
pub(super) static TX_ACTIVE: Signal = Signal::new();
