// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Pinmux-specific error codes for Earlgrey.

use pw_status::Error;
use util_error::{ErrorCode, ErrorModule};

/// The Earlgrey Pinmux error module (ASCII `'EP'`).
pub const EG_PINMUX: ErrorModule = ErrorModule::new(0x4550);

/// The requested input configuration is not supported.
pub const EG_PINMUX_INVALID_INPUT: ErrorCode = EG_PINMUX.from_pw(1, Error::InvalidArgument);

/// The requested output configuration is not supported.
pub const EG_PINMUX_INVALID_OUTPUT: ErrorCode = EG_PINMUX.from_pw(2, Error::InvalidArgument);

/// The requested pad is invalid or does not support configuration.
pub const EG_PINMUX_INVALID_PAD: ErrorCode = EG_PINMUX.from_pw(3, Error::InvalidArgument);
