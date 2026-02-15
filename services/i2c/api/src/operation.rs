// Licensed under the Apache-2.0 license

//! I2C transaction operation types.
//!
//! This module provides the `Operation` enum for building
//! multi-operation I2C transactions.

use core::fmt;

/// An I2C operation within a transaction.
///
/// Used with [`I2cClient::transaction`] to execute multiple operations
/// without releasing the bus between them.
///
/// # Examples
///
/// ```rust,ignore
/// use i2c_api::{I2cClient, Operation, BusIndex, I2cAddress};
///
/// // Write register address, then read 4 bytes
/// let mut read_buf = [0u8; 4];
/// client.transaction(bus, address, &mut [
///     Operation::Write(&[0x00]),     // Register address
///     Operation::Read(&mut read_buf), // Read response
/// ])?;
/// ```
#[derive(Debug)]
pub enum Operation<'a> {
    /// Write data to the device.
    ///
    /// The slice contains bytes to send. An empty slice can be used
    /// for address-only probing.
    Write(&'a [u8]),

    /// Read data from the device.
    ///
    /// The slice is filled with received bytes. The length determines
    /// how many bytes to read.
    Read(&'a mut [u8]),
}

impl<'a> Operation<'a> {
    /// Returns `true` if this is a write operation.
    #[inline]
    pub const fn is_write(&self) -> bool {
        matches!(self, Operation::Write(_))
    }

    /// Returns `true` if this is a read operation.
    #[inline]
    pub const fn is_read(&self) -> bool {
        matches!(self, Operation::Read(_))
    }

    /// Returns the length of data involved in this operation.
    pub fn len(&self) -> usize {
        match self {
            Operation::Write(data) => data.len(),
            Operation::Read(data) => data.len(),
        }
    }

    /// Returns `true` if this operation has no data.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl fmt::Display for Operation<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Operation::Write(data) => write!(f, "Write({} bytes)", data.len()),
            Operation::Read(data) => write!(f, "Read({} bytes)", data.len()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_operation_is_write() {
        let op = Operation::Write(&[0x00, 0x01]);
        assert!(op.is_write());
        assert!(!op.is_read());
    }

    #[test]
    fn test_operation_is_read() {
        let mut buf = [0u8; 4];
        let op = Operation::Read(&mut buf);
        assert!(op.is_read());
        assert!(!op.is_write());
    }

    #[test]
    fn test_operation_len() {
        let write_op = Operation::Write(&[0x00, 0x01, 0x02]);
        assert_eq!(write_op.len(), 3);

        let mut buf = [0u8; 8];
        let read_op = Operation::Read(&mut buf);
        assert_eq!(read_op.len(), 8);
    }

    #[test]
    fn test_operation_empty() {
        let empty_write = Operation::Write(&[]);
        assert!(empty_write.is_empty());

        let mut empty_buf: [u8; 0] = [];
        let empty_read = Operation::Read(&mut empty_buf);
        assert!(empty_read.is_empty());
    }
}
