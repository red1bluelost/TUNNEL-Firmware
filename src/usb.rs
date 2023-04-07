use crate::{mem, util::Exchange};
use heapless::spsc::{Consumer, Producer, Queue};
use stm32f4xx_hal::otg_fs::UsbBusType;
use usb_device::{bus::UsbBusAllocator, class::UsbClass};
use usbd_serial::{CdcAcmClass, Result, UsbError};

pub const QUEUE_SIZE: usize = 32;
pub type Elem = mem::BufBox;
pub type UsbQueue = Queue<Elem, QUEUE_SIZE>;
pub type UsbProducer = Producer<'static, Elem, QUEUE_SIZE>;
pub type UsbConsumer = Consumer<'static, Elem, QUEUE_SIZE>;
static mut IN_QUEUE: UsbQueue = Queue::new();
static mut OUT_QUEUE: UsbQueue = Queue::new();

pub struct UsbManager {
    serial: CdcAcmClass<'static, UsbBusType>,
    in_consumer: UsbConsumer,
    out_producer: UsbProducer,
    current_read: mem::BufBox,
}

impl UsbManager {
    pub fn class(&mut self) -> &mut dyn UsbClass<UsbBusType> {
        &mut self.serial
    }

    pub fn poll(&mut self) -> Result<()> {
        // Reserve space for reading from host
        let capacity = self.current_read.capacity();
        if self.current_read.len() < capacity {
            self.current_read.resize(capacity, 0).unwrap();
        }
        // Attempt read from host
        match self.serial.read_packet(&mut self.current_read) {
            // Assuming won't occurs since it would result in WouldBlock instead
            Ok(0) => unreachable!(),
            // Hand off the data to the queue
            Ok(len) if cfg!(any(feature = "TWO_WAY", feature = "LEADER")) => {
                let mut sending =
                    self.current_read.exchange(mem::alloc().unwrap());
                sending.truncate(len);

                if let Err(_e) = self.out_producer.enqueue(sending) {
                    crate::dbg::println!("The out going message queue is full");
                };
            }
            Ok(_) => {}
            // No new data so continue
            Err(UsbError::WouldBlock) => {}
            // Return all other errors
            Err(e) => return Err(e),
        }

        // Dequeue next write or return
        let Some(current_write) = self.in_consumer.dequeue() else { return Ok(()) };

        // Write the data to host
        match self.serial.write_packet(&current_write) {
            // Currently relying on everything being sent
            Ok(len) => debug_assert_eq!(len, current_write.len()),
            // Cannot handle if the buffers are full
            Err(UsbError::WouldBlock) => {
                crate::dbg::println!("usb IN buffer is full");
            }
            // Return all other errors
            Err(e) => return Err(e),
        }

        Ok(())
    }
}

pub struct UsbSplit {
    pub usb_manager: UsbManager,
    pub in_producer: UsbProducer,
    pub out_consumer: UsbConsumer,
}

pub fn split(alloc: &'static UsbBusAllocator<UsbBusType>) -> UsbSplit {
    cortex_m::singleton!(:bool = false).expect("May only call split once");

    let serial = CdcAcmClass::new(alloc, 64);

    let (in_producer, in_consumer) = unsafe { IN_QUEUE.split() };
    let (out_producer, out_consumer) = unsafe { OUT_QUEUE.split() };

    let usb_manager = UsbManager {
        serial,
        in_consumer,
        out_producer,
        current_read: mem::alloc().unwrap(),
    };

    UsbSplit {
        usb_manager,
        in_producer,
        out_consumer,
    }
}
