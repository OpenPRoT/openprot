// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! I3C Controller
//!
//! Main hardware abstraction for I3C bus controller.
//!
//! # Lifecycle
//!
//! Two states, matching the SMC peripheral's `Uninitialized -> Ready`
//! precedent:
//!
//! | State | Entered by | Available operations |
//! |-------|-----------|----------------------|
//! | [`Uninitialized`] | [`I3cController::new`] | [`start()`](I3cController::start) |
//! | [`Ready`] | `start()` (IRQ trampoline claimed + hardware programmed) | bus operations |
//!
//! After `start()` the integration layer unmasks the NVIC line it owns (it
//! selected the bus, so it knows the matching platform interrupt line);
//! the driver never touches the NVIC.
//!
//! # ISR decoupling
//!
//! The ISR shares **no** `&mut` state with this controller: at `start()` the
//! driver parks an ISR-owned register handle plus the role flag in the
//! per-bus registry (`hardware::register_i3c_irq_handler`), and the ISR
//! communicates back exclusively through per-bus atomics and the global IBI
//! work rings (the SMC flag-and-defer model). The controller is therefore a
//! plain owned value — no pinning, no `'static` storage, no raw context
//! pointer.
//!
//! # Example
//!
//! ```rust,ignore
//! // === BOOT/INIT CODE (runs once) ===
//! // Platform init first (clocks, resets - not part of i3c_core)
//! scu.enable_i3c_clock(bus);
//! scu.deassert_i3c_reset(bus);
//!
//! let hw = unsafe { Ast1060I3c::new(bus, yield_fn) }.ok_or(...)?;
//! let mut ctrl = I3cController::new(hw, &mut config)
//!     .start()?;            // register ISR ctx (single-shot) + program hardware
//!
//! // Integration layer owns the NVIC line; unmask it now.
//! unsafe { NVIC::unmask(integration_owned_irq_line) };
//!
//! ctrl.priv_write(pid, &mut data)?;
//! ```

use core::marker::PhantomData;

use super::ccc;
use super::config::{DeviceEntry, I3cConfig, I3cTargetConfig};
use super::constants::I3C_BROADCAST_ADDR;
use super::error::I3cError;
use super::hardware::HardwareInterface;
use super::types::{DevKind, I3cIbi, I3cIbiType, I3cMsg};
use embedded_hal::i2c::SevenBitAddress;

// =============================================================================
// Lifecycle states
// =============================================================================

/// Initial state: nothing registered, no I/O done.
pub struct Uninitialized;
/// IRQ trampoline claimed and hardware programmed; bus operations available.
/// The integration layer unmasks the NVIC line it owns after entering this
/// state.
pub struct Ready;

// =============================================================================
// Controller shell
// =============================================================================

/// I3C controller: a plain owned value over the hardware driver, borrowing
/// the caller's configuration. No pinning or `'static` storage is required —
/// the ISR never holds a pointer into this object (see the module docs).
///
/// The configuration is **borrowed** (`&'c mut I3cConfig`), not owned: the
/// config embeds the device tables (~0.5 KiB), and the typestate transition
/// (`start(self) -> Self<Ready>`) moves the controller by value — owning the
/// config would transiently stack two copies inside one frame, which the
/// 2 KiB kernel bootstrap stack cannot afford. Borrowing keeps exactly one
/// config alive, wherever the caller placed it.
pub struct I3cController<'c, H: HardwareInterface, S = Uninitialized> {
    hw: H,
    config: &'c mut I3cConfig,
    _state: PhantomData<S>,
}

impl<'c, H: HardwareInterface, S> I3cController<'c, H, S> {
    /// Split-borrow helper for operations that drive `hw` with `config`.
    #[inline]
    fn parts(&mut self) -> (&mut H, &mut I3cConfig) {
        (&mut self.hw, &mut *self.config)
    }

    /// Return this controller's bus number.
    #[inline]
    #[must_use]
    pub fn bus_num(&self) -> u8 {
        self.hw.bus_num()
    }
}

impl<'c, H: HardwareInterface> I3cController<'c, H, Uninitialized> {
    /// Bundle hardware and a borrowed configuration. No I/O, no registration.
    #[must_use]
    pub fn new(hw: H, config: &'c mut I3cConfig) -> Self {
        Self {
            hw,
            config,
            _state: PhantomData,
        }
    }

    /// Bring the controller up: park this bus's ISR context in the registry
    /// (single-shot per bus) and program the hardware.
    ///
    /// The target/kernel owns the top-level interrupt vector; its ISR calls
    /// [`dispatch_i3c_irq`](super::hardware::dispatch_i3c_irq), which services
    /// the bus through the registered context. On return the device may
    /// assert its IRQ line; nothing is delivered until the integration layer
    /// unmasks the NVIC line it owns.
    ///
    /// Returns [`I3cError::Busy`] if the bus's IRQ slot was already claimed by
    /// another controller, or [`I3cError::Timeout`] if the hardware's initial
    /// queue-reset poll timed out.
    pub fn start(mut self) -> Result<I3cController<'c, H, Ready>, I3cError> {
        let bus = self.hw.bus_num() as usize;
        let ctx = self.hw.isr_ctx(self.config.is_secondary);
        if !super::hardware::register_i3c_irq_handler(bus, ctx) {
            return Err(I3cError::Busy);
        }

        if let Err(e) = self.hw.init(self.config) {
            // Release the just-claimed slot, or every retry of `start()`
            // would fail with `Busy` against a controller that never came up.
            super::hardware::unregister_i3c_irq_handler(bus);
            return Err(e);
        }
        // Memory barrier so init writes are visible before the integration
        // layer unmasks the IRQ line.
        cortex_m::asm::dmb();

        Ok(I3cController {
            hw: self.hw,
            config: self.config,
            _state: PhantomData,
        })
    }
}

// =============================================================================
// Bus operations (Ready)
// =============================================================================

impl<'c, H: HardwareInterface> I3cController<'c, H, Ready> {
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
        let (hw, config) = self.parts();
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

        let idx = config
            .attached
            .attach(dev)
            .map_err(|_| I3cError::AddrInUse)?;
        config
            .attached
            .map_pos(slot, u8::try_from(idx).map_err(|_| I3cError::InvalidArgs)?);
        config.addrbook.mark_use(desired_da, true);

        hw.attach_i3c_dev(slot.into(), desired_da)
            .map_err(|_| I3cError::AddrInUse)
    }

    /// Detach an I3C device by DAT position
    pub fn detach_i3c_dev(&mut self, pos: usize) {
        let (hw, config) = self.parts();
        config.attached.detach_by_pos(pos);
        hw.detach_i3c_dev(pos);
    }

    /// Detach an I3C device by device index
    pub fn detach_i3c_dev_by_idx(&mut self, dev_idx: usize) {
        let (hw, config) = self.parts();
        // `get` (not `[dev_idx]`) keeps this panic-free for the `no_panics`
        // analysis; an out-of-range index is simply a no-op.
        let Some(dev) = config.attached.devices.get(dev_idx) else {
            return;
        };

        if dev.dyn_addr != 0 {
            let dyn_addr = dev.dyn_addr;
            config.addrbook.mark_use(dyn_addr, false);
        }

        let dev_pos = config.attached.devices.get(dev_idx).and_then(|dev| dev.pos);
        if let Some(pos) = dev_pos {
            hw.detach_i3c_dev(pos.into());
        }

        config.attached.detach(dev_idx);
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
        let (hw, _) = self.parts();
        hw.enter_sw_mode();
        hw.i3c_toggle_scl_in(scl_toggles);
        hw.gen_internal_stop();
        hw.exit_sw_mode();
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
    /// ctrl.recover_bus_full(reset)?;
    /// ```
    ///
    /// # Errors
    ///
    /// [`I3cError::Timeout`] if the controller reset bits did not self-clear —
    /// the engine is wedged beyond what software recovery can fix.
    pub fn recover_bus_full(&mut self, reset_mask: u32) -> Result<(), I3cError> {
        self.recover_bus(8);
        let (hw, _) = self.parts();
        hw.reset_ctrl(reset_mask)
    }

    // =========================================================================
    // Accessors
    // =========================================================================

    /// Allocate a dynamic address from `start_addr`.
    #[inline]
    pub fn alloc_dynamic_address_from(&mut self, start_addr: u8) -> Option<u8> {
        let (_, config) = self.parts();
        config.addrbook.alloc_from(start_addr)
    }

    /// Return the currently assigned target dynamic address, if any.
    ///
    /// The address is assigned by the bus master and latched by the ISR into
    /// the per-bus event block; the locally configured address (if any) is
    /// the fallback.
    #[inline]
    #[must_use]
    pub fn target_dynamic_address(&self) -> Option<u8> {
        super::hardware::isr_events(self.hw.bus_num() as usize)
            .dyn_addr()
            .or_else(|| self.config.target_config.as_ref().and_then(|t| t.addr))
    }

    /// Set the device's IBI mandatory data byte and enable IBI delivery for `addr`.
    pub fn enable_ibi(&mut self, addr: u8, mdb: u8) -> Result<(), I3cError> {
        let (hw, config) = self.parts();
        hw.set_ibi_mdb(mdb);
        hw.ibi_enable(config, addr)
    }

    /// Issue a private read to `pid`, returning the number of received bytes.
    pub fn priv_read(&mut self, pid: u64, out: &mut [u8]) -> Result<u32, I3cError> {
        let (hw, config) = self.parts();
        let actual_len = u32::try_from(out.len()).map_err(|_| I3cError::InvalidArgs)?;
        let mut msgs = [I3cMsg {
            buf: Some(out),
            actual_len,
            num_xfer: 0,
            flags: super::constants::I3C_MSG_READ | super::constants::I3C_MSG_STOP,
            hdr_mode: 0,
            hdr_cmd_mode: 0,
        }];
        hw.priv_xfer(config, pid, &mut msgs)?;
        Ok(msgs[0].actual_len)
    }

    /// Issue a private write to `pid`.
    pub fn priv_write(&mut self, pid: u64, data: &mut [u8]) -> Result<(), I3cError> {
        let (hw, config) = self.parts();
        let actual_len = u32::try_from(data.len()).map_err(|_| I3cError::InvalidArgs)?;
        let mut msgs = [I3cMsg {
            buf: Some(data),
            actual_len,
            num_xfer: 0,
            flags: super::constants::I3C_MSG_WRITE | super::constants::I3C_MSG_STOP,
            hdr_mode: 0,
            hdr_cmd_mode: 0,
        }];
        hw.priv_xfer(config, pid, &mut msgs)
    }

    /// Raise a hot-join request from the target side.
    pub fn target_raise_hot_join(&mut self) -> Result<(), I3cError> {
        let (hw, config) = self.parts();
        hw.target_ibi_raise_hj(config)
    }

    // =========================================================================
    // Master / Target operations  (Delta D1)
    // =========================================================================
    //
    // The reference exposed these through `proposed_traits::i3c_master::I3c`
    // and the `proposed_traits` target traits (`aspeed-rust/src/i3c/hal_impl.rs`).
    // That crate is unavailable in openprot and embedded-hal 1.0 defines no I3C
    // trait, so — as the I2C port did for `proposed_traits::i2c_target` — the
    // logic is preserved verbatim here as **inherent methods**. The only change
    // is that `ErrorKind`-mapped errors become direct `I3cError` variants
    // (`DynamicAddressConflict` -> `AddrInUse`, `InvalidCcc` -> `Invalid`).

    /// Assign a dynamic address to the device at `static_address` via ENTDAA,
    /// then read back PID/BCR and enable IBI. Returns the assigned address.
    pub fn assign_dynamic_address(
        &mut self,
        static_address: SevenBitAddress,
    ) -> Result<SevenBitAddress, I3cError> {
        let (hw, config) = self.parts();
        let slot = config
            .attached
            .pos_of_addr(static_address)
            .ok_or(I3cError::AddrInUse)?;

        hw.do_entdaa(config, slot.into())
            .map_err(|_| I3cError::AddrInUse)?;

        let pid = ccc::ccc_getpid(hw, config, static_address).map_err(|_| I3cError::Invalid)?;

        let dev_idx = config
            .attached
            .find_dev_idx_by_addr(static_address)
            .ok_or(I3cError::Other)?;

        let old_pid = config
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

        let bcr = ccc::ccc_getbcr(hw, config, static_address).map_err(|_| I3cError::Invalid)?;

        {
            let dev = config
                .attached
                .devices
                .get_mut(dev_idx)
                .ok_or(I3cError::Other)?;

            dev.pid = Some(pid);
            dev.bcr = bcr;
        }

        let dyn_addr: SevenBitAddress = config
            .attached
            .devices
            .get(dev_idx)
            .ok_or(I3cError::Other)?
            .dyn_addr;

        hw.ibi_enable(config, dyn_addr)
            .map_err(|_| I3cError::Other)?;

        Ok(dyn_addr)
    }

    /// Acknowledge an IBI from `address` (validates the device is known).
    pub fn acknowledge_ibi(&mut self, address: SevenBitAddress) -> Result<(), I3cError> {
        let (_, config) = self.parts();
        let dev_idx = config
            .attached
            .find_dev_idx_by_addr(address)
            .ok_or(I3cError::Other)?;

        // `get` (not `[dev_idx]`) keeps this panic-free for the `no_panics`
        // analysis; `find_dev_idx_by_addr` already returns a valid index.
        let dev = config
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
        let (_, config) = self.parts();
        if let Some(t) = config.target_config.as_mut() {
            if t.addr.is_none() {
                t.addr = Some(own_addr);
            }
        } else {
            config.target_config =
                Some(I3cTargetConfig::new(0, Some(own_addr), /* mdb */ 0xae));
        }
    }

    /// Returns `true` if `addr` matches this target's assigned address.
    #[must_use]
    pub fn target_on_address_match(&self, addr: u8) -> bool {
        self.target_dynamic_address() == Some(addr)
    }

    /// Record that the controller assigned this target a dynamic address; SIRs
    /// are then permitted by software. Also syncs the ISR-latched address into
    /// the thread-owned target config.
    pub fn target_on_dynamic_address_assigned(&mut self) {
        let da = super::hardware::isr_events(self.hw.bus_num() as usize).dyn_addr();
        if let (Some(da), Some(tc)) = (da, self.config.target_config.as_mut()) {
            tc.addr = Some(da);
        }
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
        let (hw, config) = self.parts();
        let (da, mdb) = match config.target_config.as_ref() {
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
        let rc = hw.target_pending_read_notify(config, buffer, &mut ibi);

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
