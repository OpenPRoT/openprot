// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! The MCTP endpoint IDs of the MCTP over I3C test, shared by the firmware apps
//! and the host test, so they cannot drift apart.

#![no_std]

/// The target's EID: the MCTP server runs its stack at it, and the echo and
/// control apps report it.
pub const OWN_EID: u8 = 8;

/// The host's (bus owner's) EID, where the target sends the requests it
/// initiates.
pub const HOST_EID: u8 = 9;
