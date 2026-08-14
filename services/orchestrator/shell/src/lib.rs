// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! `openprot_orchestrator_shell` — the effect-executing layer around the
//! orchestrator state machine.
//!
//! Everything device-specific arrives through the seams in [`board`]:
//! image access ([`ImageSource`]) and image judgment ([`Verifier`]),
//! bundled in one [`Board`] built by the board's composition crate.

#![no_std]
#![forbid(unsafe_code)]

mod board;

pub use board::{Board, BoardTypes, ImageSource, Verdict, Verifier};
