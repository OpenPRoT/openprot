// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Error types for the PLDM IPC wire protocol.

use core::fmt;

/// Wire-level decode/encode error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WireError {
    /// Output buffer too small for the encoded message.
    BufferTooSmall,
    /// Payload exceeds the maximum allowed size.
    PayloadTooLarge,
    /// Unrecognized operation code.
    InvalidOpcode(u8),
    /// Input buffer too short for a complete header or payload.
    Truncated,
    /// Unrecognized enum value (status discriminant, deny reason, etc).
    InvalidValue(u8),
}

impl fmt::Display for WireError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BufferTooSmall => f.write_str("buffer too small"),
            Self::PayloadTooLarge => f.write_str("payload too large"),
            Self::InvalidOpcode(op) => write!(f, "invalid opcode 0x{op:02x}"),
            Self::Truncated => f.write_str("truncated"),
            Self::InvalidValue(v) => write!(f, "invalid value 0x{v:02x}"),
        }
    }
}

/// On-wire response code from the FD.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ResponseCode {
    /// Operation completed successfully.
    Success = 0,
    /// Internal server error.
    InternalError = 1,
    /// Unrecognized opcode (InvalidOp on the wire).
    InvalidOp = 2,
    /// Operation not valid in the FD's current phase.
    WrongPhase = 3,
    /// Request frame did not decode: short, over-long, or a field the
    /// FD does not recognize.
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
            3 => Some(Self::WrongPhase),
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
            Self::WrongPhase => f.write_str("wrong phase"),
            Self::MalformedRequest => f.write_str("malformed request"),
        }
    }
}

/// Reason the orchestrator denied an operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum DenyReason {
    /// Component is isolated (compromise detected).
    Isolated = 0,
    /// Update policy violation.
    PolicyViolation = 1,
    /// Component identifier not recognized.
    UnknownTarget = 2,
    /// Another operation is in progress.
    Busy = 3,
}

impl DenyReason {
    pub const fn from_u8(val: u8) -> Option<Self> {
        match val {
            0 => Some(Self::Isolated),
            1 => Some(Self::PolicyViolation),
            2 => Some(Self::UnknownTarget),
            3 => Some(Self::Busy),
            _ => None,
        }
    }
}

impl fmt::Display for DenyReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Isolated => f.write_str("isolated"),
            Self::PolicyViolation => f.write_str("policy violation"),
            Self::UnknownTarget => f.write_str("unknown target"),
            Self::Busy => f.write_str("busy"),
        }
    }
}

/// Error returned to the orchestrator's client layer.
///
/// Wraps the on-wire `ResponseCode`, the way `MctpError` wraps
/// `mctp_api::ResponseCode`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PldmIpcError {
    pub code: ResponseCode,
}

impl PldmIpcError {
    pub const fn from_code(code: ResponseCode) -> Self {
        Self { code }
    }
}

impl fmt::Display for PldmIpcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "pldm ipc error: {}", self.code)
    }
}

impl From<ResponseCode> for PldmIpcError {
    fn from(code: ResponseCode) -> Self {
        Self::from_code(code)
    }
}
