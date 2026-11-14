// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! A managed device that boots, hangs, or dies on a script, for tests.
//!
//! The orchestrator drives a device's reset line and watches whether it comes
//! up. This models what sits on the other end of that line, and nothing else.
//! It is a pure state machine: no pins, no clock, no kernel, no dependencies.
//!
//! # What it models
//!
//! - **Reset is the input, ready is the output.** Holding reset keeps the
//!   device down and its ready level low. Releasing reset starts one boot
//!   attempt.
//! - **The device owns its behaviour.** A [`Behaviour`] script says what each
//!   boot attempt does, by how many times reset has been released. The system
//!   under test never tells the device how to act, so it cannot shape the
//!   world it is being judged against.
//! - **Hardware semantics.** The boot delay counts from the release, a reset
//!   mid-boot restarts it, and asserting reset drops the ready level.
//!
//! # Time
//!
//! Time is an argument, in milliseconds on the caller's clock. A transition
//! is stamped with the moment it was *due*, not the moment a late caller
//! noticed it, so a test can check exact timing however coarse its polling,
//! and an event loop can sleep until [`DeviceModel::next_event_at`] and wake
//! on the dot.
//!
//! # Observing it
//!
//! The ready level is [`DeviceModel::ready`]. A caller that wants edges, such
//! as a service that forwards "device booted" as a message, drains
//! [`DeviceModel::take_transition`], which does not lose a pulse that rose
//! and fell between two polls.
//!
//! The model does not know what carries these signals. A board adapter can
//! map them to pins; a simulation can map them to IPC messages.

#![no_std]
#![forbid(unsafe_code)]

mod behaviour;
mod model;

pub use behaviour::{Behaviour, Phase, Transition};
pub use model::DeviceModel;

#[cfg(test)]
mod tests;
