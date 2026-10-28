// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Errors raised while decoding an access from the Recovery Agent.

use crate::command::RecoveryCommand;

/// Why an access from the Recovery Agent could not be served.
///
/// Each variant maps to one of the protocol error codes the spec defines for
/// byte 1 of `DEVICE_STATUS`, so a handler converts with `ProtocolError::from`
/// and reports the result without a second match.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum WireError {
    /// The command byte names no recovery command.
    UnknownCommand(u8),
    /// The Recovery Agent wrote a command the device serves read-only.
    WriteToReadOnly(RecoveryCommand),
    /// The write carried the wrong number of bytes for the command.
    LengthMismatch {
        /// The command the Recovery Agent addressed.
        command: RecoveryCommand,
        /// How many bytes arrived.
        got: usize,
    },
    /// A field held a value the spec reserves or does not define.
    UnsupportedParameter {
        /// The command the Recovery Agent addressed.
        command: RecoveryCommand,
        /// Which byte of the register carried the value.
        offset: usize,
        /// The value that arrived.
        value: u8,
    },
}

impl core::fmt::Display for WireError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::UnknownCommand(byte) => write!(f, "unknown recovery command 0x{byte:02x}"),
            Self::WriteToReadOnly(command) => write!(f, "{command:?} is read-only"),
            Self::LengthMismatch { command, got } => {
                write!(f, "{command:?} write carried {got} bytes")
            }
            Self::UnsupportedParameter {
                command,
                offset,
                value,
            } => write!(f, "{command:?} byte {offset} holds 0x{value:02x}"),
        }
    }
}

impl core::error::Error for WireError {}

/// The protocol error codes of `DEVICE_STATUS` byte 1.
///
/// The device latches the code and clears it when the Recovery Agent reads
/// `DEVICE_STATUS`, which is why [`DeviceStatus::take_protocol_error`] is the
/// only way to read it.
///
/// [`DeviceStatus::take_protocol_error`]: crate::regs::DeviceStatus::take_protocol_error
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
#[non_exhaustive]
pub enum ProtocolError {
    /// Nothing to report.
    #[default]
    None = 0x0,
    /// The command is not supported, or the Recovery Agent wrote a read-only
    /// command.
    UnsupportedCommand = 0x1,
    /// A field held a reserved or undefined value.
    UnsupportedParameter = 0x2,
    /// The write carried the wrong number of bytes.
    LengthWriteError = 0x3,
    /// The PEC of the transaction did not match.
    CrcError = 0x4,
    /// Catch-all for errors the other codes do not classify.
    General = 0xff,
}

impl From<WireError> for ProtocolError {
    fn from(e: WireError) -> Self {
        match e {
            WireError::UnknownCommand(_) | WireError::WriteToReadOnly(_) => {
                Self::UnsupportedCommand
            }
            WireError::LengthMismatch { .. } => Self::LengthWriteError,
            WireError::UnsupportedParameter { .. } => Self::UnsupportedParameter,
        }
    }
}
