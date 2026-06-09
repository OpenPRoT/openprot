// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Common constants and types shared between SPDM requester and responder.

#![no_std]
#![warn(missing_docs)]

/// Default data transfer size (bytes).
///
/// This is the maximum payload size for a single SPDM message fragment.
/// Value: 0x1200 (4608 bytes)
pub const DEFAULT_DTS: u32 = 0x1200;

/// Default maximum SPDM message size (bytes).
///
/// This is the maximum size of a complete SPDM message (may span multiple fragments).
/// Value: 0x1200 (4608 bytes)
pub const DEFAULT_SMS: u32 = 0x1200;
