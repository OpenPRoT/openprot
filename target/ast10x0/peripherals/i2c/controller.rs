// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! AST1060 I2C controller implementation

use super::registers::Ast1060I2cRegisters;
use super::timing::configure_timing;
use super::types::{I2cConfig, I2cXferMode};
use super::{constants, error::I2cError};
use ast1060_pac::{i2c::RegisterBlock, i2cbuff::RegisterBlock as BuffRegisterBlock};

/// Main I2C hardware abstraction.
///
/// Holds the [`Ast1060I2cRegisters`] MMIO façade for one controller; **all**
/// `unsafe` register access is confined to that façade (the *Confined-`unsafe`
/// MMIO Façade* pattern — see `registers.rs`). This type is the driver/state
/// layer above it and is itself `unsafe`-free for construction.
///
/// `Y` is the caller-supplied yield closure. [`wait_completion`] calls
/// it as `(yield_ns)(100_000)` between status polls so the runtime can
/// hand the CPU back to a scheduler instead of busy-looping. For a
/// pure busy-wait, pass `|_| core::hint::spin_loop()`.
#[allow(clippy::struct_excessive_bools)]
pub struct Ast1060I2c<'a, Y: FnMut(u32)> {
    /// MMIO façade — the sole site of register-pointer `unsafe`. Not `Sync`
    /// (and not `Send`) by construction, so `Ast1060I2c` isn't either.
    mmio: Ast1060I2cRegisters,

    /// Transfer mode (visible to other modules for optimization decisions)
    pub(crate) xfer_mode: I2cXferMode,
    multi_master: bool,
    smbus_alert: bool,
    #[allow(dead_code)]
    bus_recover: bool,

    // Transfer state (visible to transfer/master modules)
    /// Current device address being communicated with
    pub(crate) current_addr: u8,
    /// Current transfer length
    pub(crate) current_len: u32,
    /// Bytes transferred so far
    pub(crate) current_xfer_cnt: u32,
    /// Completion flag for synchronous operations
    pub(crate) completion: bool,
    /// Master DMA staging buffer (non-cached SRAM, caller-owned)
    pub(crate) master_dma_buf: Option<&'a mut [u8]>,
    /// Slave DMA buffer for slave RX (non-cached SRAM, caller-owned)
    pub(crate) slave_dma_buf: Option<&'a mut [u8]>,
    /// Cooperative yield invoked between status polls in
    /// [`wait_completion`]. Argument is the suggested wait window in
    /// nanoseconds.
    pub(crate) yield_ns: Y,
}

impl<'a, Y: FnMut(u32)> Ast1060I2c<'a, Y> {
    /// Create a new I2C instance and initialize hardware.
    ///
    /// Performs full hardware init (controller reset, multi-master,
    /// timing, interrupts). Use [`from_initialized`] when the bus has
    /// already been brought up by application code or a previous
    /// `new()` call.
    ///
    /// Safe: the `unsafe` register-pointer perimeter was discharged once at
    /// [`Ast1060I2cRegisters::new`]; this only consumes the resulting façade.
    pub fn new(
        mmio: Ast1060I2cRegisters,
        config: &I2cConfig,
        yield_ns: Y,
    ) -> Result<Self, I2cError> {
        let mut i2c = Self::from_initialized(mmio, config, yield_ns);
        i2c.init_hardware(config)?;
        Ok(i2c)
    }

    /// Create I2C instance from pre-initialized hardware (NO hardware init).
    ///
    /// Lightweight constructor that wraps register pointers without
    /// writing to hardware. Use when:
    /// - Hardware was already initialized by app `main.rs` before kernel start
    /// - Hardware was initialized by a previous `new()` call
    /// - You want to avoid redundant re-initialization overhead
    ///
    /// Precondition (caller-ensured, not a safety obligation of this fn):
    /// hardware is already configured — I2C global registers (I2CG0C,
    /// I2CG10) set (call [`super::global::init_i2c_global`] ONCE in the app
    /// before use); controller enabled (I2CC00); timing configured; pin mux
    /// configured.
    #[must_use]
    pub fn from_initialized(mmio: Ast1060I2cRegisters, config: &I2cConfig, yield_ns: Y) -> Self {
        Self {
            mmio,
            xfer_mode: config.xfer_mode,
            multi_master: config.multi_master,
            smbus_alert: config.smbus_alert,
            bus_recover: true,
            current_addr: 0,
            current_len: 0,
            current_xfer_cnt: 0,
            completion: false,
            master_dma_buf: None,
            slave_dma_buf: None,
            yield_ns,
        }
    }

    /// Create I2C instance with DMA mode support.
    ///
    /// Like [`new`] but also attaches separate master and slave DMA buffers.
    /// Both buffers must reside in non-cached SRAM (e.g. `#[link_section = ".ram_nc"]`)
    /// so that the DMA engine and CPU see the same data without cache maintenance.
    ///
    /// - `master_dma_buf`: staging buffer for master TX/RX DMA (up to 4096 B)
    /// - `slave_dma_buf`: receive buffer for slave RX DMA (256 B recommended)
    pub fn new_with_dma(
        mmio: Ast1060I2cRegisters,
        config: &I2cConfig,
        master_dma_buf: &'a mut [u8],
        slave_dma_buf: &'a mut [u8],
        yield_ns: Y,
    ) -> Result<Self, I2cError> {
        let mut i2c = Self::from_initialized(mmio, config, yield_ns);
        i2c.master_dma_buf = Some(master_dma_buf);
        i2c.slave_dma_buf = Some(slave_dma_buf);
        i2c.init_hardware(config)?;
        Ok(i2c)
    }

    /// Create I2C instance from pre-initialized hardware with DMA buffers
    /// (NO hardware init).
    ///
    /// Like [`from_initialized`] but attaches separate master and slave DMA
    /// buffers. Use when the bus was already initialized via [`new_with_dma`]
    /// or `init_bus` and you want to avoid redundant hardware init.
    ///
    /// Both buffers must reside in non-cached SRAM and remain valid for this
    /// `Ast1060I2c`'s lifetime.
    #[must_use]
    pub fn from_initialized_with_dma(
        mmio: Ast1060I2cRegisters,
        config: &I2cConfig,
        master_dma_buf: &'a mut [u8],
        slave_dma_buf: &'a mut [u8],
        yield_ns: Y,
    ) -> Self {
        let mut i2c = Self::from_initialized(mmio, config, yield_ns);
        i2c.master_dma_buf = Some(master_dma_buf);
        i2c.slave_dma_buf = Some(slave_dma_buf);
        i2c
    }

    /// I2C register block, via the MMIO façade (sole `unsafe` deref is inside
    /// [`Ast1060I2cRegisters`]). Driver-internal use.
    #[inline]
    pub(crate) fn regs(&self) -> &RegisterBlock {
        self.mmio.i2c()
    }

    /// I2CBUFF register block, via the MMIO façade. Driver-internal use.
    #[inline]
    pub(crate) fn buff_regs(&self) -> &BuffRegisterBlock {
        self.mmio.buff()
    }

    /// Initialize hardware
    #[inline(never)]
    pub fn init_hardware(&mut self, config: &I2cConfig) -> Result<(), I2cError> {
        // PRECONDITION: I2C global registers must be initialized by app before use.
        // See: super::global::init_i2c_global().

        // Reset I2C controller
        unsafe {
            self.regs().i2cc00().write(|w| w.bits(0));
        }

        // Configure multi-master
        if !self.multi_master {
            self.regs()
                .i2cc00()
                .modify(|_, w| w.dis_multimaster_capability_for_master_fn_only().set_bit());
        }

        // Enable master function and bus auto-release
        self.regs().i2cc00().modify(|_, w| {
            w.enbl_bus_autorelease_when_scllow_sdalow_or_slave_mode_inactive_timeout()
                .set_bit()
                .enbl_master_fn()
                .set_bit()
        });

        // Configure timing
        configure_timing(self.regs(), config)?;

        // Clear all interrupts
        unsafe {
            self.regs().i2cm14().write(|w| w.bits(0xffff_ffff));
        }

        // Enable interrupts for packet mode
        self.regs().i2cm10().modify(|_, w| {
            w.enbl_pkt_cmd_done_int()
                .set_bit()
                .enbl_bus_recover_done_int()
                .set_bit()
        });

        if self.smbus_alert {
            self.regs()
                .i2cm10()
                .modify(|_, w| w.enbl_smbus_dev_alert_int().set_bit());
        }

        Ok(())
    }

    /// Enable interrupts
    pub fn enable_interrupts(&mut self, mask: u32) {
        unsafe {
            self.regs().i2cm10().write(|w| w.bits(mask));
        }
    }

    /// Clear interrupts
    pub fn clear_interrupts(&mut self, mask: u32) {
        unsafe {
            self.regs().i2cm14().write(|w| w.bits(mask));
        }
    }

    /// Check if bus is busy
    ///
    /// Checks if any I2C transfer is currently active by examining status register bits.
    #[must_use]
    pub fn is_bus_busy(&self) -> bool {
        let status = self.regs().i2cm14().read().bits();
        // Bus is busy if any transfer command bits are set
        (status
            & (constants::AST_I2CM_TX_CMD
                | constants::AST_I2CM_RX_CMD
                | constants::AST_I2CM_START_CMD))
            != 0
    }

    /// Wait for completion with timeout (visible to master/transfer modules).
    /// Calls the caller-supplied `yield_ns` closure between status polls.
    pub(crate) fn wait_completion(&mut self, timeout_us: u32) -> Result<(), I2cError> {
        let mut timeout = timeout_us;
        self.completion = false;

        while timeout > 0 && !self.completion {
            self.handle_interrupt()?;
            timeout = timeout.saturating_sub(1);
            (self.yield_ns)(100_000);
        }

        if self.completion {
            Ok(())
        } else {
            Err(I2cError::Timeout)
        }
    }
}
