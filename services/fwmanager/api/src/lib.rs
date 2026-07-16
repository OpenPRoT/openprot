// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Device-facing capability traits for the Boot Orchestrator.
//!
//! `BootControl` is the actuation capability: the orchestrator drives a
//! single managed device's reset without knowing which controller line it
//! maps to. The binding of a HAL reset controller line to a device happens
//! once, in platform configuration, via [`HalBootControl`].

#![cfg_attr(not(test), no_std)]

mod boot_control;

pub use boot_control::{BootControl, HalBootControl};
