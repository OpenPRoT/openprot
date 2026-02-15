// Licensed under the Apache-2.0 license

//! I2C Address types and validation
//!
//! This module provides type-safe I2C address handling with validation
//! for 7-bit addresses (the most common format).

use core::fmt;

/// Error returned when an invalid I2C address is provided.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddressError {
    /// Address is outside the valid 7-bit range (0x00-0x7F).
    OutOfRange(u8),
    /// Address is in the reserved range (0x00-0x07 or 0x78-0x7F).
    Reserved(u8),
}

impl fmt::Display for AddressError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AddressError::OutOfRange(addr) => {
                write!(f, "I2C address 0x{addr:02X} is out of 7-bit range")
            }
            AddressError::Reserved(addr) => {
                write!(f, "I2C address 0x{addr:02X} is reserved")
            }
        }
    }
}

/// A validated 7-bit I2C address.
///
/// I2C addresses are 7 bits wide (0x00-0x7F), but certain ranges are reserved:
/// - 0x00-0x07: Reserved for special purposes (general call, etc.)
/// - 0x78-0x7F: Reserved for 10-bit addressing extension
///
/// This type validates addresses upon construction and provides type safety
/// for I2C operations.
///
/// # Examples
///
/// ```rust,ignore
/// use i2c_api::I2cAddress;
///
/// // Create a valid address
/// let addr = I2cAddress::new(0x48).expect("valid address");
/// assert_eq!(addr.value(), 0x48);
///
/// // Reserved addresses are rejected by default
/// assert!(I2cAddress::new(0x00).is_err());
///
/// // Use new_unchecked for reserved addresses if needed
/// let general_call = I2cAddress::new_unchecked(0x00);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct I2cAddress(u8);

impl I2cAddress {
    /// The general call address (0x00).
    pub const GENERAL_CALL: I2cAddress = I2cAddress(0x00);

    /// Creates a new I2C address, validating that it's in the valid range
    /// and not in the reserved ranges.
    ///
    /// # Errors
    ///
    /// Returns `AddressError::OutOfRange` if the address is greater than 0x7F.
    /// Returns `AddressError::Reserved` if the address is in 0x00-0x07 or 0x78-0x7F.
    pub const fn new(addr: u8) -> Result<Self, AddressError> {
        if addr > 0x7F {
            return Err(AddressError::OutOfRange(addr));
        }
        if addr <= 0x07 || addr >= 0x78 {
            return Err(AddressError::Reserved(addr));
        }
        Ok(I2cAddress(addr))
    }

    /// Creates a new I2C address without validation.
    ///
    /// This is useful for reserved addresses like the general call address,
    /// which are valid but not typically used for device communication.
    ///
    /// # Safety
    ///
    /// The caller must ensure the address is valid for the intended use.
    /// Using an address outside 0x00-0x7F may cause undefined behavior
    /// on the I2C bus.
    pub const fn new_unchecked(addr: u8) -> Self {
        I2cAddress(addr & 0x7F)
    }

    /// Returns the raw 7-bit address value.
    #[inline]
    pub const fn value(self) -> u8 {
        self.0
    }

    /// Returns the address formatted for a write operation (R/W bit = 0).
    ///
    /// The returned value is the address shifted left by 1 with bit 0 = 0.
    #[inline]
    pub const fn write_address(self) -> u8 {
        self.0 << 1
    }

    /// Returns the address formatted for a read operation (R/W bit = 1).
    ///
    /// The returned value is the address shifted left by 1 with bit 0 = 1.
    #[inline]
    pub const fn read_address(self) -> u8 {
        (self.0 << 1) | 1
    }
}

impl TryFrom<u8> for I2cAddress {
    type Error = AddressError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        I2cAddress::new(value)
    }
}

impl From<I2cAddress> for u8 {
    fn from(addr: I2cAddress) -> u8 {
        addr.0
    }
}

impl fmt::Display for I2cAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{:02X}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_addresses() {
        // Typical device address range
        assert!(I2cAddress::new(0x08).is_ok());
        assert!(I2cAddress::new(0x48).is_ok());
        assert!(I2cAddress::new(0x77).is_ok());
    }

    #[test]
    fn test_reserved_low_addresses() {
        for addr in 0x00..=0x07 {
            assert_eq!(I2cAddress::new(addr), Err(AddressError::Reserved(addr)));
        }
    }

    #[test]
    fn test_reserved_high_addresses() {
        for addr in 0x78..=0x7F {
            assert_eq!(I2cAddress::new(addr), Err(AddressError::Reserved(addr)));
        }
    }

    #[test]
    fn test_out_of_range() {
        assert_eq!(I2cAddress::new(0x80), Err(AddressError::OutOfRange(0x80)));
        assert_eq!(I2cAddress::new(0xFF), Err(AddressError::OutOfRange(0xFF)));
    }

    #[test]
    fn test_unchecked() {
        let addr = I2cAddress::new_unchecked(0x00);
        assert_eq!(addr.value(), 0x00);

        // Out of range gets masked
        let addr = I2cAddress::new_unchecked(0xFF);
        assert_eq!(addr.value(), 0x7F);
    }

    #[test]
    fn test_wire_format() {
        let addr = I2cAddress::new(0x48).unwrap();
        assert_eq!(addr.write_address(), 0x90); // 0x48 << 1
        assert_eq!(addr.read_address(), 0x91);  // (0x48 << 1) | 1
    }
}
