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

pub type Result<T> = core::result::Result<T, StErr>;

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
        let now = super::globals::now();
        let Timeout {
            tmo,
            tmo_start_time,
        } = *self;
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
}
