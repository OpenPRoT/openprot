// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Flash-specific error codes.

use crate::{ErrorCode, ErrorModule};
use pw_status::Error;

// TODO: review the pw_status error codes.

/// The generic flash error module.
pub const FLASH_GENERIC: ErrorModule = ErrorModule::new(0x464c); //ascii `FL`.
/// The flash device is busy.
pub const FLASH_GENERIC_BUSY: ErrorCode = FLASH_GENERIC.from_pw(0, Error::Unavailable);
/// The erase address is invalid (e.g., not page-aligned).
pub const FLASH_GENERIC_ERASE_INVALID_ADDR: ErrorCode =
    FLASH_GENERIC.from_pw(1, Error::InvalidArgument);
/// The operation has bad alignment.
pub const FLASH_GENERIC_BAD_ALIGNMENT: ErrorCode = FLASH_GENERIC.from_pw(2, Error::InvalidArgument);
/// The read request is too long.
pub const FLASH_GENERIC_READ_TOO_LONG: ErrorCode = FLASH_GENERIC.from_pw(3, Error::InvalidArgument);
/// The program operation exceeds the hardware window size.
pub const FLASH_GENERIC_PROGRAM_EXCEEDS_WINDOW_SIZE: ErrorCode =
    FLASH_GENERIC.from_pw(4, Error::InvalidArgument);
/// The program operation spans a hardware window boundary.
pub const FLASH_GENERIC_PROGRAM_SPANS_WINDOW_BOUNDARY: ErrorCode =
    FLASH_GENERIC.from_pw(5, Error::InvalidArgument);
/// The address is out of bounds.
pub const FLASH_GENERIC_ADDR_OUT_OF_BOUNDS: ErrorCode =
    FLASH_GENERIC.from_pw(6, Error::InvalidArgument);
/// The flash page size is invalid.
pub const FLASH_GENERIC_INVALID_PAGE_SIZE: ErrorCode =
    FLASH_GENERIC.from_pw(7, Error::InvalidArgument);
/// The flash size is invalid.
pub const FLASH_GENERIC_INVALID_SIZE: ErrorCode = FLASH_GENERIC.from_pw(8, Error::InvalidArgument);
/// The erase size is invalid.
pub const FLASH_GENERIC_ERASE_INVALID_SIZE: ErrorCode =
    FLASH_GENERIC.from_pw(9, Error::InvalidArgument);
/// The flash device is not initialized.
pub const FLASH_GENERIC_NOT_INITIALIZED: ErrorCode =
    FLASH_GENERIC.from_pw(10, Error::FailedPrecondition);

/// SFDP: Invalid memory density.
pub const FLASH_GENERIC_SFDP_INVALID_MEMORY_DENSITY: ErrorCode =
    FLASH_GENERIC.from_pw(1024, Error::InvalidArgument);
/// SFDP: Invalid signature.
pub const FLASH_GENERIC_SFDP_INVALID_SIGNATURE: ErrorCode =
    FLASH_GENERIC.from_pw(1025, Error::InvalidArgument);
/// SFDP: No valid parameter header found.
pub const FLASH_GENERIC_SFDP_NO_VALID_PARAMETER_HEADER_FOUND: ErrorCode =
    FLASH_GENERIC.from_pw(1026, Error::InvalidArgument);
/// SFDP: Parameters are too short.
pub const FLASH_GENERIC_SFDP_PARAMETERS_TOO_SHORT: ErrorCode =
    FLASH_GENERIC.from_pw(1027, Error::InvalidArgument);
/// SFDP: Unsupported header major revision.
pub const FLASH_GENERIC_SFDP_UNSUPPORTED_HEADER_MAJOR_REV: ErrorCode =
    FLASH_GENERIC.from_pw(1028, Error::InvalidArgument);
/// SFDP: Unsupported parameters major revision.
pub const FLASH_GENERIC_SFDP_UNSUPPORTED_PARAMS_MAJOR_REV: ErrorCode =
    FLASH_GENERIC.from_pw(1029, Error::InvalidArgument);
/// SFDP: Parameters are too long.
pub const FLASH_GENERIC_SFDP_PARAMETERS_TOO_LONG: ErrorCode =
    FLASH_GENERIC.from_pw(1030, Error::InvalidArgument);

/// The OpenTitan flash error module.
pub const FLASH_OPENTITAN: ErrorModule = ErrorModule::new(0x464f); //ascii `FO`.
