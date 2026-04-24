// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

use crate::{ErrorCode, ErrorModule};
use pw_status::Error;

// TODO: review the pw_status error codes.

pub const FLASH_GENERIC: ErrorModule = ErrorModule::new(0x464c); //ascii `FL`.
pub const FLASH_GENERIC_BUSY: ErrorCode = FLASH_GENERIC.from_pw(0, Error::Unavailable);
pub const FLASH_GENERIC_ERASE_INVALID_ADDR: ErrorCode =
    FLASH_GENERIC.from_pw(1, Error::InvalidArgument);
pub const FLASH_GENERIC_BAD_ALIGNMENT: ErrorCode = FLASH_GENERIC.from_pw(2, Error::InvalidArgument);
pub const FLASH_GENERIC_READ_TOO_LONG: ErrorCode = FLASH_GENERIC.from_pw(3, Error::InvalidArgument);
pub const FLASH_GENERIC_PROGRAM_EXCEEDS_WINDOW_SIZE: ErrorCode =
    FLASH_GENERIC.from_pw(4, Error::InvalidArgument);
pub const FLASH_GENERIC_PROGRAM_SPANS_WINDOW_BOUNDARY: ErrorCode =
    FLASH_GENERIC.from_pw(5, Error::InvalidArgument);
pub const FLASH_GENERIC_ADDR_OUT_OF_BOUNDS: ErrorCode =
    FLASH_GENERIC.from_pw(6, Error::InvalidArgument);
pub const FLASH_GENERIC_INVALID_PAGE_SIZE: ErrorCode =
    FLASH_GENERIC.from_pw(7, Error::InvalidArgument);
pub const FLASH_GENERIC_INVALID_SIZE: ErrorCode = FLASH_GENERIC.from_pw(8, Error::InvalidArgument);

pub const FLASH_GENERIC_SFDP_INVALID_MEMORY_DENSITY: ErrorCode =
    FLASH_GENERIC.from_pw(1024, Error::InvalidArgument);
pub const FLASH_GENERIC_SFDP_INVALID_SIGNATURE: ErrorCode =
    FLASH_GENERIC.from_pw(1025, Error::InvalidArgument);
pub const FLASH_GENERIC_SFDP_NO_VALID_PARAMETER_HEADER_FOUND: ErrorCode =
    FLASH_GENERIC.from_pw(1026, Error::InvalidArgument);
pub const FLASH_GENERIC_SFDP_PARAMETERS_TOO_SHORT: ErrorCode =
    FLASH_GENERIC.from_pw(1027, Error::InvalidArgument);
pub const FLASH_GENERIC_SFDP_UNSUPPORTED_HEADER_MAJOR_REV: ErrorCode =
    FLASH_GENERIC.from_pw(1028, Error::InvalidArgument);
pub const FLASH_GENERIC_SFDP_UNSUPPORTED_PARAMS_MAJOR_REV: ErrorCode =
    FLASH_GENERIC.from_pw(1029, Error::InvalidArgument);
pub const FLASH_GENERIC_SFDP_PARAMETERS_TOO_LONG: ErrorCode =
    FLASH_GENERIC.from_pw(1030, Error::InvalidArgument);

pub const FLASH_OPENTITAN: ErrorModule = ErrorModule::new(0x464f); //ascii `FO`.
