// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Flash device abstractions layered on top of SMC wrappers.

mod block_device;
mod flash;

pub use block_device::{BlockDeviceInfo, SpiNorBlockDevice};
pub use flash::{
    FlashAddressingPolicy, FlashCommandProfile, JedecId, SpiNorFlash, SpiNorFlashDevice,
};
