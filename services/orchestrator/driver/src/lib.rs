// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! `openprot_orchestrator_driver` — the effect-executing layer around the
//! orchestrator state machine.
//!
//! [`PlatformDriver`] implements the SM's [`Platform`] seam: one method per `Effect`,
//! each documenting its obligation from the platform-boundary contract
//! (`docs/src/design/orchestrator/orchestrator-model.md` §6). Unimplemented
//! executors return [`DriverError::NotImplemented`]; the SM fail-closes on
//! them.
//!
//! Executor-produced events queue in the driver; the event loop drains them
//! via [`PlatformDriver::take_event`] and dispatches each.
//!
//! Everything device-specific arrives through the seams in [`board`]:
//! image access ([`ImageSource`]) and image judgment ([`Verifier`]),
//! bundled in one [`Board`] built by the board's composition crate.
//!
//! [`Platform`]: openprot_orchestrator_sm::Platform

#![no_std]
#![forbid(unsafe_code)]

mod board;
mod driver;
#[cfg(test)]
mod tests;

pub use board::{Board, BoardCapabilities, ImageSource, Verdict, Verifier};
pub use driver::{DriverError, PlatformDriver};
