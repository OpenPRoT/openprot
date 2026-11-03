// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! The device side of the OCP Recovery interface: the register file and the
//! dispatch that answers the Recovery Agent.
//!
//! Everything here is transport-free and host-testable, the
//! `services/i2c/server` half of the pattern. The register file instance
//! lives in the i2c server-runtime loop at run time, because only that loop
//! can answer an SMBus read within the clock-stretch budget; this crate gives
//! the loop a [`dispatch`] to call and the policy side the event takes
//! ([`RecoveryRegisterFile::take_activation`],
//! [`RecoveryRegisterFile::take_reset_request`]) to poll.
//!
//! The dispatch is mechanical: it serves register bytes, latches protocol
//! errors into DEVICE_STATUS byte 1 and drains `INDIRECT_FIFO_DATA` payloads
//! into a [`FifoSink`]. What a state means, when the device status changes
//! and what happens after an activation is policy, decided by the caller
//! through the setters and takes.

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

mod dispatch;
mod registers;
mod staging;

pub use dispatch::dispatch;
pub use registers::{ActivationRequest, RecoveryRegisterFile};
pub use staging::{FifoSink, RamStaging, StagingError};
