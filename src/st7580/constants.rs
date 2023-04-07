#[cfg(feature = "CUSTOM_MIB_FREQUENCY")]
pub const TXFREQS: [u8; 3] = [0, 0, 0];

#[cfg(feature = "GAIN_SELECTOR")]
pub const TXGAIN: u8 = 0;

/// ST7580 PHY configuration parameters fitting
pub const PHY_CONFIG: [u8; 14] = [
    0x01, 0xC9, 0x08, 0x01, 0x8E, 0x70, 0x0E, 0x15, 0x00, 0x00, 0x02, 0x35,
    0x9B, 0x58,
];

/// ST7580 MODEM configuration parameters fitting
/// Use PHY data
#[cfg(all())]
pub const MODEM_CONFIG: [u8; 1] = [0x00];
/// Use DL data
#[cfg(any())]
pub const MODEM_CONFIG: [u8; 1] = [0x11];

/// MIBs Objects
/// Modem configuration MIB
pub const MIB_MODEM_CONF: u8 = 0x00;
/// PHY configuration MIB
pub const MIB_PHY_CONF: u8 = 0x01;
/// SS key MIB
pub const MIB_SS_KEY: u8 = 0x02;
/// Last data indication MIB
pub const MIB_LAST_DATA_IND: u8 = 0x04;
/// Last TX confirm MIB
pub const MIB_LAST_TX_CNF: u8 = 0x05;
/// PHY Data MIB
pub const MIB_PHY_DATA: u8 = 0x06;
/// DL Data MIB
pub const MIB_DL_DATA: u8 = 0x07;
/// SS Data MIB
pub const MIB_SS_DATA: u8 = 0x08;
/// Host interface timeout MIB
pub const MIB_HOST_IF_TOUT: u8 = 0x09;
/// Firmware version MIB
pub const MIB_FW_VERSION: u8 = 0x0A;

/// Acknowledgement codes
pub const ACK: u8 = 0x06;
pub const NAK: u8 = 0x15;
pub const BUSY_MASK: u8 = 0x06;

/// Start of frame codes
pub const STX_02: u8 = 0x02;
pub const STX_03: u8 = 0x03;
pub const STX_STATUS: u8 = 0x3F;

pub const PHY_DATALEN_MAX: usize = 250;
pub const DL_DATALEN_MAX: usize = 242;
pub const SS_DATALEN_MAX: usize = 226;

pub const PHY_DL_SS_RET_LEN: usize = 5;

/// Intercharacter timeout msec
pub const IC_TMO: u32 = 10;
/// ACK timeout msec
pub const ACK_TMO: u32 = 40;
/// Status message timeout
pub const STATUS_MSG_TMO: u32 = 200;

/// Command timeout
pub const CMD_TMO: u32 = 4000;

/// Command codes
/// Reset request command
pub const CMD_RESET_REQ: u8 = 0x3C;
/// Reset confirmation command
pub const CMD_RESET_CNF: u8 = 0x3D;
/// Reset indication command
pub const CMD_RESET_IND: u8 = 0x3E;
/// Reset error command code
#[allow(unused)]
pub const CMD_RESET_ERR: u8 = 0x3F;

/// MIB Write request command
pub const CMD_MIB_WRITE_REQ: u8 = 0x08;
/// MIB Write confirmation command
pub const CMD_MIB_WRITE_CNF: u8 = 0x09;
/// MIB Write error command
pub const CMD_MIB_WRITE_ERR: u8 = 0x0B;

/// MIB Read request command
pub const CMD_MIB_READ_REQ: u8 = 0x0C;
/// MIB Read confirmation command
pub const CMD_MIB_READ_CNF: u8 = 0x0D;
/// MIB Read error command
pub const CMD_MIB_READ_ERR: u8 = 0x0F;

/// MIB Erase request command
pub const CMD_MIB_ERASE_REQ: u8 = 0x10;
/// MIB Erase confirmation command
pub const CMD_MIB_ERASE_CNF: u8 = 0x11;
/// MIB Erase error command
pub const CMD_MIB_ERASE_ERR: u8 = 0x13;

/// PING request command
pub const CMD_PING_REQ: u8 = 0x2C; /* PING request command */
/// PING confirmation command
pub const CMD_PING_CNF: u8 = 0x2D; /* PING confirmation command */

/// PHY Data request command
pub const CMD_PHY_DATA_REQ: u8 = 0x24;
/// PHY Data confirmation command
pub const CMD_PHY_DATA_CNF: u8 = 0x25;
/// PHY Data indication command
pub const CMD_PHY_DATA_IND: u8 = 0x26;
/// PHY Data error command
pub const CMD_PHY_DATA_ERR: u8 = 0x27;

/// DL Data request command
pub const CMD_DL_DATA_REQ: u8 = 0x50;
/// DL Data confirmation command
pub const CMD_DL_DATA_CNF: u8 = 0x51;
/// DL Data indication command
pub const CMD_DL_DATA_IND: u8 = 0x52;
/// DL Data error command
pub const CMD_DL_DATA_ERR: u8 = 0x53;
/// DL Sniffer indication command
pub const CMD_DL_SNIFFER_IND: u8 = 0x5A;

/// SS Data request command
pub const CMD_SS_DATA_REQ: u8 = 0x54;
/// SS Data confirmation command
pub const CMD_SS_DATA_CNF: u8 = 0x55;
/// SS Data indication command
pub const CMD_SS_DATA_IND: u8 = 0x56;
/// SS Data error command
pub const CMD_SS_DATA_ERR: u8 = 0x57;
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

pub trait AckValue {
    fn to_ack(self) -> u8;
}

impl AckValue for bool {
    fn to_ack(self) -> u8 {
        if self {
            ACK
        } else {
            NAK
        }
    }
}
