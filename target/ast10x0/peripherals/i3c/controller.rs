// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! I3C Controller
//!
//! Main hardware abstraction for I3C bus controller.
//!
//! # Construction Patterns
//!
//! The controller uses an explicit two-stage bring-up:
//!
//! | Step | Purpose | Performance | Use Case |
//! |------|---------|-------------|----------|
//! | [`new()`](I3cController::new) / [`from_initialized()`](I3cController::from_initialized) | Construct controller value only | Fast (no I/O) | Build the owner that will be pinned |
//! | [`init_hardware()`](I3cController::init_hardware) | Register IRQ handler + program hardware | Slower (register writes) | First-time setup after the controller is pinned |
//!
//! # Example
//!
//! ```rust,ignore
//! // === BOOT/INIT CODE (runs once) ===
//! // Platform init first (clocks, resets - not part of i3c_core)
//! scu.enable_i3c_clock(bus);
//! scu.deassert_i3c_reset(bus);
//!
//! // Construct, pin, then initialize so the IRQ handler sees a stable address.
//! let mut ctrl = core::pin::pin!(I3cController::new(hw, config));
//! ctrl.as_mut().init_hardware();
//!
//! // === HOT PATH (hardware already configured) ===
//! let ctrl = I3cController::from_initialized(hw, config);
//! ctrl.do_transfer(...);
//! ```

use core::marker::PhantomPinned;
use core::pin::Pin;

use super::ccc;
use super::config::{DeviceEntry, I3cConfig, I3cTargetConfig};
use super::constants::I3C_BROADCAST_ADDR;
use super::error::I3cError;
use super::hardware::HardwareInterface;
use super::types::{DevKind, I3cIbi, I3cIbiType, I3cMsg};
use embedded_hal::i2c::SevenBitAddress;

/// I3C controller wrapping hardware interface
pub struct I3cController<H: HardwareInterface> {
    /// Hardware interface implementation
    hw: H,
    /// Bus configuration
    config: I3cConfig,
    _pin: PhantomPinned,
}

impl<H: HardwareInterface> I3cController<H> {
    // =========================================================================
    // Construction
    // =========================================================================

    /// Construct an I3C controller value without touching hardware.
    ///
    /// This does **not** register an IRQ handler or program registers. Call
    /// [`init_hardware`](Self::init_hardware) after pinning the controller to a
    /// stable address.
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
        Self {
            hw,
            config,
            _pin: PhantomPinned,
        }
    }

    /// Initialize/reinitialize hardware registers
    ///
    /// Registers the IRQ handler and configures the hardware.
    ///
    /// This method requires a pinned controller so the IRQ registry can keep a
    /// stable pointer to it. The target/kernel owns the top-level interrupt
    /// vector; its ISR should call [`dispatch_i3c_irq`](super::hardware::dispatch_i3c_irq).
    pub fn init_hardware(self: Pin<&mut Self>) {
        let this = unsafe { self.get_unchecked_mut() };
        let ctx = core::ptr::from_mut::<Self>(this) as usize;
        let bus = this.hw.bus_num() as usize;
        super::hardware::register_i3c_irq_handler(bus, Self::irq_trampoline, ctx);

        // IMPORTANT: init() must complete before enable_irq() to prevent
        // IRQ firing on partially-initialized hardware
        this.hw.init(&mut this.config);

        // Memory barrier to ensure init writes are visible before IRQ enable
        cortex_m::asm::dmb();

        this.hw.enable_irq();
    }

    /// IRQ trampoline function
    fn irq_trampoline(ctx: usize) {
        // SAFETY: `ctx` was created from `&mut Self` in `init_hardware()`.
        // Aliasing safety relies on caller not holding `&mut self` when IRQs enabled.
        let ctrl: &mut Self = unsafe { &mut *(ctx as *mut Self) };
        ctrl.hw.i3c_aspeed_isr(&mut ctrl.config);
    }

    #[inline]
    fn project_mut(self: Pin<&mut Self>) -> &mut Self {
        unsafe { self.get_unchecked_mut() }
    }

    #[inline]
    fn project_ref(self: Pin<&Self>) -> &Self {
        Pin::get_ref(self)
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
    pub fn attach_i3c_dev(
        self: Pin<&mut Self>,
        pid: u64,
        desired_da: u8,
        slot: u8,
    ) -> Result<(), I3cError> {
        let this = self.project_mut();
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

        let idx = this
            .config
            .attached
            .attach(dev)
            .map_err(|_| I3cError::AddrInUse)?;
        this.config
            .attached
            .map_pos(slot, u8::try_from(idx).map_err(|_| I3cError::InvalidArgs)?);
        this.config.addrbook.mark_use(desired_da, true);

        this.hw
            .attach_i3c_dev(slot.into(), desired_da)
            .map_err(|_| I3cError::AddrInUse)
    }

    /// Detach an I3C device by DAT position
    pub fn detach_i3c_dev(self: Pin<&mut Self>, pos: usize) {
        let this = self.project_mut();
        this.config.attached.detach_by_pos(pos);
        this.hw.detach_i3c_dev(pos);
    }

    /// Detach an I3C device by device index
    pub fn detach_i3c_dev_by_idx(self: Pin<&mut Self>, dev_idx: usize) {
        let this = self.project_mut();
        // `get` (not `[dev_idx]`) keeps this panic-free for the `no_panics`
        // analysis; an out-of-range index is simply a no-op.
        let Some(dev) = this.config.attached.devices.get(dev_idx) else {
            return;
        };

        if dev.dyn_addr != 0 {
            this.config.addrbook.mark_use(dev.dyn_addr, false);
        }

        let dev_pos = dev.pos;
        if let Some(pos) = dev_pos {
            this.hw.detach_i3c_dev(pos.into());
        }

        this.config.attached.detach(dev_idx);
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
    pub fn recover_bus(self: Pin<&mut Self>, scl_toggles: u32) {
        let this = self.project_mut();
        this.hw.enter_sw_mode();
        this.hw.i3c_toggle_scl_in(scl_toggles);
        this.hw.gen_internal_stop();
        this.hw.exit_sw_mode();
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
    pub fn recover_bus_full(mut self: Pin<&mut Self>, reset_mask: u32) {
        self.as_mut().recover_bus(8);
        self.project_mut().hw.reset_ctrl(reset_mask);
    }

    // Accessors
    // =========================================================================

    /// Return this controller's bus number.
    #[inline]
    pub fn bus_num(self: Pin<&Self>) -> u8 {
        self.project_ref().hw.bus_num()
    }

    /// Allocate a dynamic address from `start_addr`.
    #[inline]
    pub fn alloc_dynamic_address_from(self: Pin<&mut Self>, start_addr: u8) -> Option<u8> {
        self.project_mut().config.addrbook.alloc_from(start_addr)
    }

    /// Return the currently assigned target dynamic address, if any.
    #[inline]
    pub fn target_dynamic_address(self: Pin<&Self>) -> Option<u8> {
        self.project_ref()
            .config
            .target_config
            .as_ref()
            .and_then(|t| t.addr)
    }

    /// Set the device's IBI mandatory data byte and enable IBI delivery for `addr`.
    pub fn enable_ibi(self: Pin<&mut Self>, addr: u8, mdb: u8) -> Result<(), I3cError> {
        let this = self.project_mut();
        this.hw.set_ibi_mdb(mdb);
        this.hw.ibi_enable(&mut this.config, addr)
    }

    /// Issue a private read to `pid`, returning the number of received bytes.
    pub fn priv_read(self: Pin<&mut Self>, pid: u64, out: &mut [u8]) -> Result<u32, I3cError> {
        let this = self.project_mut();
        let actual_len = u32::try_from(out.len()).map_err(|_| I3cError::InvalidArgs)?;
        let mut msgs = [I3cMsg {
            buf: Some(out),
            actual_len,
            num_xfer: 0,
            flags: super::constants::I3C_MSG_READ | super::constants::I3C_MSG_STOP,
            hdr_mode: 0,
            hdr_cmd_mode: 0,
        }];
        this.hw.priv_xfer(&mut this.config, pid, &mut msgs)?;
        Ok(msgs[0].actual_len)
    }

    /// Issue a private write to `pid`.
    pub fn priv_write(self: Pin<&mut Self>, pid: u64, data: &mut [u8]) -> Result<(), I3cError> {
        let this = self.project_mut();
        let actual_len = u32::try_from(data.len()).map_err(|_| I3cError::InvalidArgs)?;
        let mut msgs = [I3cMsg {
            buf: Some(data),
            actual_len,
            num_xfer: 0,
            flags: super::constants::I3C_MSG_WRITE | super::constants::I3C_MSG_STOP,
            hdr_mode: 0,
            hdr_cmd_mode: 0,
        }];
        this.hw.priv_xfer(&mut this.config, pid, &mut msgs)
    }

    /// Raise a hot-join request from the target side.
    pub fn target_raise_hot_join(self: Pin<&mut Self>) -> Result<(), I3cError> {
        let this = self.project_mut();
        this.hw.target_ibi_raise_hj(&mut this.config)
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
        self: Pin<&mut Self>,
        static_address: SevenBitAddress,
    ) -> Result<SevenBitAddress, I3cError> {
        let this = self.project_mut();
        let slot = this
            .config
            .attached
            .pos_of_addr(static_address)
            .ok_or(I3cError::AddrInUse)?;

        this.hw
            .do_entdaa(&mut this.config, slot.into())
            .map_err(|_| I3cError::AddrInUse)?;

        let pid = ccc::ccc_getpid(&mut this.hw, &mut this.config, static_address)
            .map_err(|_| I3cError::Invalid)?;

        let dev_idx = this
            .config
            .attached
            .find_dev_idx_by_addr(static_address)
            .ok_or(I3cError::Other)?;

        let old_pid = this
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

        let bcr = ccc::ccc_getbcr(&mut this.hw, &mut this.config, static_address)
            .map_err(|_| I3cError::Invalid)?;

        {
            let dev = this
                .config
                .attached
                .devices
                .get_mut(dev_idx)
                .ok_or(I3cError::Other)?;

            dev.pid = Some(pid);
            dev.bcr = bcr;
        }

        let dyn_addr: SevenBitAddress = this
            .config
            .attached
            .devices
            .get(dev_idx)
            .ok_or(I3cError::Other)?
            .dyn_addr;

        this.hw
            .ibi_enable(&mut this.config, dyn_addr)
            .map_err(|_| I3cError::Other)?;

        Ok(dyn_addr)
    }

    /// Acknowledge an IBI from `address` (validates the device is known).
    pub fn acknowledge_ibi(self: Pin<&mut Self>, address: SevenBitAddress) -> Result<(), I3cError> {
        let this = self.project_mut();
        let dev_idx = this
            .config
            .attached
            .find_dev_idx_by_addr(address)
            .ok_or(I3cError::Other)?;

        // `get` (not `[dev_idx]`) keeps this panic-free for the `no_panics`
        // analysis; `find_dev_idx_by_addr` already returns a valid index.
        let dev = this
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
    pub fn handle_hot_join(self: Pin<&mut Self>) -> Result<(), I3cError> {
        Ok(())
    }

    /// Bus speed is fixed on the AST1060 controller; this is a no-op.
    #[allow(clippy::unused_self)]
    pub fn set_bus_speed(self: Pin<&mut Self>) -> Result<(), I3cError> {
        Ok(())
    }

    /// The AST1060 controller does not support multi-master; this is a no-op.
    #[allow(clippy::unused_self)]
    pub fn request_mastership(self: Pin<&mut Self>) -> Result<(), I3cError> {
        Ok(())
    }

    // --- Target (secondary) mode callbacks ---

    /// Initialize target mode with `own_addr` (sets the static/target address).
    pub fn target_init(self: Pin<&mut Self>, own_addr: u8) {
        let this = self.project_mut();
        if let Some(t) = this.config.target_config.as_mut() {
            if t.addr.is_none() {
                t.addr = Some(own_addr);
            }
        } else {
            this.config.target_config =
                Some(I3cTargetConfig::new(0, Some(own_addr), /* mdb */ 0xae));
        }
    }

    /// Returns `true` if `addr` matches this target's assigned address.
    #[must_use]
    pub fn target_on_address_match(self: Pin<&Self>, addr: u8) -> bool {
        self.project_ref()
            .config
            .target_config
            .as_ref()
            .and_then(|t| t.addr)
            == Some(addr)
    }

    /// Record that the controller assigned this target a dynamic address; SIRs
    /// are then permitted by software.
    pub fn target_on_dynamic_address_assigned(self: Pin<&mut Self>) {
        self.project_mut().config.sir_allowed_by_sw = true;
    }

    /// This target always wants to raise IBIs when it has data.
    #[must_use]
    #[allow(clippy::unused_self)]
    pub fn target_wants_ibi(self: Pin<&Self>) -> bool {
        true
    }

    /// Build and submit the IBI payload `[mdb, crc8_ccitt(addr_rnw, mdb)]` for a
    /// pending target read, returning the number of bytes made available.
    pub fn target_get_ibi_payload(
        self: Pin<&mut Self>,
        buffer: &mut [u8],
    ) -> Result<usize, I3cError> {
        let this = self.project_mut();
        let (da, mdb) = match this.config.target_config.as_ref() {
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
        let rc = this
            .hw
            .target_pending_read_notify(&mut this.config, buffer, &mut ibi);

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
