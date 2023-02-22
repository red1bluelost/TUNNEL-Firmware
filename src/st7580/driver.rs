use super::{constants::*, frame::*, globals, types::*};
use hal::{
    gpio::{Input, Output, Pull, PushPull, Speed, PA8, PC0, PC1},
    pac, rcc, serial,
    timer::{DelayUs, ExtU32, TimerExt},
};
use stm32f4xx_hal as hal;

pub struct Driver {
    resetn: PA8<Output<PushPull>>,
    pub delay: DelayUs<pac::TIM3>,

    #[allow(unused)]
    tx_on: PC0<Input>,
    #[allow(unused)]
    rx_on: PC1<Input>,

    ind_frame_queue: globals::FrameConsumer<{ globals::QUEUE_SIZE }>,
    cnf_frame_queue: globals::FrameConsumer<2>,
    tx_frame_queue: globals::FrameProducer<2>,

    cmd_tmo: Timeout,
    status_msg_tmo: Timeout,
    ack_tmo: Timeout,
    sf_state: TxStatus,
}

impl Driver {
    pub fn new(
        resetn: PA8<Output<PushPull>>,
        tx_on: PC0<Input>,
        rx_on: PC1<Input>,
        tim3: pac::TIM3,
        clocks: &rcc::Clocks,
    ) -> Self {
        let mut resetn =
            resetn.internal_resistor(Pull::None).speed(Speed::VeryHigh);
        resetn.set_high();
        Self {
            resetn,
            tx_on: tx_on.internal_resistor(Pull::None),
            rx_on: rx_on.internal_resistor(Pull::None),
            delay: tim3.delay_us(clocks),
            ind_frame_queue: unsafe { globals::FRAME_QUEUE.split() }.1,
            cnf_frame_queue: unsafe { globals::CONFIRM_FRAME.split() }.1,
            tx_frame_queue: unsafe { globals::TX_FRAME.split() }.0,
            cmd_tmo: Default::default(),
            status_msg_tmo: Default::default(),
            ack_tmo: Default::default(),
            sf_state: TxStatus::TxreqLow,
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
        data[1..buf.len() + 1].clone_from_slice(buf);
        let tx_frame =
            Frame::new(STX_02, buf.len() as u8 + 1, CMD_MIB_WRITE_REQ, data);

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
            _ => Err(StErr::ErrConfirm),
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

    /// Ping ST7580 PLC Modem.
    ///
    /// # Arguments
    ///
    /// * `buf` - buffer containing ping test data to be sent. If ping is
    ///   success ST7580 PLC Modem will reply with the same data.
    pub fn ping(&mut self, buf: &[u8]) -> StResult<()> {
        assert!(buf.len() < 255);
        let mut data = [0; 255];
        data[..buf.len()].clone_from_slice(buf);
        let tx_frame = Frame::new(STX_02, buf.len() as u8, CMD_PING_REQ, data);

        let confirm_frame = self.transmit_frame(tx_frame)?;

        if confirm_frame.command != CMD_PING_CNF {
            return Err(confirm_frame.data[0].try_into().unwrap());
        }
        if &confirm_frame.data[..buf.len()] != buf {
            return Err(StErr::ErrPing);
        }
        Ok(())
    }

    pub fn phy_data(
        &mut self,
        plm_opts: u8,
        send_buf: &[u8],
        conf_buf: Option<&mut [u8]>,
    ) -> StResult<()> {
        self.impl_phy_dl_data::<
            PHY_DATALEN_MAX,
            CMD_PHY_DATA_REQ,
            CMD_PHY_DATA_CNF,
            CMD_PHY_DATA_ERR
        >(plm_opts, send_buf, conf_buf)
    }

    pub fn dl_data(
        &mut self,
        plm_opts: u8,
        send_buf: &[u8],
        conf_buf: Option<&mut [u8]>,
    ) -> StResult<()> {
        self.impl_phy_dl_data::<
            DL_DATALEN_MAX,
            CMD_DL_DATA_REQ,
            CMD_DL_DATA_CNF,
            CMD_DL_DATA_ERR
        >(plm_opts, send_buf, conf_buf)
    }

    fn impl_phy_dl_data<
        const LEN_MAX: usize,
        const REQ: u8,
        const CNF: u8,
        const ERR: u8,
    >(
        &mut self,
        plm_opts: u8,
        send_buf: &[u8],
        conf_buf: Option<&mut [u8]>,
    ) -> StResult<()> {
        if send_buf.len() > LEN_MAX {
            return Err(StErr::ErrArgs);
        }
        let mut data = [0; 255];
        let mut offset = 0;
        data[offset] = plm_opts;
        offset += 1;

        #[cfg(feature = "CUSTOM_MIB_FREQUENCY")]
        for val in TXFREQS {
            data[offset] = val;
            offset += 1;
        }

        #[cfg(feature = "GAIN_SELECTOR")]
        {
            data[offset] = TXGAIN;
            offset += 1;
        }

        data[offset..send_buf.len() + offset].clone_from_slice(send_buf);

        let tx_frame = Frame::new(STX_02, send_buf.len() as u8 + 1, REQ, data);

        let confirm_frame = self.transmit_frame(tx_frame)?;

        if confirm_frame.command == ERR {
            return Err(confirm_frame.data[0].try_into().unwrap());
        }
        if confirm_frame.command != CNF {
            return Err(StErr::ErrConfirm);
        }
        let Some(conf_buf) = conf_buf else { return Ok(()) };
        let len = PHY_DL_SS_RET_LEN;
        if conf_buf.len() < len {
            return Err(StErr::ErrBufLen);
        }
        conf_buf[..len].clone_from_slice(&confirm_frame.data[..len]);
        Ok(())
    }

    pub fn ss_data(
        &mut self,
        plm_opts: u8,
        send_buf: &[u8],
        clr_len: u8,
        enc_len: u8,
        ret_buf: Option<&mut [u8]>,
    ) -> StResult<()> {
        let data_len = send_buf.len();
        assert_eq!(data_len, clr_len as usize + enc_len as usize);
        if (data_len > SS_DATALEN_MAX)
            || (enc_len == 0 && clr_len < 16)
            || (enc_len > 0 && data_len < 4)
        {
            return Err(StErr::ErrArgs);
        }

        let mut data = [0; 255];
        let mut offset = 0;
        data[offset] = plm_opts;
        offset += 1;

        #[cfg(feature = "CUSTOM_MIB_FREQUENCY")]
        for val in TXFREQS {
            data[offset] = val;
            offset += 1;
        }

        #[cfg(feature = "GAIN_SELECTOR")]
        {
            data[offset] = TXGAIN;
            offset += 1;
        }

        data[offset] = clr_len;
        offset += 1;

        data[offset..send_buf.len() + offset].clone_from_slice(send_buf);

        let tx_frame =
            Frame::new(STX_02, send_buf.len() as u8 + 2, CMD_SS_DATA_REQ, data);

        let confirm_frame = self.transmit_frame(tx_frame)?;

        if confirm_frame.command == CMD_SS_DATA_ERR {
            return Err(confirm_frame.data[0].try_into().unwrap());
        }
        if confirm_frame.command != CMD_SS_DATA_CNF {
            return Err(StErr::ErrConfirm);
        }
        let Some(ret_buf) = ret_buf else { return Ok(()) };
        let len = PHY_DL_SS_RET_LEN;
        if ret_buf.len() < len {
            return Err(StErr::ErrBufLen);
        }
        ret_buf[..len].clone_from_slice(&confirm_frame.data[..len]);
        Ok(())
    }

    #[inline(always)]
    pub fn receive_frame(&mut self) -> Option<Frame> {
        self.ind_frame_queue.dequeue()
    }

    /// Returns the confirmation frame or an error
    fn transmit_frame(&mut self, txf: Frame) -> StResult<Frame> {
        crate::dbg::println!("enqueuing new frame:");
        // crate::dbg::println!("{:?}", txf);
        self.tx_frame_queue.enqueue(txf).unwrap();

        nb::block!(self.send_frame())?;

        self.cmd_tmo.set(CMD_TMO);
        loop {
            if let Some(f) = self.cnf_frame_queue.dequeue() {
                self.cmd_tmo.clear();
                return Ok(f);
            }
            if self.cmd_tmo.is_expired() {
                self.cmd_tmo.clear();
                crate::dbg::println!("cmd_tmo has expired");
                return Err(StErr::ErrTimeout);
            }
        }
    }

    fn send_frame(&mut self) -> nb::Result<(), StErr> {
        use nb::Error::{Other, WouldBlock};
        match self.sf_state {
            TxStatus::TxreqLow => {
                globals::LOCAL_FRAME_TX.reset();
                globals::STATUS_VALUE.dequeue();
                unsafe { globals::T_REQ_PIN.as_mut() }.unwrap().set_low();
                self.status_msg_tmo.set(STATUS_MSG_TMO);
                globals::WAIT_STATUS.set();
                self.sf_state = TxStatus::WaitStatusFrame;
                Err(WouldBlock)
            }
            TxStatus::WaitStatusFrame => {
                if self.status_msg_tmo.is_expired() {
                    crate::dbg::println!("status_msg_tmo has expired");
                    unsafe { globals::T_REQ_PIN.as_mut() }.unwrap().set_high();
                    self.sf_state = TxStatus::TxreqLow;
                    globals::WAIT_STATUS.reset();
                    return Err(Other(StErr::TxErrNoStatus));
                }

                let Some(status) = globals::STATUS_VALUE.dequeue() else { return Err(WouldBlock) };

                if status & BUSY_MASK != 0 {
                    unsafe { globals::T_REQ_PIN.as_mut() }.unwrap().set_high();
                    self.sf_state = TxStatus::TxreqLow;
                    Err(Other(StErr::TxErrBusy))
                } else {
                    self.sf_state = TxStatus::WaitTxFrameDone;
                    globals::TX_ACTIVE.set();
                    unsafe { globals::SERIAL_PLM.as_mut() }
                        .unwrap()
                        .listen(serial::Event::Txe);
                    Err(WouldBlock)
                }
            }
            TxStatus::WaitTxFrameDone => {
                if globals::LOCAL_FRAME_TX.check() {
                    self.ack_tmo.set(ACK_TMO);
                    globals::WAIT_ACK.set();
                    self.sf_state = TxStatus::WaitAck;
                }
                Err(WouldBlock)
            }
            TxStatus::WaitAck => {
                if self.ack_tmo.is_expired() {
                    crate::dbg::println!("ack_tmo has expired");
                    self.sf_state = TxStatus::TxreqLow;
                    globals::WAIT_ACK.reset();
                    return Err(Other(StErr::TxErrAckTmo));
                }

                let ack = match globals::ACK_RX_VALUE.dequeue() {
                    Some(a) => a,
                    None => return Err(WouldBlock),
                };

                self.sf_state = TxStatus::TxreqLow;
                globals::WAIT_ACK.reset();
                if ack == ACK {
                    Ok(())
                } else {
                    Err(Other(StErr::TxErrNak))
                }
            }
        }
    }
}
