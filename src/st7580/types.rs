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
