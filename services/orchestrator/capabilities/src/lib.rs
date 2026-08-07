// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Device-facing capability traits for the Boot Orchestrator.
//!
//! `BootControl` is the actuation capability: the orchestrator drives a
//! single managed device's reset without knowing which controller line it
//! maps to.
//!
//! `BootStatus` is the shared vocabulary for boot-liveness evidence, and
//! `EvidenceReader` resolves a board-defined signal id to it. The schema
//! names no signal kinds: each board's device table declares its
//! checkpoints as data (`BootCheckpoint` in `orchestrator-config`), and the
//! board's reader gives the ids meaning.
//!
//! `BootWatch` is the seam the orchestrator polls: one device's boot walk,
//! erased of every device-specific type, answering with a `WalkVerdict`.
//!
//! This crate is a dependency-free leaf: it holds the capability contracts,
//! and everything depends downward on it. Concrete adapters bind a capability
//! to a signal source and live in their own crates, so naming a capability
//! never drags in the stack behind it — the HAL-backed `HalBootControl` and
//! the `GpioBootMonitor` read helper are in `orchestrator-hal-adapters`. The
//! per-board device table schema lives in the separate `orchestrator-config`
//! crate; board tables (`target/<board>/devices.rs`) declare the values.

#![cfg_attr(not(test), no_std)]

mod boot_control;
mod boot_watch;
mod evidence;

pub use boot_control::BootControl;
pub use boot_watch::{BootWatch, FailureCause, WalkVerdict};
pub use evidence::{BootStatus, EvidenceReader};
