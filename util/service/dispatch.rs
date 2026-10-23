// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Server end of an IPC round-trip.

use crate::TransportError;

/// Why a dispatch produced no response frame at all.
///
/// Service errors go in the response frame (still `Ok`). This covers
/// the case where even an error frame does not fit.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DispatchError {
    /// The response does not fit the buffer the caller gave, not even
    /// as an error frame.
    ResponseTooLarge,
}

impl core::fmt::Display for DispatchError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::ResponseTooLarge => f.write_str("response does not fit the buffer"),
        }
    }
}

impl core::error::Error for DispatchError {}

impl From<DispatchError> for TransportError {
    fn from(e: DispatchError) -> Self {
        match e {
            DispatchError::ResponseTooLarge => Self::TooLarge,
        }
    }
}

/// Server end: one request frame in, one response frame out.
///
/// Returns bytes written to `response`. No state between calls beyond
/// what the service itself owns.
pub trait Dispatch {
    fn dispatch(&mut self, request: &[u8], response: &mut [u8]) -> Result<usize, DispatchError>;
}
