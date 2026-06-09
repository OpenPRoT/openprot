// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Kernel-specific error codes.

use crate::{ErrorCode, ErrorModule};

/// The kernel error module.
pub const KERNEL_ERROR: ErrorModule = ErrorModule::new(0x4b45); // ascii `KE`
/// The operation was cancelled.
pub const KERNEL_ERROR_CANCELLED: ErrorCode = KERNEL_ERROR.error(1);
/// An unknown error occurred in the kernel.
pub const KERNEL_ERROR_UNKNOWN: ErrorCode = KERNEL_ERROR.error(2);
/// An invalid argument was provided to a kernel call.
pub const KERNEL_ERROR_INVALID_ARGUMENT: ErrorCode = KERNEL_ERROR.error(3);
/// The deadline for the operation was exceeded.
pub const KERNEL_ERROR_DEADLINE_EXCEEDED: ErrorCode = KERNEL_ERROR.error(4);
/// The requested resource was not found.
pub const KERNEL_ERROR_NOT_FOUND: ErrorCode = KERNEL_ERROR.error(5);
/// The resource already exists.
pub const KERNEL_ERROR_ALREADY_EXISTS: ErrorCode = KERNEL_ERROR.error(6);
/// Permission was denied for the operation.
pub const KERNEL_ERROR_PERMISSION_DENIED: ErrorCode = KERNEL_ERROR.error(7);
/// Resources have been exhausted.
pub const KERNEL_ERROR_RESOURCE_EXHAUSTED: ErrorCode = KERNEL_ERROR.error(8);
/// A precondition for the operation failed.
pub const KERNEL_ERROR_FAILED_PRECONDITION: ErrorCode = KERNEL_ERROR.error(9);
/// The operation was aborted.
pub const KERNEL_ERROR_ABORTED: ErrorCode = KERNEL_ERROR.error(10);
/// The value is out of range.
pub const KERNEL_ERROR_OUT_OF_RANGE: ErrorCode = KERNEL_ERROR.error(11);
/// The operation is unimplemented.
pub const KERNEL_ERROR_UNIMPLEMENTED: ErrorCode = KERNEL_ERROR.error(12);
/// An internal kernel error occurred.
pub const KERNEL_ERROR_INTERNAL: ErrorCode = KERNEL_ERROR.error(13);
/// The service or resource is unavailable.
pub const KERNEL_ERROR_UNAVAILABLE: ErrorCode = KERNEL_ERROR.error(14);
/// Data loss has occurred.
pub const KERNEL_ERROR_DATA_LOSS: ErrorCode = KERNEL_ERROR.error(15);
/// The caller is unauthenticated.
pub const KERNEL_ERROR_UNAUTHENTICATED: ErrorCode = KERNEL_ERROR.error(16);
