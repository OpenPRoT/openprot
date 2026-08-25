// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! HAL-backed adapters for the Boot Orchestrator capability traits.
//!
//! Each type here binds an orchestrator-facing seam to a HAL-blocking trait:
//! [`HalBootControl`] drives `BootControl` over a `ResetControl` line, and
//! [`GpioBootMonitor`] reads a `GpioPort` input line into a `BootStatus`.
//! Adapters live in this crate — not in the leaf
//! `orchestrator-capabilities` — so that depending on a capability contract
//! never pulls in the HAL. A transport-backed adapter belongs in its own crate
//! depending on its own stack, by the same rule.

#![cfg_attr(not(test), no_std)]

mod gpio_boot_monitor;
mod hal_boot_control;

pub use gpio_boot_monitor::{GpioBootMonitor, GpioCause, MonitorError};
pub use hal_boot_control::{BootError, HalBootControl};
