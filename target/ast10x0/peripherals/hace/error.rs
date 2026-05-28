// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! HACE error definitions.

use openprot_hal_blocking::cipher::{Error as CipherError, ErrorKind as CipherErrorKind};
use openprot_hal_blocking::digest::{Error as DigestError, ErrorKind};
use openprot_hal_blocking::mac::{Error as MacError, ErrorKind as MacErrorKind};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum HaceError {
    /// Engine was not able to accept a new operation.
    Busy,
    /// Operation did not complete before the timeout budget.
    Timeout,
    /// Caller-provided parameters are invalid.
    InvalidInput,
    /// Unexpected hardware/software failure.
    Internal,
}

impl DigestError for HaceError {
    fn kind(&self) -> ErrorKind {
        match self {
            HaceError::Busy => ErrorKind::HardwareFailure,
            HaceError::Timeout => ErrorKind::HardwareFailure,
            HaceError::InvalidInput => ErrorKind::InvalidInputLength,
            HaceError::Internal => ErrorKind::HardwareFailure,
        }
    }
}

impl MacError for HaceError {
    fn kind(&self) -> MacErrorKind {
        match self {
            HaceError::Busy => MacErrorKind::Busy,
            HaceError::Timeout => MacErrorKind::HardwareFailure,
            HaceError::InvalidInput => MacErrorKind::InvalidInputLength,
            HaceError::Internal => MacErrorKind::HardwareFailure,
        }
    }
}

impl CipherError for HaceError {
    fn kind(&self) -> CipherErrorKind {
        match self {
            HaceError::Busy => CipherErrorKind::HardwareFailure,
            HaceError::Timeout => CipherErrorKind::HardwareFailure,
            HaceError::InvalidInput => CipherErrorKind::InvalidInput,
            HaceError::Internal => CipherErrorKind::HardwareFailure,
        }
    }
}
