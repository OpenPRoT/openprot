// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! SPI-specific error codes.

use crate::{ErrorCode, ErrorModule};
use pw_status::Error;

/// The generic SPI error module.
pub const SPI_GENERIC: ErrorModule = ErrorModule::new(0x5350); // ascii `SP`.

/// Invalid transaction parameters or state.
pub const SPI_GENERIC_INVALID_TRANSACTION: ErrorCode =
    SPI_GENERIC.from_pw(1, Error::InvalidArgument);
/// TX FIFO overflow.
pub const SPI_GENERIC_FIFO_OVERFLOW: ErrorCode = SPI_GENERIC.from_pw(2, Error::ResourceExhausted);
/// RX FIFO underflow.
pub const SPI_GENERIC_FIFO_UNDERFLOW: ErrorCode = SPI_GENERIC.from_pw(3, Error::ResourceExhausted);
/// Operation timed out.
pub const SPI_GENERIC_TIMEOUT: ErrorCode = SPI_GENERIC.from_pw(4, Error::DeadlineExceeded);
/// Hardware error reported by the controller.
pub const SPI_GENERIC_HARDWARE_ERROR: ErrorCode = SPI_GENERIC.from_pw(5, Error::Internal);
