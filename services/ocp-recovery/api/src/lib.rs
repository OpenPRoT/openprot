// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Register layouts of the OCP Recovery interface, for the eRoT serving them
//! as the device.
//!
//! The BMC is the Recovery Agent and drives the registers over SMBus block
//! transactions; the eRoT answers as the Device, the word the recovery spec
//! uses for the side being recovered. This crate holds
//! the half that does not depend on the transport: the command codes, the
//! byte layouts, the field encodings and the rules for which accesses the
//! device refuses. The SMBus framing, the PEC and the register file the
//! server-runtime owns live elsewhere, so this crate stays host-testable and
//! free of kernel dependencies.
//!
//! [`RecoveryCommand`] decodes the command byte once and carries the access
//! rule, the length and the availability scope with it. The register structs
//! in [`regs`] match the spec byte layout, so the handler writes their bytes
//! to the transport without a conversion step.
//!
//! The AST1030 has no OCP recovery silicon, so the device side is firmware
//! and has to keep the access semantics that the agent state machine expects
//! from hardware. Three of them are easy to get wrong, and each is a named
//! operation here rather than a plain field write: `RECOVERY_CTRL` byte 2 is
//! activated by the agent writing [`ACTIVATE_RECOVERY_IMAGE`] and zeroed by
//! the device afterwards, `DEVICE_RESET` byte 0 works the same way, and
//! `DEVICE_STATUS` byte 1 clears when the agent reads it.
//!
//! [`ACTIVATE_RECOVERY_IMAGE`]: regs::ACTIVATE_RECOVERY_IMAGE

#![cfg_attr(not(test), no_std)]
#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    clippy::todo,
    clippy::unimplemented
)]
// Tests unwrap results whose shape the same lines settle.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

pub mod command;
pub mod error;
pub mod regs;

pub use command::{Access, Length, RecoveryCommand, Scope};
pub use error::{ProtocolError, WireError};

/// Largest payload one SMBus access to a data aperture may carry.
///
/// The spec caps `INDIRECT_DATA` and `INDIRECT_FIFO_DATA` at 252 bytes for
/// SMBus. I3C and USB cap them at a size the transport negotiates instead,
/// which is why the cap is an argument to
/// [`RecoveryCommand::check_write`](command::RecoveryCommand::check_write)
/// rather than a constant this crate applies on its own.
pub const SMBUS_MAX_PAYLOAD: usize = 252;
