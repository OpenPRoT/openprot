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

use ast10x0_peripherals::scu::{ClockRegisterHalf, ScuRegisterHalf};
use ast10x0_peripherals::scu::{PinctrlPin, ScuRegisters};

pub mod spi_monitor;
pub mod spim_wiring;

pub use spi_monitor::Ast1060SpiMonitor;
pub use spim_wiring::{apply_spim_wiring, presets, SpimWiring, SpimWiringError};

pub use ast10x0_peripherals::i2c::{I2cConfig, I2cError};

/// Per-bus I2C bring-up entry: which controller, and the config its
/// `init_hardware()` (timing/master/interrupts) is run with.
#[derive(Clone, Debug)]
pub struct I2cBusCfg {
    /// Controller index (`0..=i2c_backend::MAX_BUS`).
    pub bus: u8,
    /// Per-controller configuration applied by `init_bus`.
    pub config: I2cConfig,
}

/// Board descriptor metadata for AST10x0 board initialization.
#[derive(Clone, Debug)]
pub struct Ast10x0BoardDescriptor {
    /// Pin control groups to apply during board init.
    /// Applied in order via `ScuRegisters::apply_pinctrl_group()`.
    pub pinctrl_groups: &'static [&'static [PinctrlPin]],
    /// I2C controllers this board wires, with their per-bus config. The board
    /// brings every one up eagerly during [`Ast10x0Board::init`]. Empty for
    /// boards/tests that manage I2C themselves.
    pub i2c_buses: &'static [I2cBusCfg],
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

    /// Initialize board: apply pinctrl groups and initialize I2C subsystem.
    ///
    /// This performs the complete platform-level initialization:
    /// 1. Apply pinctrl groups
    /// 2. Enable I2C clock via SCU
    /// 3. Assert I2C/SMBus controller reset
    /// 4. Delay for reset to settle
    /// 5. Deassert reset
    /// 6. Delay for recovery
    /// 7. Configure I2C global registers (clock dividers, etc.)
    /// 8. Per-controller bring-up (`init_bus`) for every wired bus in
    ///    `descriptor.i2c_buses`.
    ///
    /// # Errors
    /// Returns the first per-controller init failure. Steps 1-7 are infallible.
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

        // Enable I2C clock (Group 0, bit 2)
        scu.ungate_clock_mask(ClockRegisterHalf::Lower, 1 << 2);

        // Assert I2C reset (Upper half, bit 2)
        scu.assert_reset_mask(ScuRegisterHalf::Upper, 1 << 2);
        delay_us(1000);

        // Deassert I2C reset
        scu.deassert_reset_mask(ScuRegisterHalf::Upper, 1 << 2);
        delay_us(1000);

        // Configure I2C global registers (clock dividers, etc.)
        unsafe { ast10x0_peripherals::i2c::init_i2c_global() };

        // Eagerly bring up every wired controller. The server later re-wraps
        // these with no re-init via `open_bus` / `open_bus_dma`.
        for b in self.descriptor.i2c_buses {
            // SAFETY: subsystem init (clock/reset/global/pin-mux) is complete;
            // board init is single-threaded and owns every controller here.
            unsafe { i2c_backend::init_bus(b.bus, &b.config)? };
        }

        Ok(())
    }
}

/// Simple busy-wait delay in microseconds.
///
/// This is a placeholder; production code should use a proper timer or delay provider.
/// Spins for approximately `micros` microseconds.
#[inline]
fn delay_us(micros: u32) {
    // Very rough approximation: ~16 cycles per microsecond on Cortex-M4 @ ~50MHz
    // This is calibration-free but inaccurate; improve for production.
    for _ in 0..(micros * 16) {
        core::hint::spin_loop();
    }
}
