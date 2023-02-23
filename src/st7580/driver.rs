use super::{constants::*, frame::*, globals, mem, types::*};
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

    pub fn reset(&mut self) -> StResult<DSTag> {
        let tx_frame = Frame::new(STX_02, 0, CMD_RESET_REQ, [0; 255]);

        Ok(DSTag(tx_frame, SenderTag::Reset))
    }

    pub fn mib_write(&mut self, idx: u8, buf: &[u8]) -> StResult<DSTag> {
        assert!(buf.len() < 255);
        let mut data = [0; 255];
        data[0] = idx;
        data[1..buf.len() + 1].clone_from_slice(buf);
        let tx_frame =
            Frame::new(STX_02, buf.len() as u8 + 1, CMD_MIB_WRITE_REQ, data);

        Ok(DSTag(tx_frame, SenderTag::MibWrite))
    }

    /*
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
    */

    pub fn mib_erase(&mut self, idx: u8) -> StResult<DSTag> {
        let mut data = [0; 255];
        data[0] = idx;
        let tx_frame = Frame::new(STX_02, 1, CMD_MIB_ERASE_REQ, data);

        Ok(DSTag(tx_frame, SenderTag::MibErase))
    }

    /// Ping ST7580 PLC Modem.
    ///
    /// # Arguments
    ///
    /// * `buf` - buffer containing ping test data to be sent. If ping is
    ///   success ST7580 PLC Modem will reply with the same data.
    pub fn ping(&mut self, buf: mem::BufBox) -> StResult<DSTag> {
        assert!(buf.len() < 255);
        let mut data = [0; 255];
        data[..buf.len()].clone_from_slice(&buf);
        let tx_frame = Frame::new(STX_02, buf.len() as u8, CMD_PING_REQ, data);

        Ok(DSTag(tx_frame, SenderTag::Ping(buf)))
    }

    pub fn phy_data(
        &mut self,
        plm_opts: u8,
        send_buf: mem::BufBox,
    ) -> StResult<DSTag> {
        self.impl_phy_dl_data::<PHY_DATALEN_MAX, CMD_PHY_DATA_REQ>(
            plm_opts,
            send_buf,
            SenderTag::PhyData,
        )
    }

    pub fn dl_data(
        &mut self,
        plm_opts: u8,
        send_buf: mem::BufBox,
    ) -> StResult<DSTag> {
        self.impl_phy_dl_data::<DL_DATALEN_MAX, CMD_DL_DATA_REQ>(
            plm_opts,
            send_buf,
            SenderTag::DlData,
        )
    }

    #[inline(always)]
    fn impl_phy_dl_data<const LEN_MAX: usize, const REQ: u8>(
        &mut self,
        plm_opts: u8,
        send_buf: mem::BufBox,
        tag: SenderTag,
    ) -> StResult<DSTag> {
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

        data[offset..send_buf.len() + offset].clone_from_slice(&send_buf);

        let tx_frame = Frame::new(STX_02, send_buf.len() as u8 + 1, REQ, data);

        Ok(DSTag(tx_frame, tag))
    }

    /*
    pub fn ss_data(
        &mut self,
        plm_opts: u8,
        send_buf: &[u8],
        clr_len: u8,
        enc_len: u8,
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
        Ok(())
    }
    */

    #[inline(always)]
    pub fn receive_frame(&mut self) -> Option<Frame> {
        self.ind_frame_queue.dequeue()
    }
}

pub struct DSTag(Frame, SenderTag);

pub struct DSender {
    sf_state: TxStatus,
    tag: SenderTag,

    tx_frame_queue: globals::FrameProducer<2>,
    cnf_frame_queue: globals::FrameConsumer<2>,

    ack_tmo: Timeout,
    cmd_tmo: Timeout,
    status_msg_tmo: Timeout,
}

impl DSender {
    pub(super) fn new() -> Self {
        DSender {
            sf_state: TxStatus::TxreqLow,
            tag: SenderTag::Inactive,
            tx_frame_queue: unsafe { globals::TX_FRAME.split() }.0,
            cnf_frame_queue: unsafe { globals::CONFIRM_FRAME.split() }.1,
            ack_tmo: Default::default(),
            cmd_tmo: Default::default(),
            status_msg_tmo: Default::default(),
        }
    }

    pub fn is_active(&self) -> bool {
        !matches!(self.tag, SenderTag::Inactive)
    }

    pub fn enqueue(&mut self, tag: DSTag) -> StResult<&mut Self> {
        assert!(
            !self.is_active() && matches!(self.sf_state, TxStatus::TxreqLow)
        );
        let DSTag(frame, tag) = tag;
        self.tx_frame_queue.enqueue(frame).unwrap();
        self.tag = tag;
        Ok(self)
    }

    fn send_frame(&mut self) -> NbStResult<Frame> {
        use nb::Error::WouldBlock;

        match self.sf_state {
            TxStatus::TxreqLow => {
                globals::LOCAL_FRAME_TX.clear();
                globals::STATUS_VALUE.dequeue();
                unsafe { globals::T_REQ_PIN.as_mut() }.unwrap().set_low();
                self.status_msg_tmo.set(STATUS_MSG_TMO);
                globals::WAIT_STATUS.set_signal();
                self.sf_state = TxStatus::WaitStatusFrame;
                Err(WouldBlock)
            }
            TxStatus::WaitStatusFrame if self.status_msg_tmo.is_expired() => {
                unsafe { globals::T_REQ_PIN.as_mut() }.unwrap().set_high();
                self.sf_state = TxStatus::TxreqLow;
                globals::WAIT_STATUS.clear();
                Err(StErr::TxErrNoStatus.into())
            }
            TxStatus::WaitStatusFrame => {
                let status = globals::STATUS_VALUE.dequeue();
                let Some(status) = status else { return Err(WouldBlock) };

                if status & BUSY_MASK != 0 {
                    unsafe { globals::T_REQ_PIN.as_mut() }.unwrap().set_high();
                    self.sf_state = TxStatus::TxreqLow;
                    Err(StErr::TxErrBusy.into())
                } else {
                    self.sf_state = TxStatus::WaitTxFrameDone;
                    globals::TX_ACTIVE.set_signal();
                    unsafe { globals::SERIAL_PLM.as_mut() }
                        .unwrap()
                        .listen(serial::Event::Txe);
                    Err(WouldBlock)
                }
            }
            TxStatus::WaitTxFrameDone
                if globals::LOCAL_FRAME_TX.take_signal() =>
            {
                self.ack_tmo.set(ACK_TMO);
                globals::WAIT_ACK.set_signal();
                self.sf_state = TxStatus::WaitAck;
                Err(WouldBlock)
            }
            TxStatus::WaitTxFrameDone => Err(WouldBlock),
            TxStatus::WaitAck if self.ack_tmo.is_expired() => {
                self.sf_state = TxStatus::TxreqLow;
                globals::WAIT_ACK.clear();
                Err(StErr::TxErrAckTmo.into())
            }
            TxStatus::WaitAck => {
                let ack = globals::ACK_RX_VALUE.dequeue();
                let Some(ack) = ack else { return Err(WouldBlock) };

                globals::WAIT_ACK.clear();
                if ack == ACK {
                    self.cmd_tmo.set(CMD_TMO);
                    self.sf_state = TxStatus::WaitCnf;
                    Err(WouldBlock)
                } else {
                    self.sf_state = TxStatus::TxreqLow;
                    Err(StErr::TxErrNak.into())
                }
            }
            TxStatus::WaitCnf if self.cmd_tmo.is_expired() => {
                self.cmd_tmo.clear();
                Err(StErr::ErrTimeout.into())
            }
            TxStatus::WaitCnf => {
                self.cnf_frame_queue.dequeue().ok_or(WouldBlock).map(|f| {
                    self.cmd_tmo.clear();
                    f
                })
            }
        }
    }

    pub fn process(&mut self) -> NbStResult<()> {
        use nb::Error::Other;

        let cnf_frame = self.send_frame()?;
        let mut tag = SenderTag::Inactive;
        core::mem::swap(&mut self.tag, &mut tag);

        macro_rules! def_case {
            ($err:ident, $cnf:ident) => {
                match cnf_frame.command {
                    $err => Err(Other(cnf_frame.data[0].try_into().unwrap())),
                    $cnf => Ok(()),
                    _ => Err(Other(StErr::ErrConfirm.into())),
                }
            };
        }
        match tag {
            SenderTag::Inactive => unreachable!(),
            SenderTag::Reset => {
                def_case!(CMD_RESET_ERR, CMD_RESET_CNF)
            }
            SenderTag::MibWrite => {
                def_case!(CMD_MIB_WRITE_ERR, CMD_MIB_WRITE_CNF)
            }
            SenderTag::MibErase => {
                def_case!(CMD_MIB_ERASE_ERR, CMD_MIB_ERASE_CNF)
            }
            SenderTag::DlData => {
                def_case!(CMD_DL_DATA_ERR, CMD_DL_DATA_CNF)
            }
            SenderTag::PhyData => {
                def_case!(CMD_PHY_DATA_ERR, CMD_PHY_DATA_CNF)
            }
            SenderTag::Ping(buf) => {
                if cnf_frame.command != CMD_PING_CNF {
                    Err(Other(cnf_frame.data[0].try_into().unwrap()))
                } else if &cnf_frame.data[..buf.len()] != &buf[..buf.len()] {
                    Err(StErr::ErrPing.into())
                } else {
                    Ok(())
                }
            }
        }
    }
}
