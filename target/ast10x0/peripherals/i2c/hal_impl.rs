// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! embedded-hal trait implementations for AST1060 I2C driver
//!
//! This module provides [embedded-hal](https://docs.rs/embedded-hal) trait implementations
//! for the AST1060 I2C controller, enabling compatibility with the broader Rust embedded
//! ecosystem.
//!
//! # Implementation Status
//!
//! Currently implements:
//! - [`embedded_hal::i2c::I2c`] - Master mode I2C operations
//! - [`embedded_hal::i2c::Error`] - Standard error type mapping
//!
//! This allows the AST1060 I2C driver to be used with any crate that accepts embedded-hal
//! I2C traits, such as device drivers, protocols, and middleware.
//!
//! # Hardware Features
//!
//! The AST1060 I2C controller supports:
//! - **14 independent I2C controllers**
//! - **Buffer mode with 32-byte FIFO** for efficient transfers
//! - **Multi-master capability** with arbitration
//! - **Standard (100 kHz) and Fast (400 kHz) modes**
//! - **Slave/target mode** with hardware packet mode and interrupts
//! - **`SMBus` protocol support**
//! - **Automatic bus recovery**
//!
//! # Slave Mode API
//!
//! Slave functionality is provided through hardware-specific methods:
//! - [`configure_slave()`](super::controller::Ast1060I2c::configure_slave) - Configure slave address and mode
//! - [`slave_read()`](super::controller::Ast1060I2c::slave_read) - Read data received from master
//! - [`slave_write()`](super::controller::Ast1060I2c::slave_write) - Write response data for master
//! - [`handle_slave_interrupt()`](super::controller::Ast1060I2c::handle_slave_interrupt) - Process slave events
//!
//! These methods are more ergonomic and hardware-aware than generic HAL traits,
//! exposing AST1060-specific features like packet mode and structured interrupt events.
//!
//! # Integration
//!
//! For Hubris RTOS integration, see the `drv-ast1060-i2c-refactor` crate which
//! wraps this driver to implement the `drv-i2c-types::I2cHardware` trait.
//!
//! # Example Usage
//!
//! ```no_run
//! use embedded_hal::i2c::I2c;
//! use aspeed_ddk::i2c_core::controller::Ast1060I2c;
//! use aspeed_ddk::i2c_core::types::{I2cConfig, I2cSpeed};
//!
//! // Initialize I2C controller 0 at 400 kHz
//! let config = I2cConfig::new(I2cSpeed::Fast);
//! let mut i2c = unsafe { Ast1060I2c::new(0) };
//! i2c.init_hardware(&config);
//!
//! // Use embedded-hal I2c trait methods
//! i2c.write(0x50, &[0x10, 0x20])?;
//! let mut buffer = [0u8; 4];
//! i2c.read(0x51, &mut buffer)?;
//! i2c.write_read(0x52, &[0x00], &mut buffer)?;
//! # Ok::<(), aspeed_ddk::i2c_core::error::I2cError>(())
//! ```

use super::controller::Ast1060I2c;
use super::error::I2cError;
use embedded_hal::i2c::{
    Error, ErrorKind, ErrorType, I2c, NoAcknowledgeSource, Operation, SevenBitAddress,
};

// ============================================================================
// embedded-hal Error Trait Implementation
// ============================================================================

/// Implements the embedded-hal Error trait for `I2cError`.
///
/// Maps AST1060-specific I2C errors to standard embedded-hal `ErrorKind` values.
/// This enables interoperability with any code using the embedded-hal I2C traits.
///
/// # Error Mappings
///
/// - `NoAcknowledge` → `ErrorKind::NoAcknowledge` - Slave did not acknowledge
/// - `ArbitrationLoss` → `ErrorKind::ArbitrationLoss` - Lost bus arbitration
/// - `Bus`, `BusRecoveryFailed` → `ErrorKind::Bus` - Bus errors
/// - `Overrun` → `ErrorKind::Overrun` - Data overrun/underrun
/// - All others → `ErrorKind::Other` - Hardware-specific errors
impl Error for I2cError {
    fn kind(&self) -> ErrorKind {
        match self {
            I2cError::NoAcknowledge => ErrorKind::NoAcknowledge(NoAcknowledgeSource::Unknown),
            I2cError::ArbitrationLoss => ErrorKind::ArbitrationLoss,
            I2cError::Bus | I2cError::BusRecoveryFailed => ErrorKind::Bus,
            I2cError::Overrun => ErrorKind::Overrun,
            I2cError::Timeout
            | I2cError::Busy
            | I2cError::Invalid
            | I2cError::Abnormal
            | I2cError::SlaveError
            | I2cError::InvalidAddress => ErrorKind::Other,
        }
    }
}

/// Associates the `I2cError` type with `Ast1060I2c` for embedded-hal trait implementations.
impl<Y: FnMut(u32)> ErrorType for Ast1060I2c<'_, Y> {
    type Error = I2cError;
}

// ============================================================================
// embedded-hal I2c Trait Implementation (Master Mode)
// ============================================================================

/// Implements the embedded-hal I2c trait for `Ast1060I2c`.
///
/// Provides standard I2C master mode operations compatible with the embedded-hal ecosystem.
/// This implementation uses the AST1060's buffer mode with a 32-byte FIFO for efficient
/// data transfer.
///
/// # Supported Operations
///
/// - **write**: Send data to a slave device
/// - **read**: Receive data from a slave device  
/// - **`write_read`**: Write then read without releasing the bus (combined transaction)
/// - **transaction**: Execute a sequence of read/write operations
///
/// # Buffer Mode Limitations
///
/// The AST1060 hardware uses a 32-byte FIFO buffer. For transfers larger than 32 bytes,
/// the driver automatically splits them into multiple hardware transactions while
/// maintaining the logical transaction semantics.
///
/// # Example
///
/// ```no_run
/// use embedded_hal::i2c::I2c;
/// # use aspeed_ddk::i2c_core::controller::Ast1060I2c;
/// # let mut i2c = unsafe { Ast1060I2c::new(0) };
///
/// // Write data to slave at address 0x50
/// i2c.write(0x50, &[0x00, 0x01, 0x02])?;
///
/// // Read data from slave at address 0x51
/// let mut buffer = [0u8; 4];
/// i2c.read(0x51, &mut buffer)?;
///
/// // Combined write-read transaction
/// i2c.write_read(0x52, &[0xA0], &mut buffer)?;
/// # Ok::<(), aspeed_ddk::i2c_core::error::I2cError>(())
/// ```
impl<Y: FnMut(u32)> I2c<SevenBitAddress> for Ast1060I2c<'_, Y> {
    /// Writes data to an I2C slave device.
    ///
    /// # Arguments
    ///
    /// * `address` - 7-bit I2C slave address
    /// * `bytes` - Data bytes to write
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The slave does not acknowledge (`NoAcknowledge`)
    /// - Bus arbitration is lost (`ArbitrationLoss`)
    /// - A bus error occurs (`Bus`)
    /// - The operation times out (`Timeout`)
    fn write(&mut self, address: SevenBitAddress, bytes: &[u8]) -> Result<(), Self::Error> {
        Ast1060I2c::write(self, address, bytes)
    }

    /// Reads data from an I2C slave device.
    ///
    /// # Arguments
    ///
    /// * `address` - 7-bit I2C slave address
    /// * `buffer` - Buffer to store received data
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The slave does not acknowledge (`NoAcknowledge`)
    /// - Bus arbitration is lost (`ArbitrationLoss`)
    /// - A bus error occurs (`Bus`)
    /// - The operation times out (`Timeout`)
    fn read(&mut self, address: SevenBitAddress, buffer: &mut [u8]) -> Result<(), Self::Error> {
        Ast1060I2c::read(self, address, buffer)
    }

    /// Performs a combined write-read transaction without releasing the bus.
    ///
    /// This is commonly used to write a register address followed by reading
    /// the register value in a single atomic transaction.
    ///
    /// # Arguments
    ///
    /// * `address` - 7-bit I2C slave address
    /// * `bytes` - Data bytes to write first
    /// * `buffer` - Buffer to store read data
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The slave does not acknowledge (`NoAcknowledge`)
    /// - Bus arbitration is lost (`ArbitrationLoss`)
    /// - A bus error occurs (`Bus`)
    /// - The operation times out (`Timeout`)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use embedded_hal::i2c::I2c;
    /// # use aspeed_ddk::i2c_core::controller::Ast1060I2c;
    /// # let mut i2c = unsafe { Ast1060I2c::new(0) };
    /// // Read from register 0x10 at slave address 0x50
    /// let mut data = [0u8; 4];
    /// i2c.write_read(0x50, &[0x10], &mut data)?;
    /// # Ok::<(), aspeed_ddk::i2c_core::error::I2cError>(())
    /// ```
    fn write_read(
        &mut self,
        address: SevenBitAddress,
        bytes: &[u8],
        buffer: &mut [u8],
    ) -> Result<(), Self::Error> {
        Ast1060I2c::write_read(self, address, bytes, buffer)
    }

    /// Executes a sequence of I2C operations as a single transaction.
    ///
    /// Each operation is executed in order. The bus is held between operations
    /// and only released after all operations complete or an error occurs.
    ///
    /// # Arguments
    ///
    /// * `address` - 7-bit I2C slave address for all operations
    /// * `operations` - Slice of read/write operations to execute
    ///
    /// # Errors
    ///
    /// Returns an error if any operation fails. Subsequent operations are not
    /// executed if an error occurs.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use embedded_hal::i2c::{I2c, Operation};
    /// # use aspeed_ddk::i2c_core::controller::Ast1060I2c;
    /// # let mut i2c = unsafe { Ast1060I2c::new(0) };
    /// let mut read_buf = [0u8; 4];
    /// let mut ops = [
    ///     Operation::Write(&[0x10]),        // Write register address
    ///     Operation::Read(&mut read_buf),   // Read register value
    /// ];
    /// i2c.transaction(0x50, &mut ops)?;
    /// # Ok::<(), aspeed_ddk::i2c_core::error::I2cError>(())
    /// ```
    fn transaction(
        &mut self,
        address: SevenBitAddress,
        operations: &mut [Operation<'_>],
    ) -> Result<(), Self::Error> {
        // Execute each operation in sequence
        for op in operations {
            match op {
                Operation::Read(buf) => self.read(address, buf)?,
                Operation::Write(data) => self.write(address, data)?,
            }
        }
        Ok(())
    }
}
