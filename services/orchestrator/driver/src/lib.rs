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
//! Nothing the driver executes waits on a slow service. `VerifyFirmware`
//! starts a check and returns; the verdict comes back later from
//! [`PlatformDriver::poll`] as an event for the run loop to dispatch.
//! There is no driver-side event queue: the run loop polls. Short local calls
//! (reset lines, the SVN floor, reports) still complete inside `execute`.
//!
//! Everything device-specific arrives through the seams in [`board`]:
//! image access ([`ImageSource`]), image verification ([`Verifier`], start and
//! poll), reset actuation ([`orchestrator_capabilities::BootControl`]), boot
//! supervision ([`orchestrator_capabilities::BootWatch`]) and the
//! anti-rollback floor ([`orchestrator_capabilities::SvnFloor`]), bundled
//! in one [`Board`] built by the board's composition crate.
//!
//! Verification verdicts and boot-walk verdicts are the asynchronous reads:
//! the run loop calls [`PlatformDriver::poll`], which covers both, and
//! dispatches the returned events
//! (`VerificationPassed`/`VerificationFailed`, `ComponentReady`/`Booted`/
//! `BootFailed`) into the SM.
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
    VerifyStatus,
};
pub use driver::{BootWalkPoll, DriverError, DriverPoll, PlatformDriver};
