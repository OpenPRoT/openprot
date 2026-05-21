// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Error types for the PLDM service.

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
    /// A response length exceeded the available buffer.
    InvalidResponseLength,
    /// An IPC channel error (read, respond, or transact failure).
    Ipc,
}

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
