// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

use crate::{ErrorCode, ErrorModule};

pub const KERNEL_ERROR: ErrorModule = ErrorModule::new(0x4b45); // ascii `KE`
pub const KERNEL_ERROR_CANCELLED: ErrorCode = KERNEL_ERROR.error(1);
pub const KERNEL_ERROR_UNKNOWN: ErrorCode = KERNEL_ERROR.error(2);
pub const KERNEL_ERROR_INVALID_ARGUMENT: ErrorCode = KERNEL_ERROR.error(3);
pub const KERNEL_ERROR_DEADLINE_EXCEEDED: ErrorCode = KERNEL_ERROR.error(4);
pub const KERNEL_ERROR_NOT_FOUND: ErrorCode = KERNEL_ERROR.error(5);
pub const KERNEL_ERROR_ALREADY_EXISTS: ErrorCode = KERNEL_ERROR.error(6);
pub const KERNEL_ERROR_PERMISSION_DENIED: ErrorCode = KERNEL_ERROR.error(7);
pub const KERNEL_ERROR_RESOURCE_EXHAUSTED: ErrorCode = KERNEL_ERROR.error(8);
pub const KERNEL_ERROR_FAILED_PRECONDITION: ErrorCode = KERNEL_ERROR.error(9);
pub const KERNEL_ERROR_ABORTED: ErrorCode = KERNEL_ERROR.error(10);
pub const KERNEL_ERROR_OUT_OF_RANGE: ErrorCode = KERNEL_ERROR.error(11);
pub const KERNEL_ERROR_UNIMPLEMENTED: ErrorCode = KERNEL_ERROR.error(12);
pub const KERNEL_ERROR_INTERNAL: ErrorCode = KERNEL_ERROR.error(13);
pub const KERNEL_ERROR_UNAVAILABLE: ErrorCode = KERNEL_ERROR.error(14);
pub const KERNEL_ERROR_DATA_LOSS: ErrorCode = KERNEL_ERROR.error(15);
pub const KERNEL_ERROR_UNAUTHENTICATED: ErrorCode = KERNEL_ERROR.error(16);
