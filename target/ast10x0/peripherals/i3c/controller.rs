// Licensed under the Apache-2.0 license

//! I3C Controller
//!
//! Main hardware abstraction for I3C bus controller.
//!
//! # Construction Patterns
//!
//! Two construction paths are provided:
//!
//! | Constructor | Purpose | Performance | Use Case |
//! |-------------|---------|-------------|----------|
//! | [`new()`](I3cController::new) | Full hardware init | Slower (register writes) | First-time setup, reset |
//! | [`from_initialized()`](I3cController::from_initialized) | Wrap pre-configured HW | Fast (no I/O) | Per-operation, hot path |
//!
//! # Example
//!
//! ```rust,ignore
//! // === BOOT/INIT CODE (runs once) ===
//! // Platform init first (clocks, resets - not part of i3c_core)
//! scu.enable_i3c_clock(bus);
//! scu.deassert_i3c_reset(bus);
//!
//! // Full hardware init
//! let mut ctrl = I3cController::new(hw, config)?;
//!
//! // === HOT PATH (hardware already configured) ===
//! let ctrl = I3cController::from_initialized(hw, config);
//! ctrl.do_transfer(...);
//! ```

use super::config::{DeviceEntry, I3cConfig};
use super::constants::I3C_BROADCAST_ADDR;
use super::error::I3cError;
use super::hardware::HardwareInterface;
use super::types::DevKind;

/// I3C controller wrapping hardware interface
pub struct I3cController<H: HardwareInterface> {
    /// Hardware interface implementation
    pub hw: H,
    /// Bus configuration
    pub config: I3cConfig,
}

impl<H: HardwareInterface> I3cController<H> {
    // =========================================================================
    // Construction
    // =========================================================================

    /// Create and initialize I3C controller (full init)
    ///
    /// Performs complete hardware initialization:
    /// - Registers IRQ handler
    /// - Enables interrupts
    /// - Initializes hardware registers
    ///
    /// Use [`from_initialized`](Self::from_initialized) if hardware is already
    /// configured.
    ///
    /// # Preconditions
    ///
    /// Platform initialization must be done before calling this:
    /// - Clocks enabled (via SCU)
    /// - Reset deasserted (via SCU)
    /// - Pin mux configured
    ///
    /// # Returns
    ///
    /// Initialized controller ready for use.
    pub fn new(hw: H, config: I3cConfig) -> Self {
        Self::from_initialized(hw, config)
    }

    /// Wrap pre-initialized hardware (lightweight, no I/O)
    ///
    /// Creates instance without touching hardware registers.
    ///
    /// # When to Use
    ///
    /// - Hardware was initialized at boot before kernel/RTOS start
    /// - Creating temporary instances for single operations
    /// - Avoiding redundant re-initialization overhead
    /// - Hot path where performance matters
    ///
    /// # Preconditions
    ///
    /// Caller must ensure hardware is already configured:
    /// - [`new()`](Self::new) was called previously, OR
    /// - Hardware initialized by bootloader/firmware
    ///
    /// # Performance
    ///
    /// No register writes - significantly faster than `new()`.
    #[must_use]
    pub fn from_initialized(hw: H, config: I3cConfig) -> Self {
        Self { hw, config }
    }

    /// Initialize/reinitialize hardware registers
    ///
    /// Registers the IRQ handler and configures the hardware.
    /// Called automatically by [`new()`](Self::new), but can be called
    /// explicitly to reinitialize after error recovery.
    ///
    /// # Safety Invariant
    ///
    /// After calling this method, the caller must ensure that no `&mut self`
    /// methods are called while interrupts are enabled, as the IRQ handler
    /// also takes `&mut self`. Violation causes undefined behavior.
    pub fn init_hardware(&mut self) {
        let ctx = core::ptr::from_mut::<Self>(self) as usize;
        let bus = self.hw.bus_num() as usize;
        super::hardware::register_i3c_irq_handler(bus, Self::irq_trampoline, ctx);

        // IMPORTANT: init() must complete before enable_irq() to prevent
        // IRQ firing on partially-initialized hardware
        self.hw.init(&mut self.config);

        // Memory barrier to ensure init writes are visible before IRQ enable
        cortex_m::asm::dmb();

        self.hw.enable_irq();
    }

    /// IRQ trampoline function
    fn irq_trampoline(ctx: usize) {
        // SAFETY: `ctx` was created from `&mut Self` in `init_hardware()`.
        // Aliasing safety relies on caller not holding `&mut self` when IRQs enabled.
        let ctrl: &mut Self = unsafe { &mut *(ctx as *mut Self) };
        ctrl.hw.i3c_aspeed_isr(&mut ctrl.config);
    }

    // =========================================================================
    // Device Management
    // =========================================================================

    /// Attach an I3C device to the bus
    ///
    /// # Arguments
    /// * `pid` - Provisional ID of the device
    /// * `desired_da` - Desired dynamic address
    /// * `slot` - DAT slot to use
    pub fn attach_i3c_dev(&mut self, pid: u64, desired_da: u8, slot: u8) -> Result<(), I3cError> {
        if desired_da == 0 || desired_da >= I3C_BROADCAST_ADDR {
            return Err(I3cError::InvalidArgs);
        }

        let dev = DeviceEntry {
            kind: DevKind::I3c,
            pid: Some(pid),
            static_addr: 0,
            dyn_addr: desired_da,
            desired_da,
            bcr: 0,
            dcr: 0,
            maxrd: 0,
            maxwr: 0,
            mrl: 0,
            mwl: 0,
            max_ibi: 0,
            ibi_en: false,
            pos: Some(slot),
        };

        let idx = self
            .config
            .attached
            .attach(dev)
            .map_err(|_| I3cError::AddrInUse)?;
        self.config
            .attached
            .map_pos(slot, u8::try_from(idx).map_err(|_| I3cError::InvalidArgs)?);
        self.config.addrbook.mark_use(desired_da, true);

        self.hw
            .attach_i3c_dev(slot.into(), desired_da)
            .map_err(|_| I3cError::AddrInUse)
    }

    /// Detach an I3C device by DAT position
    pub fn detach_i3c_dev(&mut self, pos: usize) {
        self.config.attached.detach_by_pos(pos);
        self.hw.detach_i3c_dev(pos);
    }

    /// Detach an I3C device by device index
    pub fn detach_i3c_dev_by_idx(&mut self, dev_idx: usize) {
        let dev = &self.config.attached.devices[dev_idx];

        if dev.dyn_addr != 0 {
            self.config.addrbook.mark_use(dev.dyn_addr, false);
        }

        if let Some(pos) = dev.pos {
            self.hw.detach_i3c_dev(pos.into());
        }

        self.config.attached.detach(dev_idx);
    }

    // =========================================================================
    // Bus Recovery
    // =========================================================================

    /// Recover the I3C bus from a stuck state
    ///
    /// Performs bus recovery sequence:
    /// 1. Enter software (bit-bang) mode
    /// 2. Toggle SCL to clear stuck slaves
    /// 3. Generate STOP condition
    /// 4. Exit software mode
    ///
    /// # Arguments
    /// * `scl_toggles` - Number of SCL toggles (typically 9 to clear a stuck byte)
    ///
    /// # When to Use
    ///
    /// - Bus appears hung (transfers timing out)
    /// - Device not responding after partial transfer
    /// - After detecting SDA stuck low
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// // Standard recovery with 9 SCL clocks
    /// ctrl.recover_bus(9);
    ///
    /// // More aggressive recovery
    /// ctrl.recover_bus(18);
    /// ```
    pub fn recover_bus(&mut self, scl_toggles: u32) {
        self.hw.enter_sw_mode();
        self.hw.i3c_toggle_scl_in(scl_toggles);
        self.hw.gen_internal_stop();
        self.hw.exit_sw_mode();
    }

    /// Perform full bus recovery with controller reset
    ///
    /// More aggressive recovery that also resets controller FIFOs:
    /// 1. Bus recovery (SCL toggle + STOP)
    /// 2. Reset TX/RX FIFOs
    /// 3. Reset command queue
    ///
    /// # Arguments
    /// * `reset_mask` - Controller components to reset (use `RESET_CTRL_*` constants)
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use aspeed_rust::i3c_core::{RESET_CTRL_RX_FIFO, RESET_CTRL_TX_FIFO, RESET_CTRL_CMD_QUEUE};
    ///
    /// // Full recovery with FIFO reset
    /// let reset = RESET_CTRL_RX_FIFO | RESET_CTRL_TX_FIFO | RESET_CTRL_CMD_QUEUE;
    /// ctrl.recover_bus_full(reset);
    /// ```
    pub fn recover_bus_full(&mut self, reset_mask: u32) {
        self.recover_bus(8);
        self.hw.reset_ctrl(reset_mask);
    }

    // Accessors
    // =========================================================================

    /// Get a reference to the hardware interface
    #[inline]
    pub fn hw(&self) -> &H {
        &self.hw
    }

    /// Get a mutable reference to the hardware interface
    #[inline]
    pub fn hw_mut(&mut self) -> &mut H {
        &mut self.hw
    }

    /// Get a reference to the configuration
    #[inline]
    pub fn config(&self) -> &I3cConfig {
        &self.config
    }

    /// Get a mutable reference to the configuration
    #[inline]
    pub fn config_mut(&mut self) -> &mut I3cConfig {
        &mut self.config
    }
}

// =============================================================================
// Conversions
// =============================================================================

impl<H: HardwareInterface> From<(H, I3cConfig)> for I3cController<H> {
    /// Lightweight conversion (no hardware I/O)
    ///
    /// Equivalent to [`from_initialized`](I3cController::from_initialized).
    fn from((hw, config): (H, I3cConfig)) -> Self {
        Self::from_initialized(hw, config)
    }
}
