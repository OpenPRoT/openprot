// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Boot Orchestrator core: orchestration logic over the capability seams in
//! `fwmanager-api`.
//!
//! This crate is sans-IO: time is injected as monotonic milliseconds and
//! boot evidence arrives through the [`fwmanager_api::BootMonitor`] seam, so
//! every decision it makes is host-testable. The kernel-side pump (real
//! clock, sleeping between polls) lives with the runtime, not here.

#![cfg_attr(not(test), no_std)]

mod boot_progress;

pub use boot_progress::{BootFailure, BootWalk, MonitorMap, Progress};
