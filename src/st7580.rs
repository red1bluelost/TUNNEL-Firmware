//! Code relating to the ST7580 chip

use cortex_m::singleton;
use heapless::spsc;
use stm32f4xx_hal as hal;

enum Err {
    TxInProgress = -1,
    ErrConfirm = -2,
    ErrBufLen = -3,
    ErrTimeout = -4,
    ErrPing = -5,
    ErrArgs = -6,
    UnexpectedFrame = -7,
    RcvBufTooSmall = -8,
    TxErrNak = -10,
    TxErrNoStatus = -11,
    TxErrAckTmo = -12,
    TxErrBusy = -13,
}

pub type Result<T> = core::result::Result<T, Err>;

#[repr(C)]
pub struct Frame {
    stx: u8,
    length: u8,
    command: u8,
    data: [u8; 255],
    checksum: u16,
}

impl Frame {
    fn checksum(command: u8, length: u8, data: &[u8]) -> u16 {
        assert_eq!(length as usize, data.len());
        data.iter().fold(
            u16::wrapping_add(command.into(), length.into()),
            |acc, &val| u16::wrapping_add(acc, val.into()),
        )
    }
}

const QUEUE_SIZE: usize = 8;
static mut FRAME_QUEUE: spsc::Queue<Frame, QUEUE_SIZE> = spsc::Queue::new();

pub struct Producer {
    inner: spsc::Producer<'static, Frame, QUEUE_SIZE>,
}

impl Producer {
    pub fn take() -> Self {
        // Ensure that take is only called once
        singleton!(: bool = false).unwrap();
        Self {
            inner: unsafe { FRAME_QUEUE.split().0 },
        }
    }

    pub fn enqueue(&mut self, f: Frame) -> Result<()> {
        match self.inner.enqueue(f) {
            Ok(()) => Ok(()),
            Err(_) => Err(Err::RcvBufTooSmall),
        }
    }
}

pub struct Consumer {
    inner: spsc::Consumer<'static, Frame, QUEUE_SIZE>,
}

impl Consumer {
    pub fn take() -> Self {
        // Ensure that take is only called once
        singleton!(: bool = false).unwrap();
        Self {
            inner: unsafe { FRAME_QUEUE.split().1 },
        }
    }

    pub fn dequeue(&mut self) -> Option<Frame> {
        self.inner.dequeue()
    }
}
