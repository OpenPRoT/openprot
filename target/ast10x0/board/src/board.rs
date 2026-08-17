// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! The board manifest: bring up the SoC once, then wire AST1060 pins to capabilities.
//! [`Pins::take`] is the sole `unsafe` gate; every `open` downstream is safe.

use ast10x0_peripherals::scu::{create_pins, route, ScuRegisters};
use i2c_backend::{i2c_bus, I2c1Bus, I2c2Bus};

/// The board's claimable hardware: each I2C controller wired to (and its pins muxed for) its bus.
pub struct Pins {
    /// I2C controller 1 — SCL2/SDA2 on SCU414[30:31].
    pub i2c1: I2c1Bus,
    /// I2C controller 2 — SCL3/SDA3 on SCU418[0:1].
    pub i2c2: I2c2Bus,
}

impl Pins {
    /// Bring up the SoC, bind the board's I2C buses from their pin tokens, and route them in one folded SCU pass; the one runtime `unsafe` gate.
    ///
    /// # Safety
    /// Call once, at boot, with exclusive SoC access.
    #[must_use]
    pub unsafe fn take() -> Self {
        // SAFETY: sole boot gate — single-threaded, exclusive SoC access, so the one SCU unlock and
        // the bring-up it feeds happen exactly once.
        let scu = unsafe { ScuRegisters::new_global_unlocked() };
        ast10x0_peripherals::i2c::bringup(scu, crate::delay_us);
        // SAFETY: sole pin creation site — created once here at boot; the pins! table is this chip's true pin map.
        let pins = unsafe { create_pins() };
        let i2c1 = i2c_bus(pins.scu414_30, pins.scu414_31);
        let i2c2 = i2c_bus(pins.scu418_0, pins.scu418_1);
        route(&(&i2c1, &i2c2));
        Pins { i2c1, i2c2 }
    }
}
