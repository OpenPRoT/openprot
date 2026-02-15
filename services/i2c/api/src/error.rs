// Licensed under the Apache-2.0 license

//! I2C Error types
//!
//! This module defines error types for I2C operations, providing both
//! low-level hardware errors and higher-level service response codes.

use core::fmt;

/// Response codes from the I2C service.
///
/// These codes indicate the result of an I2C operation and are designed
/// to be compatible with IPC/RPC response handling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ResponseCode {
    /// Operation completed successfully.
    Success = 0,
    /// No device acknowledged at the given address (NACK on address byte).
    NoDevice = 1,
    /// Device NACKed during data transfer.
    NackData = 2,
    /// Bus arbitration was lost to another master.
    ArbitrationLost = 3,
    /// Bus is stuck (SDA or SCL held low).
    BusStuck = 4,
    /// Operation timed out.
    Timeout = 5,
    /// Invalid bus index specified.
    InvalidBus = 6,
    /// Invalid address specified.
    InvalidAddress = 7,
    /// Buffer too small for the requested operation.
    BufferTooSmall = 8,
    /// Buffer too large for the hardware to handle.
    BufferTooLarge = 9,
    /// The I2C controller is not initialized.
    NotInitialized = 10,
    /// The I2C controller is busy with another operation.
    Busy = 11,
    /// Permission denied - task not authorized for this bus.
    Unauthorized = 12,
    /// General I/O error.
    IoError = 13,
    /// Internal server error.
    ServerError = 14,
}

impl ResponseCode {
    /// Returns `true` if this represents a successful operation.
    #[inline]
    pub const fn is_success(self) -> bool {
        matches!(self, ResponseCode::Success)
    }

    /// Returns `true` if this represents an error condition.
    #[inline]
    pub const fn is_error(self) -> bool {
        !self.is_success()
    }

    /// Converts from a raw u8 value.
    pub const fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(ResponseCode::Success),
            1 => Some(ResponseCode::NoDevice),
            2 => Some(ResponseCode::NackData),
            3 => Some(ResponseCode::ArbitrationLost),
            4 => Some(ResponseCode::BusStuck),
            5 => Some(ResponseCode::Timeout),
            6 => Some(ResponseCode::InvalidBus),
            7 => Some(ResponseCode::InvalidAddress),
            8 => Some(ResponseCode::BufferTooSmall),
            9 => Some(ResponseCode::BufferTooLarge),
            10 => Some(ResponseCode::NotInitialized),
            11 => Some(ResponseCode::Busy),
            12 => Some(ResponseCode::Unauthorized),
            13 => Some(ResponseCode::IoError),
            14 => Some(ResponseCode::ServerError),
            _ => None,
        }
    }
}

impl fmt::Display for ResponseCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResponseCode::Success => write!(f, "success"),
            ResponseCode::NoDevice => write!(f, "no device at address"),
            ResponseCode::NackData => write!(f, "device NACKed data"),
            ResponseCode::ArbitrationLost => write!(f, "bus arbitration lost"),
            ResponseCode::BusStuck => write!(f, "bus stuck"),
            ResponseCode::Timeout => write!(f, "operation timed out"),
            ResponseCode::InvalidBus => write!(f, "invalid bus index"),
            ResponseCode::InvalidAddress => write!(f, "invalid address"),
            ResponseCode::BufferTooSmall => write!(f, "buffer too small"),
            ResponseCode::BufferTooLarge => write!(f, "buffer too large"),
            ResponseCode::NotInitialized => write!(f, "controller not initialized"),
            ResponseCode::Busy => write!(f, "controller busy"),
            ResponseCode::Unauthorized => write!(f, "unauthorized"),
            ResponseCode::IoError => write!(f, "I/O error"),
            ResponseCode::ServerError => write!(f, "server error"),
        }
    }
}

/// Specific I2C error conditions at the hardware level.
///
/// These errors are compatible with `embedded_hal::i2c::ErrorKind`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum I2cErrorKind {
    /// I2C bus error (misplaced start/stop conditions or external interference).
    Bus,
    /// Lost arbitration to another master on the bus.
    ArbitrationLoss,
    /// No acknowledgment received from the target device.
    NoAcknowledge(NoAcknowledgeSource),
    /// Data overrun/underrun error.
    Overrun,
    /// An unspecified error occurred.
    Other,
}

/// Source of a NoAcknowledge error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoAcknowledgeSource {
    /// NACK received during address phase.
    Address,
    /// NACK received during data phase.
    Data,
    /// Unknown source of NACK.
    Unknown,
}

impl fmt::Display for I2cErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            I2cErrorKind::Bus => write!(f, "bus error"),
            I2cErrorKind::ArbitrationLoss => write!(f, "arbitration loss"),
            I2cErrorKind::NoAcknowledge(src) => match src {
                NoAcknowledgeSource::Address => write!(f, "NACK on address"),
                NoAcknowledgeSource::Data => write!(f, "NACK on data"),
                NoAcknowledgeSource::Unknown => write!(f, "NACK (unknown source)"),
            },
            I2cErrorKind::Overrun => write!(f, "overrun"),
            I2cErrorKind::Other => write!(f, "other error"),
        }
    }
}

/// I2C operation error.
///
/// This is the main error type returned by I2C client operations,
/// combining response codes with additional context.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct I2cError {
    /// The response code from the service.
    pub code: ResponseCode,
    /// The specific error kind, if known.
    pub kind: Option<I2cErrorKind>,
}

impl I2cError {
    /// Creates a new error from a response code.
    pub const fn from_code(code: ResponseCode) -> Self {
        let kind = match code {
            ResponseCode::NoDevice => Some(I2cErrorKind::NoAcknowledge(NoAcknowledgeSource::Address)),
            ResponseCode::NackData => Some(I2cErrorKind::NoAcknowledge(NoAcknowledgeSource::Data)),
            ResponseCode::ArbitrationLost => Some(I2cErrorKind::ArbitrationLoss),
            ResponseCode::BusStuck => Some(I2cErrorKind::Bus),
            _ => None,
        };
        I2cError { code, kind }
    }

    /// Creates a new error with explicit kind.
    pub const fn new(code: ResponseCode, kind: I2cErrorKind) -> Self {
        I2cError { code, kind: Some(kind) }
    }

    /// Returns `true` if this is a no-device error.
    #[inline]
    pub const fn is_no_device(&self) -> bool {
        matches!(self.code, ResponseCode::NoDevice)
    }

    /// Returns `true` if this is a timeout error.
    #[inline]
    pub const fn is_timeout(&self) -> bool {
        matches!(self.code, ResponseCode::Timeout)
    }
}

impl fmt::Display for I2cError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "I2C error: {}", self.code)?;
        if let Some(kind) = &self.kind {
            write!(f, " ({kind})")?;
        }
        Ok(())
    }
}

impl From<ResponseCode> for I2cError {
    fn from(code: ResponseCode) -> Self {
        I2cError::from_code(code)
    }
}

// Implement embedded_hal::i2c::Error trait for interoperability
impl embedded_hal::i2c::Error for I2cError {
    fn kind(&self) -> embedded_hal::i2c::ErrorKind {
        match self.kind {
            Some(I2cErrorKind::Bus) => embedded_hal::i2c::ErrorKind::Bus,
            Some(I2cErrorKind::ArbitrationLoss) => embedded_hal::i2c::ErrorKind::ArbitrationLoss,
            Some(I2cErrorKind::NoAcknowledge(src)) => {
                let nack_src = match src {
                    NoAcknowledgeSource::Address => {
                        embedded_hal::i2c::NoAcknowledgeSource::Address
                    }
                    NoAcknowledgeSource::Data => {
                        embedded_hal::i2c::NoAcknowledgeSource::Data
                    }
                    NoAcknowledgeSource::Unknown => {
                        embedded_hal::i2c::NoAcknowledgeSource::Unknown
                    }
                };
                embedded_hal::i2c::ErrorKind::NoAcknowledge(nack_src)
            }
            Some(I2cErrorKind::Overrun) => embedded_hal::i2c::ErrorKind::Overrun,
            Some(I2cErrorKind::Other) | None => embedded_hal::i2c::ErrorKind::Other,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_response_code_roundtrip() {
        for i in 0..=14 {
            let code = ResponseCode::from_u8(i).expect("valid code");
            assert_eq!(code as u8, i);
        }
    }

    #[test]
    fn test_error_from_code() {
        let err = I2cError::from_code(ResponseCode::NoDevice);
        assert!(err.is_no_device());
        assert_eq!(
            err.kind,
            Some(I2cErrorKind::NoAcknowledge(NoAcknowledgeSource::Address))
        );
    }

    #[test]
    fn test_error_display() {
        let err = I2cError::from_code(ResponseCode::Timeout);
        assert!(err.is_timeout());
        assert!(!err.is_no_device());
    }
}
