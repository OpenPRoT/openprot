// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! `openprot_orchestrator_driver` — the effect-executing layer around the
//! orchestrator state machine.
//!
//! [`PlatformDriver`] implements the SM's [`Platform`] seam, delegating each
//! `Effect` to a board-composed capability per the platform-boundary contract
//! (`docs/src/design/orchestrator/orchestrator-model.md` §6). Effects whose
//! capability is not composed yet fail closed in `execute`; the driver grows
//! an executor only when the capability it delegates to exists.
//!
//! Synchronous results (the verification verdict) return through `execute`;
//! the SM queues and settles them within the same dispatch run. There is no
//! driver-side event queue.
//!
//! Everything device-specific arrives through the seams in [`board`]:
//! image access ([`ImageSource`]), image judgment ([`Verifier`]), reset
//! actuation ([`orchestrator_capabilities::BootControl`]), boot
//! supervision ([`orchestrator_capabilities::BootWatch`]) and the
//! anti-rollback floor ([`orchestrator_capabilities::SvnFloor`]), bundled
//! in one [`Board`] built by the board's composition crate.
//!
//! Boot-walk verdicts are the one asynchronous read: the run loop calls
//! [`PlatformDriver::poll_boot_walks`] and dispatches the returned events
//! (`ComponentReady`/`Booted`/`Timeout`) into the SM.
//!
//! [`Platform`]: openprot_orchestrator_sm::Platform

#![no_std]
#![forbid(unsafe_code)]

mod board;
mod driver;
#[cfg(test)]
mod tests;

pub use board::{
    Board, BoardCapabilities, ImageSource, Report, ReportSink, SvnFloorBinding, Verdict, Verifier,
};
pub use driver::{BootWalkPoll, DriverError, PlatformDriver};
