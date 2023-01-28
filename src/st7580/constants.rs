/// Acknowledgement codes
pub const ACK: u8 = 0x06;
pub const NAK: u8 = 0x15;
pub const BUSY_MASK: u8 = 0x06;

/// Start of frame codes
pub const STX_02: u8 = 0x02;
pub const STX_03: u8 = 0x03;
pub const STX_STATUS: u8 = 0x3F;

/// Intercharacter timeout msec
pub const IC_TMO: u32 = 10;
/// Command timeout
pub const CMD_TMO: u32 = 4000;
/// Status message timeout
pub const STATUS_MSG_TMO: u32 = 200;

/// Command codes
/// Reset request command
pub const CMD_RESET_REQ: u8 = 0x3C;
/// Reset confirmation command
pub const CMD_RESET_CNF: u8 = 0x3D;
/// Reset indication command
pub const CMD_RESET_IND: u8 = 0x3E;

/// PHY Data indication command
pub const CMD_PHY_DATA_IND: u8 = 0x26;

/// DL Data indication command
pub const CMD_DL_DATA_IND: u8 = 0x52;
/// DL Sniffer indication command
pub const CMD_DL_SNIFFER_IND: u8 = 0x5A;

/// SS Data indication command
pub const CMD_SS_DATA_IND: u8 = 0x56;
/// SS Sniffer indication command
pub const CMD_SS_SNIFFER_IND: u8 = 0x5E;

pub trait IndicationValue {
    fn is_indication(&self) -> bool;
}

impl IndicationValue for u8 {
    #[inline(always)]
    fn is_indication(&self) -> bool {
        matches!(
            *self,
            CMD_RESET_IND
                | CMD_PHY_DATA_IND
                | CMD_DL_DATA_IND
                | CMD_DL_SNIFFER_IND
                | CMD_SS_DATA_IND
                | CMD_SS_SNIFFER_IND
        )
    }
}
