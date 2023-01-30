use super::{constants::*, frame::*, globals, types::*};
use hal::{
    gpio::*,
    pac, rcc, serial,
    timer::{DelayMs, ExtU32, TimerExt},
};
use stm32f4xx_hal as hal;

pub struct Driver {
    resetn: PA8<Output<PushPull>>,
    delay: DelayMs<pac::TIM5>,

    ind_frame_queue: globals::FrameConsumer<{ globals::QUEUE_SIZE }>,
    cnf_frame_queue: globals::FrameConsumer<2>,
    tx_frame_queue: globals::FrameProducer<2>,

    cmd_tmo: Timeout,
    status_msg_tmo: Timeout,
    ack_tmo: Timeout,
    sf_state: TxStatus,
    sf_first_iter: bool,
}

impl Driver {
    pub fn new(
        resetn: PA8<Output<PushPull>>,
        tim5: pac::TIM5,
        clocks: &rcc::Clocks,
    ) -> Self {
        Self {
            resetn: resetn.internal_resistor(Pull::None).speed(Speed::High),
            delay: tim5.delay_ms(clocks),
            ind_frame_queue: unsafe { globals::FRAME_QUEUE.split() }.1,
            cnf_frame_queue: unsafe { globals::TX_FRAME.split() }.1,
            tx_frame_queue: unsafe { globals::TX_FRAME.split() }.0,
            cmd_tmo: Default::default(),
            status_msg_tmo: Default::default(),
            ack_tmo: Default::default(),
            sf_state: TxStatus::TxreqLow,
            sf_first_iter: true,
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

    pub fn reset(&mut self) -> StResult<()> {
        let tx_frame = Frame::new(STX_02, 0, CMD_RESET_REQ, [0; 255]);

        self.transmit_frame(tx_frame).and_then(|confirm_frame| {
            if confirm_frame.command != CMD_RESET_CNF {
                Err(StErr::ErrConfirm)
            } else {
                Ok(())
            }
        })
    }

    pub fn mib_write(&mut self, idx: u8, buf: &[u8]) -> StResult<()> {
        assert!(buf.len() < 255);
        let mut data = [0; 255];
        data[0] = idx;
        data[1..buf.len()].clone_from_slice(buf);
        let tx_frame =
            Frame::new(STX_02, (buf.len() as u8) + 1, CMD_MIB_WRITE_REQ, data);

        let confirm_frame = self.transmit_frame(tx_frame)?;

        match confirm_frame.command {
            CMD_MIB_WRITE_ERR => Err(confirm_frame.data[0].try_into().unwrap()),
            CMD_MIB_WRITE_CNF => Ok(()),
            _ => Err(StErr::ErrConfirm),
        }
    }

    pub fn mib_read(&mut self, idx: u8, buf: &mut [u8]) -> StResult<()> {
        let mut data = [0; 255];
        data[0] = idx;
        let tx_frame = Frame::new(STX_02, 1, CMD_MIB_READ_REQ, data);

        let confirm_frame = self.transmit_frame(tx_frame)?;

        match confirm_frame.command {
            CMD_MIB_READ_ERR => Err(confirm_frame.data[0].try_into().unwrap()),
            CMD_MIB_READ_CNF => {
                let len = confirm_frame.length as _;
                if buf.len() < len {
                    Err(StErr::ErrBufLen)
                } else {
                    buf[..len].clone_from_slice(&confirm_frame.data[..len]);
                    Ok(())
                }
            }
            _ => Err(StErr::ErrConfirm.into()),
        }
    }

    pub fn mib_erase(&mut self, idx: u8) -> StResult<()> {
        let mut data = [0; 255];
        data[0] = idx;
        let tx_frame = Frame::new(STX_02, 1, CMD_MIB_ERASE_REQ, data);

        let confirm_frame = self.transmit_frame(tx_frame)?;

        match confirm_frame.command {
            CMD_MIB_ERASE_ERR => Err(confirm_frame.data[0].try_into().unwrap()),
            CMD_MIB_ERASE_CNF => Ok(()),
            _ => Err(StErr::ErrConfirm),
        }
    }

    pub fn ping(&mut self, buf: &[u8]) -> StResult<()> {
        assert!(buf.len() < 255);
        let mut data = [0; 255];
        data[..buf.len()].clone_from_slice(buf);
        let tx_frame = Frame::new(STX_02, buf.len() as u8, CMD_PING_REQ, data);

        let confirm_frame = self.transmit_frame(tx_frame)?;

        if confirm_frame.command == CMD_MIB_ERASE_ERR {
            return Err(confirm_frame.data[0].try_into().unwrap());
        }
        if &confirm_frame.data[..buf.len()] != buf {
            return Err(StErr::ErrPing);
        }
        Ok(())
    }

    /// Returns the confirmation frame or an error
    fn transmit_frame(&mut self, txf: Frame) -> StResult<Frame> {
        self.tx_frame_queue.enqueue(txf).unwrap();
        nb::block!(self.send_frame())?;

        self.cmd_tmo.set(CMD_TMO);
        loop {
            if let Some(f) = self.cnf_frame_queue.dequeue() {
                return Ok(f);
            }
            if self.cmd_tmo.is_expired() {
                return Err(StErr::ErrTimeout);
            }
        }
    }

    fn send_frame(&mut self) -> nb::Result<(), StErr> {
        match self.sf_state {
            TxStatus::TxreqLow => {
                unsafe { globals::LOCAL_FRAME_TX.reset() };
                globals::STATUS_VALUE.dequeue();
                unsafe { globals::T_REQ_PIN.as_mut() }.unwrap().set_low();
                self.status_msg_tmo.set(STATUS_MSG_TMO);
                self.sf_state = TxStatus::WaitStatusFrame;
                Err(nb::Error::WouldBlock)
            }
            TxStatus::WaitStatusFrame => {
                if self.sf_first_iter {
                    globals::WAIT_STATUS.set();
                    self.sf_first_iter = false;
                }

                if self.status_msg_tmo.is_expired() {
                    unsafe { globals::T_REQ_PIN.as_mut() }.unwrap().set_high();
                    self.sf_state = TxStatus::TxreqLow;
                    globals::WAIT_STATUS.reset();
                    self.sf_first_iter = true;
                    return Err(nb::Error::Other(StErr::TxErrNoStatus));
                }

                let status = match globals::STATUS_VALUE.dequeue() {
                    Some(s) => s,
                    None => return Err(nb::Error::WouldBlock),
                };

                self.sf_first_iter = true;
                if status & BUSY_MASK != 0 {
                    unsafe { globals::T_REQ_PIN.as_mut() }.unwrap().set_high();
                    self.sf_state = TxStatus::TxreqLow;
                    Err(nb::Error::Other(StErr::TxErrBusy))
                } else {
                    self.sf_state = TxStatus::WaitTxFrameDone;
                    unsafe { globals::SERIAL_PLM.as_mut() }
                        .unwrap()
                        .listen(serial::Event::Txe);
                    Err(nb::Error::WouldBlock)
                }
            }
            TxStatus::WaitTxFrameDone => {
                if globals::LOCAL_FRAME_TX.check() {
                    self.ack_tmo.set(ACK_TMO);
                    self.sf_state = TxStatus::WaitAck;
                }
                Err(nb::Error::WouldBlock)
            }
            TxStatus::WaitAck => {
                if self.sf_first_iter {
                    globals::WAIT_ACK.set();
                    self.sf_first_iter = false;
                }

                if self.ack_tmo.is_expired() {
                    self.sf_state = TxStatus::TxreqLow;
                    globals::WAIT_ACK.reset();
                    self.sf_first_iter = true;
                    return Err(nb::Error::Other(StErr::TxErrAckTmo));
                }

                let ack = match globals::ACK_RX_VALUE.dequeue() {
                    Some(a) => a,
                    None => return Err(nb::Error::WouldBlock),
                };

                self.sf_state = TxStatus::TxreqLow;
                globals::WAIT_ACK.reset();
                self.sf_first_iter = true;
                if ack == ACK {
                    Ok(())
                } else {
                    Err(nb::Error::Other(StErr::TxErrNak))
                }
            }
        }
    }
}
