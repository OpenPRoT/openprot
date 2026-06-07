// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Error module and codes for Earlgrey target utilities.

use pw_status::Error;
use util_error::{ErrorCode, ErrorModule};

/// The Earlgrey utility error module (ASCII `'FL'`).
pub const EG_ERROR: ErrorModule = ErrorModule::new(0x464c);

/// Certificate was not found in personalization data.
pub const EG_ERROR_CERT_NOT_FOUND: ErrorCode = EG_ERROR.from_pw(1, Error::NotFound);
/// Certificate name is not valid UTF-8.
pub const EG_ERROR_CERT_BAD_NAME: ErrorCode = EG_ERROR.from_pw(2, Error::InvalidArgument);
/// The boot log integrity check failed.
pub const EG_ERROR_BAD_BOOT_LOG: ErrorCode = EG_ERROR.from_pw(3, Error::Unknown);
/// The boot slot is unknown or invalid.
pub const EG_ERROR_BOOT_SLOT_UNKNOWN: ErrorCode = EG_ERROR.from_pw(4, Error::Unknown);
