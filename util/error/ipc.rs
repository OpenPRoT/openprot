// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

use crate::{ErrorCode, ErrorModule};
use pw_status::Error;

pub const IPC_ERROR: ErrorModule = ErrorModule::new(0x4943); // ascii `IC`
pub const IPC_ERROR_RSP_BAD_LEN: ErrorCode = IPC_ERROR.from_pw(1, Error::InvalidArgument);
pub const IPC_ERROR_RSP_TOO_LARGE: ErrorCode = IPC_ERROR.from_pw(2, Error::InvalidArgument);
pub const IPC_ERROR_BAD_REQ: ErrorCode = IPC_ERROR.from_pw(3, Error::InvalidArgument);
pub const IPC_ERROR_BAD_REQ_LEN: ErrorCode = IPC_ERROR.from_pw(4, Error::InvalidArgument);
pub const IPC_ERROR_UNKNOWN_OP: ErrorCode = IPC_ERROR.from_pw(5, Error::Unknown);
