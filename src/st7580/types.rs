#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StErr {
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

impl TryFrom<u8> for StErr {
    type Error = u8;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        use StErr::*;
        match value {
            x if x == TxInProgress as u8 => Ok(TxInProgress),
            x if x == ErrConfirm as u8 => Ok(ErrConfirm),
            x if x == ErrBufLen as u8 => Ok(ErrBufLen),
            x if x == ErrTimeout as u8 => Ok(ErrTimeout),
            x if x == ErrPing as u8 => Ok(ErrPing),
            x if x == ErrArgs as u8 => Ok(ErrArgs),
            x if x == UnexpectedFrame as u8 => Ok(UnexpectedFrame),
            x if x == RcvBufTooSmall as u8 => Ok(RcvBufTooSmall),
            x if x == TxErrNak as u8 => Ok(TxErrNak),
            x if x == TxErrNoStatus as u8 => Ok(TxErrNoStatus),
            x if x == TxErrAckTmo as u8 => Ok(TxErrAckTmo),
            x if x == TxErrBusy as u8 => Ok(TxErrBusy),
            x => Err(x),
        }
    }
}

pub type StResult<T> = core::result::Result<T, StErr>;

/// Frame tx High Level state machine states.
pub enum TxStatus {
    TxreqLow,
    WaitStatusFrame,
    WaitTxFrameDone,
    WaitAck,
}

///  Frame Tx Interrupt Level state machine states.
pub enum TxIrqStatus {
    SendStx,
    SendLength,
    SendCommand,
    SendData,
    SendChecksumLsb,
    SendChecksumMsb,
    TxDone,
}

/// Rx frame state machine states
pub enum RxIrqStatus {
    FirstByte,
    StatusValue,
    Length,
    Command,
    Data,
    ChecksumLsb,
    ChecksumMsb,
}

#[derive(Default, Debug, Clone, Copy)]
pub struct Timeout {
    tmo: u32,
    tmo_start_time: u32,
}

impl Timeout {
    pub fn is_expired(&self) -> bool {
        let Timeout {
            tmo,
            tmo_start_time,
        } = *self;
        if tmo == 0 {
            return false;
        }

        let now = super::globals::now();

        let elapse = if now >= tmo_start_time {
            now - tmo_start_time
        } else {
            now + (u32::MAX - tmo_start_time)
        };
        elapse >= tmo
    }

    pub fn set(&mut self, tmo: u32) {
        *self = Timeout {
            tmo,
            tmo_start_time: super::globals::now(),
        };
    }

    pub fn clear(&mut self) {
        *self = Default::default();
    }
}
