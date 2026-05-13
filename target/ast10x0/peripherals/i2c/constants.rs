// Licensed under the Apache-2.0 license

//! Hardware constants for AST1060 I2C controller
//!
//! # Reference Implementation
//!
//! Constants derived from the original working code:
//! - **aspeed-rust/src/i2c/ast1060_i2c.rs** lines 55-100
//!
//! # Register Map (New Register Definition Mode)
//!
//! | Offset | Register | Description |
//! |--------|----------|-------------|
//! | 0x00   | I2CC00   | Function Control Register |
//! | 0x04   | I2CC04   | AC Timing Register |
//! | 0x08   | I2CC08   | Byte Buffer / Line Status Register |
//! | 0x0C   | I2CC0C   | Pool Buffer Control Register |
//! | 0x10   | I2CM10   | Master Interrupt Control Register |
//! | 0x14   | I2CM14   | Master Interrupt Status Register (write-to-clear) |
//! | 0x18   | I2CM18   | Master Command Register |
//! | 0x1C   | I2CM1C   | Master DMA Length Register |
//!
//! # Critical Register Notes
//!
//! - **i2cm18**: Command register - write all command bits here
//! - **i2cm14**: Status register - read status, write to clear interrupts
//! - **i2cc08**: Byte data register for byte mode (`tx_byte_buffer`, `rx_byte_buffer`)
//! - **i2cc0c**: Buffer size register for buffer mode (`tx_data_byte_count`, `rx_pool_buffer_size`)

/// HPLL frequency (1 `GHz`)
pub const HPLL_FREQ: u32 = 1_000_000_000;

/// ASPEED I2C bus clock (100 `MHz` typical)
pub const ASPEED_I2C_BUS_CLK_HZ: u32 = 100_000_000;

/// Standard mode speed (100 kHz)
pub const I2C_STANDARD_MODE_HZ: u32 = 100_000;

/// Fast mode speed (400 kHz)
pub const I2C_FAST_MODE_HZ: u32 = 400_000;

/// Fast-plus mode speed (1 `MHz`)
pub const I2C_FAST_PLUS_MODE_HZ: u32 = 1_000_000;

/// Buffer mode maximum size (32 bytes)
pub const BUFFER_MODE_SIZE: usize = 32;

/// DMA mode maximum transfer size (4096 bytes, hardware limit)
pub const DMA_MODE_MAX_SIZE: usize = 4096;

/// I2C buffer size register value
/// Reference: `ast1060_i2c.rs:97`
pub const I2C_BUF_SIZE: u8 = 0x20;

/// Default timeout in microseconds
pub const DEFAULT_TIMEOUT_US: u32 = 1_000_000;

/// Maximum retry attempts for operations
pub const MAX_RETRY_ATTEMPTS: u32 = 3;

// ============================================================================
// Register bit definitions
// Reference: aspeed-rust/src/i2c/ast1060_i2c.rs lines 55-100
// ============================================================================

// Function Control Register (I2CC00)
/// Enable slave function
pub const AST_I2CC_SLAVE_EN: u32 = 1 << 1;
/// Enable master function
pub const AST_I2CC_MASTER_EN: u32 = 1 << 0;
/// Disable multi-master capability
pub const AST_I2CC_MULTI_MASTER_DIS: u32 = 1 << 15;
/// Enable save address byte into buffer for Slave Packet mode receive command (I2CC00 bit 20)
/// Per AST1060 datasheet section 13.3.1, when this bit is set, the slave address byte
/// is saved into the receive buffer in slave packet mode.
pub const AST_I2CC_SLAVE_PKT_SAVE_ADDR: u32 = 1 << 20;

// Master Command Register (I2CM18) bit definitions
// Reference: ast1060_i2c.rs:58-69
/// Enable packet mode
pub const AST_I2CM_PKT_EN: u32 = 1 << 16;
/// Enable RX DMA mode
pub const AST_I2CM_RX_DMA_EN: u32 = 1 << 9;
/// Enable TX DMA mode
pub const AST_I2CM_TX_DMA_EN: u32 = 1 << 8;
/// Enable RX buffer mode
pub const AST_I2CM_RX_BUFF_EN: u32 = 1 << 7;
/// Enable TX buffer mode
pub const AST_I2CM_TX_BUFF_EN: u32 = 1 << 6;
/// Send STOP condition
pub const AST_I2CM_STOP_CMD: u32 = 1 << 5;
/// RX command with NACK (last byte)
pub const AST_I2CM_RX_CMD_LAST: u32 = 1 << 4;
/// RX command with ACK
pub const AST_I2CM_RX_CMD: u32 = 1 << 3;
/// TX command
pub const AST_I2CM_TX_CMD: u32 = 1 << 1;
/// Send START condition
pub const AST_I2CM_START_CMD: u32 = 1 << 0;

// Master Interrupt Status Register (I2CM14) bit definitions
// Reference: ast1060_i2c.rs:71-77
// NOTE: These bits overlap with command bits but have different meanings when read from i2cm14
/// SCL low timeout
pub const AST_I2CM_SCL_LOW_TO: u32 = 1 << 6;
/// Abnormal STOP condition
pub const AST_I2CM_ABNORMAL: u32 = 1 << 5;
/// Normal STOP condition
pub const AST_I2CM_NORMAL_STOP: u32 = 1 << 4;
/// Arbitration loss
pub const AST_I2CM_ARBIT_LOSS: u32 = 1 << 3;
/// RX transfer done
pub const AST_I2CM_RX_DONE: u32 = 1 << 2;
/// TX received NACK
pub const AST_I2CM_TX_NAK: u32 = 1 << 1;
/// TX received ACK
pub const AST_I2CM_TX_ACK: u32 = 1 << 0;

// Packet mode status (I2CM14 bits 12-17)
// Reference: ast1060_i2c.rs:86-92
/// Packet mode error
pub const AST_I2CM_PKT_ERROR: u32 = 1 << 17;
/// Packet mode done
pub const AST_I2CM_PKT_DONE: u32 = 1 << 16;
/// Bus recovery failed
pub const AST_I2CM_BUS_RECOVER_FAIL: u32 = 1 << 15;
/// SDA data line timeout
pub const AST_I2CM_SDA_DL_TO: u32 = 1 << 14;
/// Bus recovery done
pub const AST_I2CM_BUS_RECOVER: u32 = 1 << 13;
/// `SMBus` alert
pub const AST_I2CM_SMBUS_ALT: u32 = 1 << 12;

// Slave mode constants
// Reference: ast1060_i2c.rs:83-84 and 99-125
/// Enable slave packet mode
pub const AST_I2CS_PKT_MODE_EN: u32 = 1 << 16;
/// Active on all addresses
pub const AST_I2CS_ACTIVE_ALL: u32 = 0x3 << 17;
/// Enable slave RX DMA
pub const AST_I2CS_RX_DMA_EN: u32 = 1 << 9;
/// Enable slave TX DMA
pub const AST_I2CS_TX_DMA_EN: u32 = 1 << 8;
/// Enable slave RX buffer
pub const AST_I2CS_RX_BUFF_EN: u32 = 1 << 7;
/// Enable slave TX buffer
pub const AST_I2CS_TX_BUFF_EN: u32 = 1 << 6;
/// Slave TX command
pub const AST_I2CS_TX_CMD: u32 = 1 << 2;
/// Slave address matched
pub const AST_I2CS_SLAVE_MATCH: u32 = 1 << 7;
/// STOP condition received
pub const AST_I2CS_STOP: u32 = 1 << 4;
/// Slave RX done NACK
pub const AST_I2CS_RX_DONE_NAK: u32 = 1 << 3;
/// Slave RX done
pub const AST_I2CS_RX_DONE: u32 = 1 << 2;
/// Slave TX got NACK
pub const AST_I2CS_TX_NAK: u32 = 1 << 1;
/// Slave TX got ACK
pub const AST_I2CS_TX_ACK: u32 = 1 << 0;
/// Slave inactive timeout
pub const AST_I2CS_INACTIVE_TO: u32 = 1 << 15;
/// Slave packet mode done
pub const AST_I2CS_PKT_DONE: u32 = 1 << 16;
/// Slave packet mode error
pub const AST_I2CS_PKT_ERROR: u32 = 1 << 17;
/// Waiting for TX DMA
pub const AST_I2CS_WAIT_TX_DMA: u32 = 1 << 25;
/// Waiting for RX DMA
pub const AST_I2CS_WAIT_RX_DMA: u32 = 1 << 24;

/// Helper to build packet mode address field
#[inline]
#[must_use]
pub fn ast_i2cm_pkt_addr(addr: u8) -> u32 {
    u32::from(addr & 0x7F) << 24
}
