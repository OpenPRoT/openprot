// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Confined-`unsafe` MMIO façade for the AST1060 I2C controller: two macro-generated `Mmio`
//! appliers (I2C + I2CBUFF). Drivers call the generic ops with offset constants; no hand-written `unsafe`.

use super::{Ast1060I2c, I2cConfig, I2cError};
use crate::Mmio;
use openprot_hal::i2c::{i2c_mmios, I2cBlock, I2cBuffBlock, I2cController, I2cScl, I2cSda};
use openprot_hal::resource::Routes;

fn spin(_ns: u32) {
    core::hint::spin_loop();
}

/// The controller-open seam lives with the driver: bring up the registers (pins are already muxed by
/// the bus constructor), then hand back the ready driver. Open-once is the owning `I2cBus`'s `self`.
impl I2cController for Ast1060I2cRegisters {
    type Config = I2cConfig;
    type Ready = Ast1060I2c<'static, fn(u32)>;
    type Error = I2cError;

    fn open(self, config: &I2cConfig) -> Result<Self::Ready, I2cError> {
        Ast1060I2c::new(self, config, spin as fn(u32))
    }
}

/// Safe façade over one controller's `(I2C, I2CBUFF)` register pair — two confined MMIO appliers.
/// Both `Mmio`s already impl `RegBlock`, so drivers touch them directly (`.i2c`/`.buff`), no forwarders.
///
/// `Copy`, deliberately: the two appliers are stateless base pointers, and exclusivity of the
/// controller is enforced upstream by the bus owning both (non-`Copy`, mint-once) pin tokens. The
/// per-op driver in the backend re-derives a transient view from this façade under `&mut self`.
#[derive(Copy, Clone)]
pub struct Ast1060I2cRegisters {
    pub(crate) i2c: Mmio<I2cBlock>,
    pub(crate) buff: Mmio<I2cBuffBlock>,
}

impl Ast1060I2cRegisters {
    /// Fabricate the façade from the bus's two pin tokens; the HAL helper const-asserts one controller and mints its two bases.
    #[must_use]
    pub const fn from_pins<Scl: Routes<I2cScl>, Sda: Routes<I2cSda>>(scl: &Scl, sda: &Sda) -> Self {
        let (i2c, buff) = i2c_mmios(scl, sda);
        Self { i2c, buff }
    }
}
