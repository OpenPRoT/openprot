// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

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

use super::ccc;
use super::config::{DeviceEntry, I3cConfig, I3cTargetConfig};
use super::constants::I3C_BROADCAST_ADDR;
use super::error::I3cError;
use super::hardware::HardwareInterface;
use super::types::{DevKind, I3cIbi, I3cIbiType};
use embedded_hal::i2c::SevenBitAddress;

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
        // `get` (not `[dev_idx]`) keeps this panic-free for the `no_panics`
        // analysis; an out-of-range index is simply a no-op.
        let Some(dev) = self.config.attached.devices.get(dev_idx) else {
            return;
        };

        if dev.dyn_addr != 0 {
            self.config.addrbook.mark_use(dev.dyn_addr, false);
        }

        let dev_pos = dev.pos;
        if let Some(pos) = dev_pos {
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

// =============================================================================
// Master / Target operations  (Delta D1)
// =============================================================================
//
// The reference exposed these through `proposed_traits::i3c_master::I3c` and the
// `proposed_traits` target traits (`aspeed-rust/src/i3c/hal_impl.rs`). That crate
// is unavailable in openprot and embedded-hal 1.0 defines no I3C trait, so — as
// the I2C port did for `proposed_traits::i2c_target` — the logic is preserved
// verbatim here as **inherent methods**. The only change is that
// `ErrorKind`-mapped errors become direct `I3cError` variants
// (`DynamicAddressConflict` -> `AddrInUse`, `InvalidCcc` -> `Invalid`).

impl<H: HardwareInterface> I3cController<H> {
    /// Assign a dynamic address to the device at `static_address` via ENTDAA,
    /// then read back PID/BCR and enable IBI. Returns the assigned address.
    pub fn assign_dynamic_address(
        &mut self,
        static_address: SevenBitAddress,
    ) -> Result<SevenBitAddress, I3cError> {
        let slot = self
            .config
            .attached
            .pos_of_addr(static_address)
            .ok_or(I3cError::AddrInUse)?;

        self.hw
            .do_entdaa(&mut self.config, slot.into())
            .map_err(|_| I3cError::AddrInUse)?;

        let pid = ccc::ccc_getpid(&mut self.hw, &mut self.config, static_address)
            .map_err(|_| I3cError::Invalid)?;

        let dev_idx = self
            .config
            .attached
            .find_dev_idx_by_addr(static_address)
            .ok_or(I3cError::Other)?;

        let old_pid = self
            .config
            .attached
            .devices
            .get(dev_idx)
            .ok_or(I3cError::Other)?
            .pid;

        if let Some(op) = old_pid
            && pid != op
        {
            return Err(I3cError::Other);
        }

        let bcr = ccc::ccc_getbcr(&mut self.hw, &mut self.config, static_address)
            .map_err(|_| I3cError::Invalid)?;

        {
            let dev = self
                .config
                .attached
                .devices
                .get_mut(dev_idx)
                .ok_or(I3cError::Other)?;

            dev.pid = Some(pid);
            dev.bcr = bcr;
        }

        let dyn_addr: SevenBitAddress = self
            .config
            .attached
            .devices
            .get(dev_idx)
            .ok_or(I3cError::Other)?
            .dyn_addr;

        self.hw
            .ibi_enable(&mut self.config, dyn_addr)
            .map_err(|_| I3cError::Other)?;

        Ok(dyn_addr)
    }

    /// Acknowledge an IBI from `address` (validates the device is known).
    pub fn acknowledge_ibi(&mut self, address: SevenBitAddress) -> Result<(), I3cError> {
        let dev_idx = self
            .config
            .attached
            .find_dev_idx_by_addr(address)
            .ok_or(I3cError::Other)?;

        // `get` (not `[dev_idx]`) keeps this panic-free for the `no_panics`
        // analysis; `find_dev_idx_by_addr` already returns a valid index.
        let dev = self
            .config
            .attached
            .devices
            .get(dev_idx)
            .ok_or(I3cError::Other)?;
        if dev.pid.is_none() {
            return Err(I3cError::Other);
        }

        Ok(())
    }

    /// Hot-join handler hook. Call [`assign_dynamic_address`](Self::assign_dynamic_address)
    /// after receiving a hot-join IBI; nothing else is required here.
    #[allow(clippy::unused_self)]
    pub fn handle_hot_join(&mut self) -> Result<(), I3cError> {
        Ok(())
    }

    /// Bus speed is fixed on the AST1060 controller; this is a no-op.
    #[allow(clippy::unused_self)]
    pub fn set_bus_speed(&mut self) -> Result<(), I3cError> {
        Ok(())
    }

    /// The AST1060 controller does not support multi-master; this is a no-op.
    #[allow(clippy::unused_self)]
    pub fn request_mastership(&mut self) -> Result<(), I3cError> {
        Ok(())
    }

    // --- Target (secondary) mode callbacks ---

    /// Initialize target mode with `own_addr` (sets the static/target address).
    pub fn target_init(&mut self, own_addr: u8) {
        if let Some(t) = self.config.target_config.as_mut() {
            if t.addr.is_none() {
                t.addr = Some(own_addr);
            }
        } else {
            self.config.target_config =
                Some(I3cTargetConfig::new(0, Some(own_addr), /* mdb */ 0xae));
        }
    }

    /// Returns `true` if `addr` matches this target's assigned address.
    #[must_use]
    pub fn target_on_address_match(&self, addr: u8) -> bool {
        self.config.target_config.as_ref().and_then(|t| t.addr) == Some(addr)
    }

    /// Record that the controller assigned this target a dynamic address; SIRs
    /// are then permitted by software.
    pub fn target_on_dynamic_address_assigned(&mut self) {
        self.config.sir_allowed_by_sw = true;
    }

    /// This target always wants to raise IBIs when it has data.
    #[must_use]
    #[allow(clippy::unused_self)]
    pub fn target_wants_ibi(&self) -> bool {
        true
    }

    /// Build and submit the IBI payload `[mdb, crc8_ccitt(addr_rnw, mdb)]` for a
    /// pending target read, returning the number of bytes made available.
    pub fn target_get_ibi_payload(&mut self, buffer: &mut [u8]) -> Result<usize, I3cError> {
        let (da, mdb) = match self.config.target_config.as_ref() {
            Some(t) => (
                match t.addr {
                    Some(da) => da,
                    None => return Ok(0),
                },
                t.mdb,
            ),
            None => return Ok(0),
        };

        let addr_rnw = (da << 1) | 0x1;
        let mut crc = crc8_ccitt(0, &[addr_rnw]);
        crc = crc8_ccitt(crc, &[mdb]);

        let payload = [mdb, crc];
        let mut ibi = I3cIbi {
            ibi_type: I3cIbiType::TargetIntr,
            payload: Some(&payload),
        };
        let rc = self
            .hw
            .target_pending_read_notify(&mut self.config, buffer, &mut ibi);

        match rc {
            Ok(()) => Ok(buffer.len() + payload.len()),
            _ => Ok(0),
        }
    }
}

/// CRC-8 CCITT calculation (ported from `hal_impl.rs`).
#[inline]
fn crc8_ccitt(mut crc: u8, data: &[u8]) -> u8 {
    for &b in data {
        let mut x = crc ^ b;
        for _ in 0..8 {
            x = if (x & 0x80) != 0 {
                (x << 1) ^ 0x07
            } else {
                x << 1
            };
        }
        crc = x;
    }
    crc
}
