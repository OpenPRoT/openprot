// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! The device side of the OCP Recovery interface: the registers the
//! Recovery Agent addresses and the dispatch that answers it.
//!
//! Everything here is transport-free and host-testable, like
//! `services/i2c/server`, which holds the I2C dispatch while
//! `services/i2c/server-runtime` holds the loop. The interface instance
//! belongs in the i2c server-runtime loop, because only that loop can answer
//! an SMBus read within the clock-stretch budget; this crate gives the loop a
//! [`dispatch`] to call and the policy side
//! [`RecoveryInterface::take_activation`] and
//! [`RecoveryInterface::take_reset_request`] to poll.
//!
//! The dispatch is mechanical: it serves register bytes, latches protocol
//! errors into DEVICE_STATUS byte 1 and drains `INDIRECT_FIFO_DATA` payloads
//! into a [`FifoSink`]. The caller decides what a state means, when the
//! device status changes and what happens after an activation, through the
//! setters, `take_activation` and `take_reset_request`.

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
pub use registers::{ActivationRequest, RecoveryInterface};
pub use staging::{FifoSink, RamStaging, StagingError};
