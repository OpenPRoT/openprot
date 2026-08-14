// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! `openprot_orchestrator_shell` — the effect-executing layer around the
//! orchestrator state machine.
//!
//! [`Shell`] implements the SM's [`Platform`] seam: one method per `Effect`,
//! each documenting its obligation from the platform-boundary contract
//! (`docs/src/design/orchestrator/orchestrator-model.md` §6). Unimplemented
//! executors return [`ShellError::NotImplemented`]; the SM fail-closes on
//! them.
//!
//! Executor-produced events queue in the shell; the driver loop drains them
//! via [`Shell::take_event`] and dispatches each.
//!
//! Everything device-specific arrives through the seams in [`board`]:
//! image access ([`ImageSource`]) and image judgment ([`Verifier`]),
//! bundled in one [`Board`] built by the board's composition crate.
//!
//! [`Platform`]: openprot_orchestrator_sm::Platform

#![no_std]
#![forbid(unsafe_code)]

mod board;
mod shell;
#[cfg(test)]
mod tests;

pub use board::{Board, BoardTypes, ImageSource, Verdict, Verifier};
pub use shell::{Shell, ShellError};
