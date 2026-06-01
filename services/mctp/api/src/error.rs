// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! MCTP error types
//!
//! This module defines error types for MCTP operations, providing both
//! transport-level errors and higher-level service response codes.

use core::fmt;

/// Response codes from the MCTP service.
///
/// These codes indicate the result of an MCTP operation and are designed
/// to be compatible with IPC/RPC response handling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ResponseCode {
    /// Operation completed successfully.
    Success = 0,
    /// Internal server error.
    InternalError = 1,
    /// No space available (e.g., message buffers full).
    NoSpace = 2,
    /// Address/handle already in use.
    AddrInUse = 3,
    /// Operation timed out.
    TimedOut = 4,
    /// Invalid argument provided.
    BadArgument = 5,
    /// Server restarted, state lost.
    ServerRestarted = 6,
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
            1 => Some(ResponseCode::InternalError),
            2 => Some(ResponseCode::NoSpace),
            3 => Some(ResponseCode::AddrInUse),
            4 => Some(ResponseCode::TimedOut),
            5 => Some(ResponseCode::BadArgument),
            6 => Some(ResponseCode::ServerRestarted),
            _ => None,
        }
    }
}

impl fmt::Display for ResponseCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResponseCode::Success => write!(f, "success"),
            ResponseCode::InternalError => write!(f, "internal error"),
            ResponseCode::NoSpace => write!(f, "no space"),
            ResponseCode::AddrInUse => write!(f, "address in use"),
            ResponseCode::TimedOut => write!(f, "timed out"),
            ResponseCode::BadArgument => write!(f, "bad argument"),
            ResponseCode::ServerRestarted => write!(f, "server restarted"),
        }
    }
}

/// MCTP operation error.
///
/// This is the main error type returned by MCTP client operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MctpError {
    /// The response code from the service.
    pub code: ResponseCode,
}

impl MctpError {
    /// Creates a new error from a response code.
    pub const fn from_code(code: ResponseCode) -> Self {
        MctpError { code }
    }

    /// Returns `true` if this is a timeout error.
    #[inline]
    pub const fn is_timeout(&self) -> bool {
        matches!(self.code, ResponseCode::TimedOut)
    }
}

impl fmt::Display for MctpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MCTP error: {}", self.code)
    }
}

impl From<ResponseCode> for MctpError {
    fn from(code: ResponseCode) -> Self {
        MctpError::from_code(code)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    extern crate std;
    use std::string::ToString;

    #[test]
    fn response_code_is_success() {
        assert!(ResponseCode::Success.is_success());
        assert!(!ResponseCode::Success.is_error());
        for &code in &[
            ResponseCode::InternalError,
            ResponseCode::NoSpace,
            ResponseCode::AddrInUse,
            ResponseCode::TimedOut,
            ResponseCode::BadArgument,
            ResponseCode::ServerRestarted,
        ] {
            assert!(!code.is_success(), "{code:?} should not be success");
            assert!(code.is_error(), "{code:?} should be an error");
        }
    }

    #[test]
    fn response_code_from_u8_roundtrip() {
        for val in 0u8..=6 {
            let code = ResponseCode::from_u8(val).expect("known code");
            assert_eq!(code as u8, val);
        }
        assert_eq!(ResponseCode::from_u8(7), None);
        assert_eq!(ResponseCode::from_u8(255), None);
    }

    #[test]
    fn response_code_display() {
        assert_eq!(ResponseCode::Success.to_string(), "success");
        assert_eq!(ResponseCode::InternalError.to_string(), "internal error");
        assert_eq!(ResponseCode::NoSpace.to_string(), "no space");
        assert_eq!(ResponseCode::AddrInUse.to_string(), "address in use");
        assert_eq!(ResponseCode::TimedOut.to_string(), "timed out");
        assert_eq!(ResponseCode::BadArgument.to_string(), "bad argument");
        assert_eq!(
            ResponseCode::ServerRestarted.to_string(),
            "server restarted"
        );
    }

    #[test]
    fn mctp_error_is_timeout() {
        let e = MctpError::from_code(ResponseCode::TimedOut);
        assert!(e.is_timeout());
        let e2 = MctpError::from_code(ResponseCode::NoSpace);
        assert!(!e2.is_timeout());
    }

    #[test]
    fn mctp_error_display() {
        let e = MctpError::from_code(ResponseCode::BadArgument);
        assert_eq!(e.to_string(), "MCTP error: bad argument");
    }

    #[test]
    fn mctp_error_from_response_code() {
        let e: MctpError = ResponseCode::ServerRestarted.into();
        assert_eq!(e.code, ResponseCode::ServerRestarted);
    }
}
