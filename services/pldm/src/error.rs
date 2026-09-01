// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Error types for the PLDM service.

use core::fmt;

use openprot_mctp_api::MctpError;
use pldm_interface::error::MsgHandlerError;

/// Buffer and arithmetic bounds-check failures.
///
/// These variants stand in for the panicking operations (direct slice
/// indexing, unchecked `+`) that this crate avoids; each is produced by a
/// `.get()`/`.get_mut()`/`checked_add()` fallback rather than an actual
/// panic.
#[derive(Debug)]
pub enum PldmMemError {
    /// A buffer expected to hold at least one byte (the MCTP framing byte)
    /// was empty.
    MalformedBuffer,
    /// A length computation overflowed, or a computed length exceeds the
    /// capacity of the buffer it would be used with.
    OverflowMaxSize,
    /// A slice operation (e.g. `buf.get(range)`) failed because `buf` was
    /// shorter than the requested range.
    BufferTooSmall,
}

/// Errors returned by PLDM service operations.
#[derive(Debug)]
pub enum PldmServiceError {
    /// An MCTP transport or stack error.
    Mctp(MctpError),
    /// A PLDM message handler error (codec failure, unsupported command, etc.).
    MsgHandler(MsgHandlerError),
    /// A buffer or arithmetic bounds-check failure; see [`PldmMemError`].
    PldmMem(PldmMemError),
}

impl fmt::Display for PldmServiceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PldmServiceError::Mctp(e) => write!(f, "MCTP transport error: {e:?}"),
            PldmServiceError::MsgHandler(e) => write!(f, "PLDM message handler error: {e:?}"),
            PldmServiceError::PldmMem(e) => write!(f, "buffer size or arithmetic overflow: {e:?}"),
        }
    }
}

impl core::error::Error for PldmServiceError {}

impl From<MctpError> for PldmServiceError {
    fn from(e: MctpError) -> Self {
        PldmServiceError::Mctp(e)
    }
}

impl From<MsgHandlerError> for PldmServiceError {
    fn from(e: MsgHandlerError) -> Self {
        PldmServiceError::MsgHandler(e)
    }
}
