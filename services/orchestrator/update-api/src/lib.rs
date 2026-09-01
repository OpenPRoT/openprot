// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! The update intake seam between the orchestrator and an update source.
//!
//! An update source is the protocol frontend that takes a candidate image
//! from outside the platform: the PLDM firmware-device service
//! (`services/pldm`) today, self-update later. This crate is the contract
//! and nothing else, so both processes depend on it and it builds on the
//! host: the [`UpdateIntake`] trait a source calls, the [`IntakeStatus`]
//! phases the orchestrator answers with, and the [`wire`] encoding between
//! them.
//!
//! The source is the kernel channel's initiator and the orchestrator its
//! handler, with no channel the other way. So the orchestrator never waits
//! on the source, and handling a request is one bounded step: no request
//! means "wait until the state machine decides". Everything the
//! orchestrator decides therefore comes back as the phase on a response,
//! read as often as the source needs ([`UpdateIntake::poll`]).
//!
//! Two consequences, both easy to undo by accident:
//!
//! - `Effect::ReportUpdateDeferred` and `Effect::ReportUpdateAborted` latch
//!   a phase, they do not send. The source collects them as
//!   [`FailureCause::Deferred`] / [`FailureCause::Superseded`].
//! - Polling need not be timer-driven: the handler can raise
//!   `Signals::USER` on the source's channel end
//!   (`syscall::object_set_peer_user_signal`, as `services/i2c` does) when
//!   the phase changes. That is a nudge carrying no data, and the syscall
//!   does not block on the peer.
//!
//! What backs the staging region is the board's choice and never the
//! source's. On a passive device the eRoT sits in the flash path, so the
//! region is that device's inactive slot and a write goes straight into it;
//! the eRoT then authenticates by reading the slot back. A board with its
//! own staging flash copies first instead. Either way the source offers,
//! writes at offsets, and completes.
//!
//! This is the `api` layer of the pattern `services/i2c` and
//! `services/mctp` established. The orchestrator-side dispatch and the
//! source-side [`UpdateIntake`] impl over `channel_transact` are separate
//! crates.

#![cfg_attr(not(test), no_std)]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod status;
pub mod traits;
pub mod wire;

pub use status::{FailureCause, IntakeStatus, Progress, Reject};
pub use traits::{IntakeError, UpdateIntake};
pub use wire::{Request, Response, UpdateOp};

/// Which managed device a candidate is for: the device's index in the
/// board's device table, which is what the driver validates an offer
/// against.
///
/// The orchestrator's `ComponentId` stays inside the orchestrator. A source
/// names a target, and the state machine never sees it at all: it only
/// decides that an update runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TargetId(pub u8);
