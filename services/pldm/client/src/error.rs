// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! What a round-trip can fail with.

use pldm_api::{ResponseCode, WireError};
use util_service::TransportError;

/// Why a round-trip did not produce an answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientError {
    /// The transport failed, or was used in the wrong order.
    Transport(TransportError),
    /// The response did not decode.
    Wire(WireError),
    /// The FD answered, and the answer is a refusal. Carries the code the
    /// FD sent: a decision, not a fault in the channel.
    Refused(ResponseCode),
    /// A request is already in flight. One at a time, so the caller
    /// collects the outstanding reply first.
    Busy,
    /// `poll` or `cancel` with nothing in flight.
    Idle,
}

impl core::fmt::Display for ClientError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Transport(e) => write!(f, "pldm ipc transport error: {e}"),
            Self::Wire(e) => write!(f, "pldm ipc response did not decode: {e}"),
            Self::Refused(code) => write!(f, "firmware device refused: {code}"),
            Self::Busy => f.write_str("a pldm ipc request is already in flight"),
            Self::Idle => f.write_str("no pldm ipc request in flight"),
        }
    }
}

impl core::error::Error for ClientError {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::Transport(e) => Some(e),
            Self::Wire(e) => Some(e),
            Self::Refused(_) | Self::Busy | Self::Idle => None,
        }
    }
}

impl From<TransportError> for ClientError {
    fn from(e: TransportError) -> Self {
        Self::Transport(e)
    }
}

impl From<WireError> for ClientError {
    fn from(e: WireError) -> Self {
        Self::Wire(e)
    }
}
