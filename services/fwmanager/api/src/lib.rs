// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Device-facing capability traits for the Boot Orchestrator.
//!
//! `BootControl` is the actuation capability: the orchestrator drives a
//! single managed device's reset without knowing which controller line it
//! maps to.
//!
//! This crate is a dependency-free leaf: it holds only the capability
//! contracts, and everything depends downward on it. Concrete adapters bind a
//! trait to a signal source and live in their own crates, so naming a
//! capability never drags in the stack behind it — the HAL-backed
//! `HalBootControl` is in `fwmanager-hal-adapters`; other backends implement
//! the same trait from their own transport crate.

#![cfg_attr(not(test), no_std)]

mod boot_control;

pub use boot_control::BootControl;
