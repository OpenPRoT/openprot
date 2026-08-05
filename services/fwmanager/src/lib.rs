// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Boot Orchestrator core: orchestration logic over the capability seams in
//! `fwmanager-api`. Sans-IO — time is injected as monotonic milliseconds and
//! evidence arrives through [`fwmanager_api::BootMonitor`] — so every
//! decision is host-testable; the kernel-side pump lives with the runtime.

#![cfg_attr(not(test), no_std)]

mod boot_progress;

pub use boot_progress::{BootFailure, BootWalk, MonitorMap, Progress};
