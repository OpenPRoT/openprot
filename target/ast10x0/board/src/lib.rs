// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#![no_std]
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    clippy::todo,
    clippy::unimplemented
)]

use ast10x0_peripherals::hace::HaceDevice;
use ast10x0_peripherals::scu::{PinctrlPin, ScuRegisters};

pub mod board;
pub mod spi_monitor;
pub mod spim_wiring;

pub use board::Pins;
pub use spi_monitor::Ast1060SpiMonitor;
pub use spim_wiring::{
    apply_spim_external_mux, apply_spim_pinctrl, apply_spim_wiring, apply_spim_wiring_with_log,
    bmc_spim_csin_levels, bmc_spim_path_debug, enable_flash_power, presets,
    release_spi_flash_resets, set_bmc_resets, spim_external_mux_state, BmcSpimPathDebug,
    SpimWiring, SpimWiringError,
};

pub use ast10x0_peripherals::i2c::{I2cConfig, I2cError};

/// Board descriptor metadata for AST10x0 board initialization.
#[derive(Clone, Debug)]
pub struct Ast10x0BoardDescriptor {
    /// Pin control groups to apply during board init.
    /// Applied in order via `ScuRegisters::apply_pinctrl_group()`.
    pub pinctrl_groups: &'static [&'static [PinctrlPin]],
}

/// Runtime board object that executes hardware initialization steps.
pub struct Ast10x0Board {
    descriptor: Ast10x0BoardDescriptor,
}

impl Ast10x0Board {
    /// Create a board runtime object from board metadata.
    #[must_use]
    pub const fn new(descriptor: Ast10x0BoardDescriptor) -> Self {
        Self { descriptor }
    }

    /// Create a [`HaceDevice`] bound to the singleton HACE instance.
    ///
    /// This is the primary factory for HACE access on AST10x0. The board
    /// crate is the single point that wires the SCU cache-flush hook into
    /// the HACE driver, keeping `ast10x0_peripherals::hace` free of any
    /// direct SCU dependency at the operation level.
    ///
    /// # Safety
    /// - Must not be called concurrently with any other HACE access.
    /// - Only one [`HaceDevice`] should be live at a time.
    pub unsafe fn hace_device<Y: FnMut(u32)>(&self, yield_fn: Y) -> HaceDevice<Y> {
        // SAFETY: caller upholds the single-instance contract.
        unsafe { HaceDevice::new_global(yield_fn) }
    }

    /// Apply pinctrl groups and bring up the shared I2C subsystem (clock/reset/global); per-controller
    /// bring-up is each server's own via `i2c_backend::open_bus_dma`.
    ///
    /// # Errors
    /// Infallible today; keeps a `Result` for forward-compatible platform init.
    ///
    /// # Safety
    /// - Must be called only once during board initialization.
    /// - Not thread-safe; caller must ensure no concurrent SCU or I2C accesses.
    pub unsafe fn init(&self) -> Result<(), I2cError> {
        // Unlock SCU once before the sequence of writes (aspeed-rust pattern)
        let scu = unsafe { ScuRegisters::new_global_unlocked() };

        // Apply pinctrl groups
        for group in self.descriptor.pinctrl_groups {
            scu.apply_pinctrl_group(group);
        }

        // Bring up the shared I2C subsystem through its one home, reusing the SCU handle already
        // unlocked above — same sequence Pins::take runs, no second unlock.
        ast10x0_peripherals::i2c::bringup(scu, delay_us);

        Ok(())
    }
}

/// Simple busy-wait delay in microseconds.
///
/// This is a placeholder; production code should use a proper timer or delay provider.
/// Spins for approximately `micros` microseconds.
#[inline]
pub fn delay_us(micros: u32) {
    // Very rough approximation: ~16 cycles per microsecond on Cortex-M4 @ ~50MHz
    // This is calibration-free but inaccurate; improve for production.
    for _ in 0..micros.saturating_mul(16) {
        core::hint::spin_loop();
    }
}
