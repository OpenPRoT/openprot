// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Error types for the crypto IPC wire protocol.

use core::fmt;

/// Wire-level decode/encode error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WireError {
    /// Output buffer too small for the encoded message.
    BufferTooSmall,
    /// Input buffer too short for a complete header or payload.
    Truncated,
    /// Unrecognized operation code.
    InvalidOpcode(u8),
    /// Unrecognized enum discriminant (status tag).
    InvalidValue(u8),
}

impl fmt::Display for WireError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BufferTooSmall => f.write_str("buffer too small"),
            Self::Truncated => f.write_str("truncated"),
            Self::InvalidOpcode(op) => write!(f, "invalid opcode 0x{op:02x}"),
            Self::InvalidValue(v) => write!(f, "invalid value 0x{v:02x}"),
        }
    }
}

impl core::error::Error for WireError {}

/// On-wire response code from the crypto service.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ResponseCode {
    Success = 0,
    InternalError = 1,
    InvalidOp = 2,
    /// Operation not valid in the service's current state.
    WrongState = 3,
    MalformedRequest = 4,
}

impl ResponseCode {
    pub const fn is_success(self) -> bool {
        matches!(self, Self::Success)
    }

    pub const fn from_u8(val: u8) -> Option<Self> {
        match val {
            0 => Some(Self::Success),
            1 => Some(Self::InternalError),
            2 => Some(Self::InvalidOp),
            3 => Some(Self::WrongState),
            4 => Some(Self::MalformedRequest),
            _ => None,
        }
    }
}

impl fmt::Display for ResponseCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Success => f.write_str("success"),
            Self::InternalError => f.write_str("internal error"),
            Self::InvalidOp => f.write_str("invalid op"),
            Self::WrongState => f.write_str("wrong state"),
            Self::MalformedRequest => f.write_str("malformed request"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn response_code_roundtrip() {
        for val in 0u8..=4 {
            let code = ResponseCode::from_u8(val).expect("known code");
            assert_eq!(code as u8, val);
        }
        assert_eq!(ResponseCode::from_u8(5), None);
        assert_eq!(ResponseCode::from_u8(255), None);
    }
}
