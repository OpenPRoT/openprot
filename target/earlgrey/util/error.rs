// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

use pw_status::Error;
use util_error::{ErrorCode, ErrorModule};

// TODO: review the pw_status error codes.

pub const EG_ERROR: ErrorModule = ErrorModule::new(0x464c); //ascii `FL`.
pub const EG_ERROR_CERT_NOT_FOUND: ErrorCode = EG_ERROR.from_pw(1, Error::NotFound);
pub const EG_ERROR_CERT_BAD_NAME: ErrorCode = EG_ERROR.from_pw(2, Error::InvalidArgument);
pub const EG_ERROR_BAD_BOOT_LOG: ErrorCode = EG_ERROR.from_pw(3, Error::Unknown);
pub const EG_ERROR_BOOT_SLOT_UNKNOWN: ErrorCode = EG_ERROR.from_pw(4, Error::Unknown);
