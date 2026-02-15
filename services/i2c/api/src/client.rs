// Licensed under the Apache-2.0 license

//! I2C client traits and bus index type.
//!
//! This module defines the core I2C client traits using the
//! `embedded_hal::i2c::ErrorType` pattern for maximum compatibility.

use core::fmt;

use embedded_hal::i2c::{Error, ErrorKind, ErrorType};

use crate::address::I2cAddress;
use crate::operation::Operation;

/// I2C bus identifier.
///
/// Each bus represents a physical I2C controller or a logical bus
/// behind a multiplexer. Bus indices are platform-specific.
///
/// # Examples
///
/// ```rust,ignore
/// use i2c_api::BusIndex;
///
/// let bus = BusIndex::new(0);
/// // Or use predefined constants
/// let bus = BusIndex::BUS_0;
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BusIndex(u8);

impl BusIndex {
    /// Bus 0 - typically the primary I2C bus.
    pub const BUS_0: BusIndex = BusIndex(0);
    /// Bus 1 - secondary I2C bus.
    pub const BUS_1: BusIndex = BusIndex(1);
    /// Bus 2 - tertiary I2C bus.
    pub const BUS_2: BusIndex = BusIndex(2);

    /// Creates a new bus index.
    #[inline]
    pub const fn new(index: u8) -> Self {
        BusIndex(index)
    }

    /// Returns the raw index value.
    #[inline]
    pub const fn value(self) -> u8 {
        self.0
    }
}

impl From<u8> for BusIndex {
    fn from(index: u8) -> Self {
        BusIndex(index)
    }
}

impl From<BusIndex> for u8 {
    fn from(bus: BusIndex) -> u8 {
        bus.0
    }
}

impl fmt::Display for BusIndex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "I2C{}", self.0)
    }
}

/// Core I2C client operations.
///
/// This trait defines the fundamental I2C operations without
/// transport-specific details. Implementations handle IPC,
/// direct hardware access, or mocking.
///
/// Uses the `ErrorType` supertrait pattern from embedded-hal
/// for maximum compatibility with the embedded ecosystem.
///
/// # Examples
///
/// ```rust,ignore
/// use i2c_api::{I2cClient, I2cAddress, BusIndex};
///
/// fn read_sensor<C: I2cClient>(client: &mut C) -> Result<[u8; 2], C::Error> {
///     let address = I2cAddress::new(0x48)?;
///     let bus = BusIndex::BUS_0;
///     
///     let mut buffer = [0u8; 2];
///     client.write_read(bus, address, &[0x00], &mut buffer)?;
///     Ok(buffer)
/// }
/// ```
pub trait I2cClient: ErrorType {
    /// Write data to a device, then read response.
    ///
    /// This is the fundamental I2C operation:
    /// - If `write` is non-empty and `read` is empty: write-only
    /// - If `write` is empty and `read` is non-empty: read-only
    /// - If both non-empty: write-then-read (repeated start)
    ///
    /// # Arguments
    ///
    /// * `bus` - I2C bus to use
    /// * `address` - Device address
    /// * `write` - Data to write (can be empty)
    /// * `read` - Buffer for read data (can be empty)
    ///
    /// # Returns
    ///
    /// Number of bytes read on success.
    fn write_read(
        &mut self,
        bus: BusIndex,
        address: I2cAddress,
        write: &[u8],
        read: &mut [u8],
    ) -> Result<usize, Self::Error>;

    /// Execute multiple operations as a single transaction.
    ///
    /// All operations execute atomically without releasing the bus.
    /// This uses repeated start conditions between operations.
    ///
    /// # Arguments
    ///
    /// * `bus` - I2C bus to use
    /// * `address` - Device address
    /// * `operations` - Sequence of operations to perform
    fn transaction(
        &mut self,
        bus: BusIndex,
        address: I2cAddress,
        operations: &mut [Operation<'_>],
    ) -> Result<(), Self::Error>;
}

/// Blocking I2C client with convenience methods.
///
/// Extends [`I2cClient`] with higher-level operations commonly
/// used with register-based I2C devices. This trait is automatically
/// implemented for all types implementing `I2cClient`.
///
/// # Examples
///
/// ```rust,ignore
/// use i2c_api::{I2cClientBlocking, I2cAddress, BusIndex};
///
/// fn configure_device<C: I2cClientBlocking>(
///     client: &mut C,
///     bus: BusIndex,
///     address: I2cAddress,
/// ) -> Result<(), C::Error> {
///     // Write to config register (0x01) with value 0x80
///     client.write_register(bus, address, [0x01], [0x80])?;
///     
///     // Read temperature register (0x00)
///     let temp: [u8; 2] = client.read_register(bus, address, [0x00])?;
///     Ok(())
/// }
/// ```
pub trait I2cClientBlocking: I2cClient {
    /// Write data to a device.
    fn write(
        &mut self,
        bus: BusIndex,
        address: I2cAddress,
        data: &[u8],
    ) -> Result<(), Self::Error> {
        self.write_read(bus, address, data, &mut [])?;
        Ok(())
    }

    /// Read data from a device.
    ///
    /// Returns the number of bytes read.
    fn read(
        &mut self,
        bus: BusIndex,
        address: I2cAddress,
        buffer: &mut [u8],
    ) -> Result<usize, Self::Error> {
        self.write_read(bus, address, &[], buffer)
    }

    /// Read a register value.
    ///
    /// Writes the register address, then reads the value in a single
    /// transaction using repeated start.
    fn read_register<R: AsRef<[u8]>, V: AsMut<[u8]> + Default>(
        &mut self,
        bus: BusIndex,
        address: I2cAddress,
        register: R,
    ) -> Result<V, Self::Error> {
        let mut value = V::default();
        self.write_read(bus, address, register.as_ref(), value.as_mut())?;
        Ok(value)
    }

    /// Write a register value.
    ///
    /// Writes register address followed by value in a single transaction.
    /// Uses transaction() to ensure both writes occur atomically.
    fn write_register<R: AsRef<[u8]>, V: AsRef<[u8]>>(
        &mut self,
        bus: BusIndex,
        address: I2cAddress,
        register: R,
        value: V,
    ) -> Result<(), Self::Error> {
        // Use transaction to combine register and value into atomic write
        self.transaction(bus, address, &mut [
            Operation::Write(register.as_ref()),
            Operation::Write(value.as_ref()),
        ])
    }

    /// Check if a device is present at the given address.
    ///
    /// Performs a zero-length write to probe for ACK.
    /// Returns `true` if a device acknowledged, `false` if NACK.
    fn probe(
        &mut self,
        bus: BusIndex,
        address: I2cAddress,
    ) -> Result<bool, Self::Error> {
        match self.write(bus, address, &[]) {
            Ok(()) => Ok(true),
            Err(e) => {
                // Use embedded-hal Error trait to check error kind
                if matches!(e.kind(), ErrorKind::NoAcknowledge(_)) {
                    Ok(false)
                } else {
                    Err(e)
                }
            }
        }
    }
}

// Blanket implementation for all I2cClient implementors
impl<T: I2cClient> I2cClientBlocking for T {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bus_index_new() {
        let bus = BusIndex::new(5);
        assert_eq!(bus.value(), 5);
    }

    #[test]
    fn test_bus_index_constants() {
        assert_eq!(BusIndex::BUS_0.value(), 0);
        assert_eq!(BusIndex::BUS_1.value(), 1);
        assert_eq!(BusIndex::BUS_2.value(), 2);
    }

    #[test]
    fn test_bus_index_from() {
        let bus: BusIndex = 3.into();
        assert_eq!(bus.value(), 3);

        let raw: u8 = BusIndex::new(4).into();
        assert_eq!(raw, 4);
    }
}
