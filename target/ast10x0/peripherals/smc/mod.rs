// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Static Memory Controller (SMC) HAL
//!
//! Provides safe abstractions over the FMC (Firmware Memory Controller)
//! and SPI1/SPI2 flash controllers.

pub mod controller;
pub mod device;
pub mod fmc;
mod helpers;
pub mod interrupts;
pub mod registers;
pub mod spi;
pub mod types;

pub use controller::{Ready, ReadySmc, Smc, UninitSmc, Uninitialized};
pub use device::{
    BlockDeviceInfo, FlashAddressingPolicy, FlashCommandProfile, JedecId, SpiNorBlockDevice,
    SpiNorFlash, SpiNorFlashDevice,
};
pub use fmc::{FmcReady, FmcUninit};
pub use interrupts::{SmcInterrupt, SmcInterruptDecoder};
pub use spi::{SpiReady, SpiUninit};
pub use types::{
    AddressWidth, ChipSelect, FlashConfig, SmcConfig, SmcController, SmcError, SmcRetryable,
    SmcTopology, TransferMode,
};

/// Result type for SMC operations
pub type Result<T> = core::result::Result<T, SmcError>;
