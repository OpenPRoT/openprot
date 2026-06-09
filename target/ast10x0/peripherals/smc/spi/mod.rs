// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! SPI1/SPI2 wrapper and transaction guard.

mod spi;
mod spi_transaction;

pub use spi::{SpiReady, SpiUninit};
pub use spi_transaction::SpiTransaction;
