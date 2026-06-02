// Licensed under the Apache-2.0 license

//! I3C error types
//!
//! Consolidated error types for the I3C subsystem.

use core::fmt;

/// Primary error type for I3C operations
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum I3cError {
    /// No DAT (Device Address Table) position available
    NoDatPos,
    /// No messages provided for transfer
    NoMsgs,
    /// Too many messages for single transfer
    TooManyMsgs,
    /// Invalid arguments provided
    InvalidArgs,
    /// Operation timed out
    Timeout,
    /// Device not found
    NoSuchDev,
    /// Access denied or not permitted
    Access,
    /// Generic I/O error
    IoError,
    /// Invalid operation or state
    Invalid,
    /// Address already in use
    AddrInUse,
    /// Address space exhausted
    AddrExhausted,
    /// No free slot available
    NoFreeSlot,
    /// Device not found in attached list
    DevNotFound,
    /// Device already attached
    DevAlreadyAttached,
    /// Invalid parameter
    InvalidParam,
    /// CCC (Common Command Code) error
    CccError(CccErrorKind),
    /// Other unspecified error
    Other,
}

impl fmt::Display for I3cError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoDatPos => write!(f, "no DAT position available"),
            Self::NoMsgs => write!(f, "no messages provided"),
            Self::TooManyMsgs => write!(f, "too many messages"),
            Self::InvalidArgs => write!(f, "invalid arguments"),
            Self::Timeout => write!(f, "operation timed out"),
            Self::NoSuchDev | Self::DevNotFound => write!(f, "device not found"),
            Self::Access => write!(f, "access denied"),
            Self::IoError => write!(f, "I/O error"),
            Self::Invalid => write!(f, "invalid operation"),
            Self::AddrInUse => write!(f, "address in use"),
            Self::AddrExhausted => write!(f, "address space exhausted"),
            Self::NoFreeSlot => write!(f, "no free slot"),
            Self::DevAlreadyAttached => write!(f, "device already attached"),
            Self::InvalidParam => write!(f, "invalid parameter"),
            Self::CccError(kind) => write!(f, "CCC error: {kind:?}"),
            Self::Other => write!(f, "other error"),
        }
    }
}

/// CCC-specific error kinds
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum CccErrorKind {
    /// Invalid parameter for CCC
    InvalidParam,
    /// Target not found
    NotFound,
    /// No free slot for CCC operation
    NoFreeSlot,
    /// Invalid CCC response or operation
    Invalid,
}

/// Convenience Result type for I3C operations
pub type Result<T> = core::result::Result<T, I3cError>;

// Conversion from legacy error types for backwards compatibility

impl From<CccErrorKind> for I3cError {
    #[inline]
    fn from(kind: CccErrorKind) -> Self {
        Self::CccError(kind)
    }
}

/// Implement embedded-hal I2C error trait for interoperability
impl embedded_hal::i2c::Error for I3cError {
    fn kind(&self) -> embedded_hal::i2c::ErrorKind {
        match self {
            Self::Timeout => embedded_hal::i2c::ErrorKind::NoAcknowledge(
                embedded_hal::i2c::NoAcknowledgeSource::Unknown,
            ),
            Self::NoSuchDev | Self::DevNotFound => embedded_hal::i2c::ErrorKind::NoAcknowledge(
                embedded_hal::i2c::NoAcknowledgeSource::Address,
            ),
            Self::IoError | Self::Access => embedded_hal::i2c::ErrorKind::Bus,
            _ => embedded_hal::i2c::ErrorKind::Other,
        }
    }
}

/// Implement proposed I3C master error trait
impl proposed_traits::i3c_master::Error for I3cError {
    #[inline]
    fn kind(&self) -> proposed_traits::i3c_master::ErrorKind {
        match self {
            Self::AddrInUse => proposed_traits::i3c_master::ErrorKind::DynamicAddressConflict,
            Self::CccError(_) | Self::Invalid => proposed_traits::i3c_master::ErrorKind::InvalidCcc,
            _ => proposed_traits::i3c_master::ErrorKind::Other,
        }
    }
}
