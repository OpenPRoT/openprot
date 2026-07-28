// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Error types for the PLDM service.

use core::fmt;

use openprot_mctp_api::MctpError;
use pldm_interface::error::MsgHandlerError;

/// Errors returned by PLDM service operations.
#[derive(Debug)]
pub enum PldmServiceError {
    /// An MCTP transport or stack error.
    Mctp(MctpError),
    /// A PLDM message handler error (codec failure, unsupported command, etc.).
    MsgHandler(MsgHandlerError),
    /// A buffer size or arithmetic overflow.
    Overflow,
}

impl fmt::Display for PldmServiceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PldmServiceError::Mctp(e) => write!(f, "MCTP transport error: {e:?}"),
            PldmServiceError::MsgHandler(e) => write!(f, "PLDM message handler error: {e:?}"),
            PldmServiceError::Overflow => write!(f, "buffer size or arithmetic overflow"),
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
