// Licensed under the Apache-2.0 license

//! AST1060 I2C Backend for the OpenPRoT I2C Server
//!
//! Thin adapter wrapping [`aspeed_ddk::i2c_core`] to provide I2C operations for
//! the server dispatch loop.
//!
//! # Two-Layer Init Model
//!
//! I2C initialization is split across two trust boundaries:
//!
//! | Layer | What | Where | Why |
//! |-------|------|-------|-----|
//! | **Platform** | SCU reset, I2CG0C/I2CG10, pinmux | `entry.rs` (kernel boot) | Shared SCU registers — single-threaded, no contention |
//! | **Per-bus** | I2CC00, timing, interrupts | `AspeedI2cBackend::init_bus()` (server task) | Per-controller MMIO, owned by this server |
//!
//! # Architecture
//!
//! ```text
//! Kernel entry.rs (boot, single-threaded):
//!   init_i2c_global()       ← SCU reset + I2CG0C/I2CG10
//!   apply_pinctrl_group()   ← SCU4xx pin mux (all buses)
//!
//! Server task (per process):
//!   AspeedI2cBackend::new()           ← steal PAC peripherals
//!   backend.init_bus(0)?              ← Ast1060I2c::new() → I2CC00, timing, IRQs
//!
//!   per operation:
//!     controller_regs(bus)
//!     Ast1060I2c::from_initialized()  ← zero register writes (~50x faster)
//!     i2c.write() / read() / ...
//! ```
//!
//! # Why Not `new()` Per Operation?
//!
//! Hubris called `Ast1060I2c::new()` on every operation, re-initializing
//! controller registers each time. We call `new()` once in `init_bus()`,
//! then use `from_initialized()` per-operation for zero-overhead access.

#![no_std]

use aspeed_ddk::i2c_core::{Ast1060I2c, Controller as DdkController, I2cConfig, I2cError};
use i2c_api::ResponseCode;

use pw_log;

// ---------------------------------------------------------------------------
// Error mapping
// ---------------------------------------------------------------------------

/// Map aspeed-ddk [`I2cError`] to wire protocol [`ResponseCode`].
///
/// This is the single point where hardware errors become IPC response codes.
/// The mapping is intentionally conservative — ambiguous hardware errors map
/// to `IoError` rather than more specific codes.
fn map_i2c_error(e: I2cError) -> ResponseCode {
    match e {
        I2cError::NoAcknowledge => ResponseCode::NoDevice,
        I2cError::ArbitrationLoss => ResponseCode::ArbitrationLost,
        I2cError::Timeout => ResponseCode::Timeout,
        I2cError::Busy => ResponseCode::Busy,
        I2cError::InvalidAddress => ResponseCode::InvalidAddress,
        I2cError::BusRecoveryFailed => ResponseCode::BusStuck,
        I2cError::Invalid => ResponseCode::ServerError,
        // Hardware errors without a specific wire code → IoError
        I2cError::Bus | I2cError::Overrun | I2cError::Abnormal | I2cError::SlaveError => {
            ResponseCode::IoError
        }
    }
}

// ---------------------------------------------------------------------------
// Backend
// ---------------------------------------------------------------------------

/// AST1060 I2C backend owning all peripheral register blocks.
///
/// Constructed once at server startup via [`AspeedI2cBackend::new()`],
/// then each bus must be initialized via [`init_bus()`] before use.
///
/// Each operation creates a transient [`Ast1060I2c`] handle via
/// `from_initialized()` on the stack — no heap, no persistent driver state.
///
/// # Safety Model
///
/// The backend exclusively owns `ast1060_pac::Peripherals`, ensuring no
/// aliased register access. The `unsafe` is confined to [`new()`] where
/// `Peripherals::steal()` is called.
pub struct AspeedI2cBackend {
    peripherals: ast1060_pac::Peripherals,
    /// Tracks which buses have been initialized via `init_bus()`.
    initialized: u16,
}

impl AspeedI2cBackend {
    /// Create the backend by stealing PAC peripherals.
    ///
    /// # Safety
    ///
    /// Must only be called once. Caller must ensure exclusive access to I2C
    /// peripherals for the lifetime of this backend.
    ///
    /// After construction, call [`init_bus()`] for each bus this server owns.
    pub unsafe fn new() -> Self {
        Self {
            peripherals: unsafe { ast1060_pac::Peripherals::steal() },
            initialized: 0,
        }
    }

    /// Initialize a single I2C bus controller.
    ///
    /// Performs per-controller hardware setup: I2CC00 reset, master enable,
    /// timing configuration, and interrupt enable. Must be called once per bus
    /// before any operations on that bus.
    ///
    /// # Preconditions
    ///
    /// Platform init (`entry.rs`) must have already run:
    /// - `init_i2c_global()` — SCU reset, I2CG0C, I2CG10
    /// - `apply_pinctrl_group()` — SCU pin mux for this bus
    ///
    /// # Errors
    ///
    /// Returns `ResponseCode::InvalidBus` if `bus > 13`, or
    /// `ResponseCode::IoError` if hardware init fails.
    pub fn init_bus(&mut self, bus: u8) -> Result<(), ResponseCode> {
        let (regs, buffs) = self.controller_regs(bus)?;
        let ctrl = aspeed_ddk::i2c_core::I2cController {
            controller: DdkController(bus),
            registers: regs,
            buff_registers: buffs,
        };
        // Ast1060I2c::new() calls init_hardware() which configures:
        //   - I2CC00: reset, master enable, bus auto-release
        //   - Timing registers per I2cConfig
        //   - I2CM10/I2CM14: interrupt enable/clear
        let _i2c = Ast1060I2c::new(&ctrl, I2cConfig::default())
            .map_err(map_i2c_error)?;
        self.initialized |= 1 << bus;
        pw_log::info!("I2C bus {} controller initialized", bus as u8);
        Ok(())
    }

    /// Check whether a bus has been initialized.
    #[inline]
    fn is_bus_initialized(&self, bus: u8) -> bool {
        bus < 14 && (self.initialized & (1 << bus)) != 0
    }

    /// Map bus index (0–13) to PAC register block references.
    ///
    /// The AST1060 has 14 I2C controllers. Bus indices beyond 13 return
    /// `ResponseCode::InvalidBus`.
    fn controller_regs(
        &self,
        bus: u8,
    ) -> Result<
        (
            &ast1060_pac::i2c::RegisterBlock,
            &ast1060_pac::i2cbuff::RegisterBlock,
        ),
        ResponseCode,
    > {
        match bus {
            0 => Ok((&self.peripherals.i2c, &self.peripherals.i2cbuff)),
            1 => Ok((&self.peripherals.i2c1, &self.peripherals.i2cbuff1)),
            2 => Ok((&self.peripherals.i2c2, &self.peripherals.i2cbuff2)),
            3 => Ok((&self.peripherals.i2c3, &self.peripherals.i2cbuff3)),
            4 => Ok((&self.peripherals.i2c4, &self.peripherals.i2cbuff4)),
            5 => Ok((&self.peripherals.i2c5, &self.peripherals.i2cbuff5)),
            6 => Ok((&self.peripherals.i2c6, &self.peripherals.i2cbuff6)),
            7 => Ok((&self.peripherals.i2c7, &self.peripherals.i2cbuff7)),
            8 => Ok((&self.peripherals.i2c8, &self.peripherals.i2cbuff8)),
            9 => Ok((&self.peripherals.i2c9, &self.peripherals.i2cbuff9)),
            10 => Ok((&self.peripherals.i2c10, &self.peripherals.i2cbuff10)),
            11 => Ok((&self.peripherals.i2c11, &self.peripherals.i2cbuff11)),
            12 => Ok((&self.peripherals.i2c12, &self.peripherals.i2cbuff12)),
            13 => Ok((&self.peripherals.i2c13, &self.peripherals.i2cbuff13)),
            _ => Err(ResponseCode::InvalidBus),
        }
    }

    // -----------------------------------------------------------------------
    // Master operations
    // -----------------------------------------------------------------------

    /// Write data to an I2C device.
    ///
    /// Creates a transient `Ast1060I2c` via `from_initialized()` for the
    /// specified bus, performs the write, and drops the handle.
    ///
    /// # Errors
    ///
    /// Returns `ResponseCode::ServerError` if the bus was not initialized
    /// via [`init_bus()`].
    pub fn write(&mut self, bus: u8, addr: u8, data: &[u8]) -> Result<(), ResponseCode> {
        if !self.is_bus_initialized(bus) {
            return Err(ResponseCode::ServerError);
        }
        let (regs, buffs) = self.controller_regs(bus)?;
        let ctrl = aspeed_ddk::i2c_core::I2cController {
            controller: DdkController(bus),
            registers: regs,
            buff_registers: buffs,
        };
        let mut i2c = Ast1060I2c::from_initialized(&ctrl, I2cConfig::default());
        i2c.write(addr, data).map_err(map_i2c_error)
    }

    /// Read data from an I2C device.
    pub fn read(&mut self, bus: u8, addr: u8, buf: &mut [u8]) -> Result<(), ResponseCode> {
        if !self.is_bus_initialized(bus) {
            return Err(ResponseCode::ServerError);
        }
        let (regs, buffs) = self.controller_regs(bus)?;
        let ctrl = aspeed_ddk::i2c_core::I2cController {
            controller: DdkController(bus),
            registers: regs,
            buff_registers: buffs,
        };
        let mut i2c = Ast1060I2c::from_initialized(&ctrl, I2cConfig::default());
        i2c.read(addr, buf).map_err(map_i2c_error)
    }

    /// Write-then-read (combined transaction with repeated START).
    pub fn write_read(
        &mut self,
        bus: u8,
        addr: u8,
        wr: &[u8],
        rd: &mut [u8],
    ) -> Result<(), ResponseCode> {
        if !self.is_bus_initialized(bus) {
            return Err(ResponseCode::ServerError);
        }
        let (regs, buffs) = self.controller_regs(bus)?;
        let ctrl = aspeed_ddk::i2c_core::I2cController {
            controller: DdkController(bus),
            registers: regs,
            buff_registers: buffs,
        };
        let mut i2c = Ast1060I2c::from_initialized(&ctrl, I2cConfig::default());
        i2c.write_read(addr, wr, rd).map_err(map_i2c_error)
    }

    /// Attempt bus recovery (clock stretching / stuck SDA).
    pub fn recover_bus(&mut self, bus: u8) -> Result<(), ResponseCode> {
        if !self.is_bus_initialized(bus) {
            return Err(ResponseCode::ServerError);
        }
        let (regs, buffs) = self.controller_regs(bus)?;
        let ctrl = aspeed_ddk::i2c_core::I2cController {
            controller: DdkController(bus),
            registers: regs,
            buff_registers: buffs,
        };
        let mut i2c = Ast1060I2c::from_initialized(&ctrl, I2cConfig::default());
        i2c.recover_bus().map_err(map_i2c_error)
    }
}
