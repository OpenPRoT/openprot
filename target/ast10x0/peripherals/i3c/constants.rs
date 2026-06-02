// Licensed under the Apache-2.0 license

//! I3C hardware constants and register definitions
//!
//! # Register Map
//! | Offset | Register              | Description                    |
//! |--------|-----------------------|--------------------------------|
//! | 0x0C   | COMMAND_QUEUE_PORT    | Command queue port             |
//! | 0x10   | RESPONSE_QUEUE_PORT   | Response queue port            |
//! | 0x18   | IBI_QUEUE_STATUS      | IBI queue status               |
//! | 0x3C   | INTR_STATUS           | Interrupt status               |
//! | 0x40   | INTR_STATUS_EN        | Interrupt status enable        |
//! | 0x44   | INTR_SIGNAL_EN        | Interrupt signal enable        |

// =============================================================================
// Message Flags
// =============================================================================

/// I3C message write flag
pub const I3C_MSG_WRITE: u8 = 0x0;
/// I3C message read flag
pub const I3C_MSG_READ: u8 = 0x1;
/// I3C message stop flag
pub const I3C_MSG_STOP: u8 = 0x2;

// =============================================================================
// I2C Timing Constants (nanoseconds)
// =============================================================================

// Standard mode (100 kHz)
pub const I3C_BUS_I2C_STD_TLOW_MIN_NS: u32 = 4_700;
pub const I3C_BUS_I2C_STD_THIGH_MIN_NS: u32 = 4_000;
pub const I3C_BUS_I2C_STD_TR_MAX_NS: u32 = 1_000;
pub const I3C_BUS_I2C_STD_TF_MAX_NS: u32 = 300;

// Fast mode (400 kHz)
pub const I3C_BUS_I2C_FM_TLOW_MIN_NS: u32 = 1_300;
pub const I3C_BUS_I2C_FM_THIGH_MIN_NS: u32 = 600;
pub const I3C_BUS_I2C_FM_TR_MAX_NS: u32 = 300;
pub const I3C_BUS_I2C_FM_TF_MAX_NS: u32 = 300;

// Fast mode plus (1 MHz)
pub const I3C_BUS_I2C_FMP_TLOW_MIN_NS: u32 = 500;
pub const I3C_BUS_I2C_FMP_THIGH_MIN_NS: u32 = 260;
pub const I3C_BUS_I2C_FMP_TR_MAX_NS: u32 = 120;
pub const I3C_BUS_I2C_FMP_TF_MAX_NS: u32 = 120;

// I3C timing
pub const I3C_BUS_THIGH_MAX_NS: u32 = 41;

/// Nanoseconds per second
pub const NSEC_PER_SEC: u32 = 1_000_000_000;

// =============================================================================
// SDA TX Hold Configuration
// =============================================================================

pub const SDA_TX_HOLD_MIN: u32 = 0b001;
pub const SDA_TX_HOLD_MAX: u32 = 0b111;
pub const SDA_TX_HOLD_MASK: u32 = 0x0007_0000; // bits 18:16

// =============================================================================
// Slave Configuration
// =============================================================================

pub const SLV_DCR_MASK: u32 = 0x0000_ff00;
pub const SLV_EVENT_CTRL: u32 = 0x38;
pub const SLV_EVENT_CTRL_MWL_UPD: u32 = bit(7);
pub const SLV_EVENT_CTRL_MRL_UPD: u32 = bit(6);
pub const SLV_EVENT_CTRL_HJ_REQ: u32 = bit(3);
pub const SLV_EVENT_CTRL_SIR_EN: u32 = bit(0);

// =============================================================================
// I3C Global Register Bits
// =============================================================================

pub const I3CG_REG1_SCL_IN_SW_MODE_VAL: u32 = bit(23);
pub const I3CG_REG1_SDA_IN_SW_MODE_VAL: u32 = bit(27);
pub const I3CG_REG1_SCL_IN_SW_MODE_EN: u32 = bit(28);
pub const I3CG_REG1_SDA_IN_SW_MODE_EN: u32 = bit(29);

// =============================================================================
// Transfer Status
// =============================================================================

pub const CM_TFR_STS_MASTER_HALT: u8 = 0xf;
pub const CM_TFR_STS_TARGET_HALT: u8 = 0x6;

// =============================================================================
// Command Queue Port (0x0C)
// =============================================================================

pub const COMMAND_QUEUE_PORT: u32 = 0x0c;

// Command port bit flags
pub const COMMAND_PORT_PEC: u32 = bit(31);
pub const COMMAND_PORT_TOC: u32 = bit(30);
pub const COMMAND_PORT_READ_TRANSFER: u32 = bit(28);
pub const COMMAND_PORT_SDAP: u32 = bit(27);
pub const COMMAND_PORT_ROC: u32 = bit(26);
pub const COMMAND_PORT_DBP: u32 = bit(25);
pub const COMMAND_PORT_CP: u32 = bit(15);

// Command port field masks
pub const COMMAND_PORT_SPEED: u32 = bits(23, 21);
pub const COMMAND_PORT_DEV_INDEX: u32 = bits(20, 16);
pub const COMMAND_PORT_CMD: u32 = bits(14, 7);
pub const COMMAND_PORT_TID: u32 = bits(6, 3);
pub const COMMAND_PORT_ARG_DB: u32 = bits(15, 8);
pub const COMMAND_PORT_ARG_DATA_LEN: u32 = bits(31, 16);
pub const COMMAND_PORT_ATTR: u32 = bits(2, 0);
pub const COMMAND_PORT_DEV_COUNT: u32 = bits(25, 21);

// =============================================================================
// Transaction IDs
// =============================================================================

pub const TID_TARGET_IBI: u32 = 0x1;
pub const TID_TARGET_RD_DATA: u32 = 0x2;
pub const TID_TARGET_MASTER_WR: u32 = 0x8;
pub const TID_TARGET_MASTER_DEF: u32 = 0xf;

// =============================================================================
// Command Attributes
// =============================================================================

pub const COMMAND_ATTR_XFER_CMD: u32 = 0;
pub const COMMAND_ATTR_XFER_ARG: u32 = 1;
pub const COMMAND_ATTR_SHORT_ARG: u32 = 2;
pub const COMMAND_ATTR_ADDR_ASSGN_CMD: u32 = 3;
pub const COMMAND_ATTR_SLAVE_DATA_CMD: u32 = 0;

// =============================================================================
// Device Address Table
// =============================================================================

pub const DEV_ADDR_TABLE_LEGACY_I2C_DEV: u32 = bit(31);
pub const DEV_ADDR_TABLE_DYNAMIC_ADDR: u32 = bits(23, 16);
pub const DEV_ADDR_TABLE_MR_REJECT: u32 = bit(14);
pub const DEV_ADDR_TABLE_SIR_REJECT: u32 = bit(13);
pub const DEV_ADDR_TABLE_IBI_MDB: u32 = bit(12);
pub const DEV_ADDR_TABLE_IBI_PEC: u32 = bit(11);
pub const DEV_ADDR_TABLE_STATIC_ADDR: u32 = bits(6, 0);

// =============================================================================
// IBI Queue Status (0x18)
// =============================================================================

pub const IBI_QUEUE_STATUS: u32 = 0x18;
pub const IBIQ_STATUS_IBI_ID: u32 = bits(15, 8);
pub const IBIQ_STATUS_IBI_ID_SHIFT: u32 = 8;
pub const IBIQ_STATUS_IBI_DATA_LEN: u32 = bits(7, 0);
pub const IBIQ_STATUS_IBI_DATA_LEN_SHIFT: u32 = 0;

// =============================================================================
// Reset Control
// =============================================================================

pub const RESET_CTRL_IBI_QUEUE: u32 = bit(5);
pub const RESET_CTRL_RX_FIFO: u32 = bit(4);
pub const RESET_CTRL_TX_FIFO: u32 = bit(3);
pub const RESET_CTRL_RESP_QUEUE: u32 = bit(2);
pub const RESET_CTRL_CMD_QUEUE: u32 = bit(1);
pub const RESET_CTRL_SOFT: u32 = bit(0);

pub const RESET_CTRL_ALL: u32 = RESET_CTRL_IBI_QUEUE
    | RESET_CTRL_RX_FIFO
    | RESET_CTRL_TX_FIFO
    | RESET_CTRL_RESP_QUEUE
    | RESET_CTRL_CMD_QUEUE
    | RESET_CTRL_SOFT;

pub const RESET_CTRL_QUEUES: u32 = RESET_CTRL_IBI_QUEUE
    | RESET_CTRL_RX_FIFO
    | RESET_CTRL_TX_FIFO
    | RESET_CTRL_RESP_QUEUE
    | RESET_CTRL_CMD_QUEUE;

pub const RESET_CTRL_XFER_QUEUES: u32 =
    RESET_CTRL_RX_FIFO | RESET_CTRL_TX_FIFO | RESET_CTRL_RESP_QUEUE | RESET_CTRL_CMD_QUEUE;

// =============================================================================
// Response Queue Port (0x10)
// =============================================================================

pub const RESPONSE_QUEUE_PORT: u32 = 0x10;
pub const RESPONSE_PORT_ERR_STATUS_SHIFT: u32 = 28;
pub const RESPONSE_PORT_ERR_STATUS_MASK: u32 = genmask(31, 28);
pub const RESPONSE_PORT_TID_SHIFT: u32 = 24;
pub const RESPONSE_PORT_TID_MASK: u32 = genmask(27, 24);
pub const RESPONSE_PORT_DATA_LEN_SHIFT: u32 = 0;
pub const RESPONSE_PORT_DATA_LEN_MASK: u32 = genmask(15, 0);

// Response error codes
pub const RESPONSE_NO_ERROR: u32 = 0;
pub const RESPONSE_ERROR_CRC: u32 = 1;
pub const RESPONSE_ERROR_PARITY: u32 = 2;
pub const RESPONSE_ERROR_FRAME: u32 = 3;
pub const RESPONSE_ERROR_IBA_NACK: u32 = 4;
pub const RESPONSE_ERROR_ADDRESS_NACK: u32 = 5;
pub const RESPONSE_ERROR_OVER_UNDER_FLOW: u32 = 6;
pub const RESPONSE_ERROR_TRANSF_ABORT: u32 = 8;
pub const RESPONSE_ERROR_I2C_W_NACK_ERR: u32 = 9;
pub const RESPONSE_ERROR_EARLY_TERMINATE: u32 = 10;
pub const RESPONSE_ERROR_PEC_ERR: u32 = 12;

// =============================================================================
// Interrupt Registers (0x3C - 0x48)
// =============================================================================

pub const INTR_STATUS: u32 = 0x3c;
pub const INTR_STATUS_EN: u32 = 0x40;
pub const INTR_SIGNAL_EN: u32 = 0x44;
pub const INTR_FORCE: u32 = 0x48;

// Interrupt status bits
pub const INTR_BUSOWNER_UPDATE_STAT: u32 = bit(13);
pub const INTR_IBI_UPDATED_STAT: u32 = bit(12);
pub const INTR_READ_REQ_RECV_STAT: u32 = bit(11);
pub const INTR_DEFSLV_STAT: u32 = bit(10);
pub const INTR_TRANSFER_ERR_STAT: u32 = bit(9);
pub const INTR_DYN_ADDR_ASSGN_STAT: u32 = bit(8);
pub const INTR_CCC_UPDATED_STAT: u32 = bit(6);
pub const INTR_TRANSFER_ABORT_STAT: u32 = bit(5);
pub const INTR_RESP_READY_STAT: u32 = bit(4);
pub const INTR_CMD_QUEUE_READY_STAT: u32 = bit(3);
pub const INTR_IBI_THLD_STAT: u32 = bit(2);
pub const INTR_RX_THLD_STAT: u32 = bit(1);
pub const INTR_TX_THLD_STAT: u32 = bit(0);

// BCR bits
pub const I3C_BCR_IBI_PAYLOAD_HAS_DATA_BYTE: u32 = bit(2);

// =============================================================================
// Address Constants
// =============================================================================

/// I3C broadcast address
pub const I3C_BROADCAST_ADDR: u8 = 0x7E;
/// Maximum I3C address
pub const I3C_MAX_ADDR: u8 = 0x7F;

// =============================================================================
// Hardware Limits
// =============================================================================

/// Maximum number of commands in a single transfer
pub const MAX_CMDS: usize = 32;
/// Maximum number of I3C buses supported
pub const MAX_BUSES: usize = 4;
/// Maximum devices per bus
pub const MAX_DEVICES_PER_BUS: usize = 8;

// =============================================================================
// CCC (Common Command Code) Constants
// =============================================================================

pub const I3C_CCC_RSTDAA: u8 = 0x06;
pub const I3C_CCC_ENTDAA: u8 = 0x07;
pub const I3C_CCC_SETHID: u8 = 0x61;
pub const I3C_CCC_DEVCTRL: u8 = 0x62;
pub const I3C_CCC_SETDASA: u8 = 0x87;
pub const I3C_CCC_SETNEWDA: u8 = 0x88;
pub const I3C_CCC_GETPID: u8 = 0x8D;
pub const I3C_CCC_GETBCR: u8 = 0x8E;
pub const I3C_CCC_GETSTATUS: u8 = 0x90;

// CCC event bits
pub const I3C_CCC_EVT_INTR: u8 = 1 << 0;
pub const I3C_CCC_EVT_CR: u8 = 1 << 1;
pub const I3C_CCC_EVT_HJ: u8 = 1 << 3;
pub const I3C_CCC_EVT_ALL: u8 = I3C_CCC_EVT_INTR | I3C_CCC_EVT_CR | I3C_CCC_EVT_HJ;

// =============================================================================
// Helper Functions
// =============================================================================

/// Create a single bit mask at position `n`
#[inline]
#[must_use]
pub const fn bit(n: u32) -> u32 {
    1 << n
}

/// Create a bit mask from bit `l` to bit `h` (inclusive)
#[inline]
#[must_use]
pub const fn bits(h: u32, l: u32) -> u32 {
    ((1u32 << (h - l + 1)) - 1) << l
}

/// Prepare a value for a masked field
#[inline]
#[must_use]
pub const fn field_prep(mask: u32, val: u32) -> u32 {
    (val << mask.trailing_zeros()) & mask
}

/// Extract a value from a masked field
#[inline]
#[must_use]
pub const fn field_get(val: u32, mask: u32, shift: u32) -> u32 {
    (val & mask) >> shift
}

/// Generate a mask from MSB to LSB
#[inline]
#[must_use]
pub const fn genmask(msb: u32, lsb: u32) -> u32 {
    let width = msb - lsb + 1;
    if width >= 32 {
        u32::MAX
    } else {
        ((1u32 << width) - 1) << lsb
    }
}
