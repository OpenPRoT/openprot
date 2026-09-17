// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Concrete [`BootWatch`] that walks a device's boot checkpoints in order,
//! polling an [`EvidenceReader`] for each one and judging the per-checkpoint
//! windows against the caller-injected `now_millis`.
//!
//! Generic over the reader (`R`) and probe vocabulary (`P`), so the same
//! walker serves GPIO-backed boards, register-backed SoCs, and test
//! doubles. Board wiring constructs one per component and hands them to
//! the platform driver as `Board::boot_watches`.

#![cfg_attr(not(test), no_std)]

mod walk;

pub use walk::CheckpointWalk;
