// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Flash device abstractions layered on top of SMC wrappers.

mod flash;
mod block_device;

pub use flash::{FlashAddressingPolicy, FlashCommandProfile, SpiNorFlashDevice, JedecId, SpiNorFlash};
pub use block_device::{BlockDeviceInfo, SpiNorBlockDevice};
