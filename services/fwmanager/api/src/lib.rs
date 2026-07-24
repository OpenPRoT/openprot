// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Device-facing capability traits for the Boot Orchestrator.
//!
//! `BootControl` is the actuation capability: the orchestrator drives a
//! single managed device's reset without knowing which controller line it
//! maps to.
//!
//! `BootMonitor` is the observation capability: the orchestrator reads a
//! device's boot liveness.
//!
//! This crate is a dependency-free leaf: it holds only the capability
//! contracts, and everything depends downward on it. Concrete adapters bind a
//! trait to a signal source and live in their own crates, so naming a
//! capability never drags in the stack behind it — the HAL-backed
//! `HalBootControl` and `GpioBootMonitor` are in `fwmanager-hal-adapters`;
//! other backends (for example an MCTP-ready `BootMonitor`) implement the
//! same traits from their own transport crate.

#![cfg_attr(not(test), no_std)]

mod boot_control;
mod boot_monitor;

pub use boot_control::BootControl;
pub use boot_monitor::{BootMonitor, BootStatus};
