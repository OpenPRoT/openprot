// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Orchestrator server: the in-process runtime that drives the pure
//! [`openprot_orchestrator_sm`] state machine.
//!
//! Orchestrator-sm names timeouts as [`Event`](openprot_orchestrator_sm::Event)s
//! but owns no clock. [`TimerManager`] lives here, in the same process, and
//! multiplexes orchestrator-sm's boot and commit watchdogs onto the single
//! deadline the runtime's `object_wait` already accepts — no separate timer
//! task, no IPC on the arm/cancel path.

#![no_std]
#![forbid(unsafe_code)]

pub mod runtime;

pub use openprot_orchestrator_timer::{Full, TimerManager};
pub use runtime::BootWatchdogs;
