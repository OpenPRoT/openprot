// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! HAL-backed adapters for the Boot Orchestrator capability traits.
//!
//! Each type here implements a capability trait from `fwmanager-api` against
//! a HAL-blocking trait: [`HalBootControl`] drives `BootControl` over a
//! `ResetControl` line. Adapters live in this crate — not in the leaf
//! `fwmanager-api` — so that depending on a capability contract never pulls
//! in the HAL. A transport-backed adapter belongs in its own crate depending
//! on its own stack, by the same rule.

#![cfg_attr(not(test), no_std)]

mod hal_boot_control;

pub use hal_boot_control::{BootError, HalBootControl};
