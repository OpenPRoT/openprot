// Licensed under the Apache-2.0 license

//! AST1060 I2C controller implementation

use core::cell::UnsafeCell;
use core::marker::PhantomData;

use super::timing::configure_timing;
use super::types::{I2cConfig, I2cXferMode};
use super::{constants, error::I2cError};
use ast1060_pac::{i2c::RegisterBlock, i2cbuff::RegisterBlock as BuffRegisterBlock};

/// Main I2C hardware abstraction.
///
/// Wraps a raw `*const RegisterBlock` and `*const BuffRegisterBlock` pair
/// for one AST1060 I2C controller. Mirrors the [`Usart`](super::super::uart::Usart)
/// pattern: the constructor is `unsafe`; ownership/coordination is the
/// caller's responsibility.
///
/// `Y` is the caller-supplied yield closure. [`wait_completion`] calls
/// it as `(yield_ns)(100_000)` between status polls so the runtime can
/// hand the CPU back to a scheduler instead of busy-looping. For a
/// pure busy-wait, pass `|_| core::hint::spin_loop()`.
#[allow(clippy::struct_excessive_bools)]
pub struct Ast1060I2c<'a, Y: FnMut(u32)> {
    regs: *const RegisterBlock,
    buff_regs: *const BuffRegisterBlock,

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
    /// DMA buffer for DMA mode (non-cached SRAM, caller-owned)
    pub(crate) dma_buf: Option<&'a mut [u8]>,
    /// Cooperative yield invoked between status polls in
    /// [`wait_completion`]. Argument is the suggested wait window in
    /// nanoseconds.
    pub(crate) yield_ns: Y,

    /// Makes `Ast1060I2c` `!Sync` so the raw register pointers can't be
    /// shared across threads without explicit synchronization.
    _not_sync: PhantomData<UnsafeCell<()>>,
}

impl<'a, Y: FnMut(u32)> Ast1060I2c<'a, Y> {
    /// Create a new I2C instance and initialize hardware.
    ///
    /// Performs full hardware init (controller reset, multi-master,
    /// timing, interrupts). Use [`from_initialized`] when the bus has
    /// already been brought up by application code or a previous
    /// `new()` call.
    ///
    /// # Safety
    ///
    /// - `regs` and `buff_regs` must be valid, non-null pointers to the
    ///   AST1060 I2C register block and its companion buffer block for
    ///   the **same** controller instance.
    /// - The pointed register blocks must remain valid for the lifetime
    ///   of this `Ast1060I2c`.
    /// - Caller must enforce global ownership/coordination so concurrent
    ///   mutable access does not occur through other code paths.
    pub unsafe fn new(
        regs: *const RegisterBlock,
        buff_regs: *const BuffRegisterBlock,
        config: &I2cConfig,
        yield_ns: Y,
    ) -> Result<Self, I2cError> {
        let mut i2c = unsafe { Self::from_initialized(regs, buff_regs, config, yield_ns) };
        i2c.init_hardware(&config)?;
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
    /// # Safety
    ///
    /// Same contract as [`new`]. Additionally, the caller must ensure
    /// hardware is already configured: I2C global registers (I2CG0C,
    /// I2CG10) are set (call [`super::global::init_i2c_global`] ONCE in
    /// the app before use); controller is enabled (I2CC00); timing is
    /// configured; pin mux is configured.
    #[must_use]
    pub unsafe fn from_initialized(
        regs: *const RegisterBlock,
        buff_regs: *const BuffRegisterBlock,
        config: &I2cConfig,
        yield_ns: Y,
    ) -> Self {
        Self {
            regs,
            buff_regs,
            xfer_mode: config.xfer_mode,
            multi_master: config.multi_master,
            smbus_alert: config.smbus_alert,
            bus_recover: true,
            current_addr: 0,
            current_len: 0,
            current_xfer_cnt: 0,
            completion: false,
            dma_buf: None,
            yield_ns,
            _not_sync: PhantomData,
        }
    }

    /// Create I2C instance with DMA mode support.
    ///
    /// Like [`new`] but also attaches a DMA buffer for use when
    /// `xfer_mode == I2cXferMode::DmaMode`. The buffer must reside in
    /// non-cached SRAM (e.g. `#[link_section = ".ram_nc"]`) so that the
    /// DMA engine and CPU see the same data without cache maintenance.
    ///
    /// # Safety
    ///
    /// Same contract as [`new`]. Additionally `dma_buf` must remain valid
    /// for the lifetime of this `Ast1060I2c` and must be in a memory
    /// region the DMA engine can address coherently with the CPU.
    pub unsafe fn new_with_dma(
        regs: *const RegisterBlock,
        buff_regs: *const BuffRegisterBlock,
        config: &I2cConfig,
        dma_buf: &'a mut [u8],
        yield_ns: Y,
    ) -> Result<Self, I2cError> {
        let mut i2c = unsafe { Self::from_initialized(regs, buff_regs, config, yield_ns) };
        i2c.dma_buf = Some(dma_buf);
        i2c.init_hardware(&config)?;
        Ok(i2c)
    }

    /// Create I2C instance from pre-initialized hardware with DMA buffer
    /// (NO hardware init).
    ///
    /// Like [`from_initialized`] but attaches a DMA buffer for use when
    /// `xfer_mode == I2cXferMode::DmaMode`. Use this per-operation after
    /// the bus has already been initialized via [`new_with_dma`], to
    /// avoid the overhead of re-running hardware initialization.
    ///
    /// The buffer must reside in non-cached SRAM (e.g. `#[link_section = ".ram_nc"]`).
    ///
    /// # Safety
    ///
    /// Same contract as [`from_initialized`] and [`new_with_dma`].
    #[must_use]
    pub unsafe fn from_initialized_with_dma(
        regs: *const RegisterBlock,
        buff_regs: *const BuffRegisterBlock,
        config: &I2cConfig,
        dma_buf: &'a mut [u8],
        yield_ns: Y,
    ) -> Self {
        let mut i2c = unsafe { Self::from_initialized(regs, buff_regs, config, yield_ns) };
        i2c.dma_buf = Some(dma_buf);
        i2c
    }

    /// Get access to registers (visible to other modules)
    #[inline]
    pub(crate) fn regs(&self) -> &RegisterBlock {
        // SAFETY: `Ast1060I2c` construction is `unsafe`, so the caller
        // upholds pointer validity, non-nullness, and ownership.
        unsafe { &*self.regs }
    }

    /// Get access to buffer registers (visible to other modules)
    #[inline]
    pub(crate) fn buff_regs(&self) -> &BuffRegisterBlock {
        // SAFETY: see `regs`.
        unsafe { &*self.buff_regs }
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

