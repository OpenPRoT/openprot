// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! I3C error types
//!
//! Consolidated error types for the I3C subsystem.
//!
//! Ported from `aspeed-rust/src/i3c/error.rs` @ ce3b567. The
//! `proposed_traits::i3c_master::Error` impl is dropped (Delta D1): that trait
//! is unavailable in openprot, and the master operations are exposed as
//! inherent methods that return `I3cError` directly.

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
    /// Transfer completed but response status indicated failure; unlike
    /// `Timeout`, it may already have taken effect at the target.
    RespError,
    /// ENTDAA address-phase NACK: no unassigned device answered (expected
    /// end of a DAA walk, not a fault like other `RespError` cases).
    DaaNack,
    /// Target-address phase NACK on a direct transfer, e.g. unsupported
    /// optional CCC — kept distinct from other response failures.
    AddressNack,
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
    /// Resource (bus IRQ slot) already claimed by another controller
    Busy,
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
            Self::RespError => write!(f, "transfer response indicated failure"),
            Self::DaaNack => write!(f, "ENTDAA: no unassigned device answered"),
            Self::AddressNack => write!(f, "target address was not acknowledged"),
            Self::AddrInUse => write!(f, "address in use"),
            Self::AddrExhausted => write!(f, "address space exhausted"),
            Self::NoFreeSlot => write!(f, "no free slot"),
            Self::DevAlreadyAttached => write!(f, "device already attached"),
            Self::InvalidParam => write!(f, "invalid parameter"),
            Self::CccError(kind) => write!(f, "CCC error: {kind:?}"),
            Self::Busy => write!(f, "resource busy"),
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
