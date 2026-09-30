// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Schema for the per-board device table. Board device tables
//! (`target/<board>/devices.rs`) declare the values; no concrete line or
//! device is named here.
//!
//! Invariants are enforced in the `const fn` constructors, so an invalid
//! table is a build error and there is no validate step to forget. Checks
//! on board-defined types belong next to the table that gives them
//! meaning (`services/orchestrator/test/devices.rs` shows the pattern).

#![cfg_attr(not(test), no_std)]

pub mod checkpoint;
pub mod device;
pub mod layout;
pub mod record;

#[doc(inline)]
pub use checkpoint::BootCheckpoint;
#[doc(inline)]
pub use device::{assert_retry_reaches_every_image, DeviceConfig};
#[doc(inline)]
pub use layout::{Golden, ImageLayout, Region, Slot, SlotId};
#[doc(inline)]
pub use record::{RecordId, RecordLayout, RecordRegion};
