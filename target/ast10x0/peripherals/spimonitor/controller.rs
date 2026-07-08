// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! SPI monitor controller facade.

use core::marker::PhantomData;

use crate::scu::registers::ScuRegisters;
use crate::scu::types::{ScuExtMuxSelect, SpiMonitorInstance};
use crate::spimonitor::addr_priv;
use crate::spimonitor::cmd_table;
use crate::spimonitor::policy::{MonitorPolicy, MAX_REGION_SLOTS};
use crate::spimonitor::registers::{SpiMonitorController, SpiMonitorRegisters};
use crate::spimonitor::types::{
    ExtMuxSel, LockState, MonitorState, PassthroughMode, Result,
    SpiMonitorError, ViolationLogEntry,
};

/// Typestate: monitor is created but policy is not yet applied.
pub struct Uninitialized;
/// Typestate: policy tables are programmed and can still be changed.
pub struct Configured;
/// Typestate: policy is locked and runtime-mutating APIs are unavailable.
pub struct Locked;

/// Generic SPI monitor instance with typestate-enforced lifecycle.
pub struct SpiMonitor<Mode> {
    regs: SpiMonitorRegisters,
    controller: SpiMonitorController,
    scu: ScuRegisters,
    _mode: PhantomData<fn() -> Mode>,
}

/// Ergonomic alias for an uninitialized SPI monitor handle.
pub type UninitSpiMonitor = SpiMonitor<Uninitialized>;
/// Ergonomic alias for a configured-but-unlocked SPI monitor handle.
pub type ConfiguredSpiMonitor = SpiMonitor<Configured>;
/// Ergonomic alias for a locked SPI monitor handle.
pub type LockedSpiMonitor = SpiMonitor<Locked>;

// ---------------------------------------------------------------------------
// Uninitialized state
// ---------------------------------------------------------------------------

impl SpiMonitor<Uninitialized> {
    /// Construct a new controller facade for a specific monitor instance.
    ///
    /// # Safety
    /// Caller must guarantee exclusive ownership of the target SPIPF block and SCU.
    pub const unsafe fn new(controller: SpiMonitorController) -> Self {
        Self {
            regs: unsafe { SpiMonitorRegisters::new_for_controller(controller) },
            controller,
            scu: unsafe { ScuRegisters::new_global() },
            _mode: PhantomData,
        }
    }

    /// Program command-table and address-filter policy, then transition to
    /// `Configured`.
    ///
    /// Returns `Err(InvalidSlot)` if `allow_command_count` exceeds the command
    /// table length. Returns `Err(InvalidRegion)` if `region_count` exceeds
    /// `MAX_REGION_SLOTS`.
    pub fn apply_policy(self, policy: &MonitorPolicy) -> Result<SpiMonitor<Configured>> {
        if policy.allow_command_count > policy.allow_commands.len() {
            return Err(SpiMonitorError::InvalidSlot);
        }
        if policy.region_count > MAX_REGION_SLOTS {
            return Err(SpiMonitorError::InvalidRegion);
        }

        // Program command allow-list table (encoded SPIPFWT words + VALID bit).
        cmd_table::init_allow_cmd_table(
            &self.regs,
            &policy.allow_commands[..policy.allow_command_count],
        );

        // Program address privilege tables (bit-per-16KB SPIPFWA entries).
        for i in 0..policy.region_count {
            if let Some(region) = policy.regions[i] {
                addr_priv::configure_address_privilege(
                    &self.regs,
                    region.direction,
                    region.op,
                    region.start,
                    region.length,
                )?;
            }
        }

        Ok(SpiMonitor {
            regs: self.regs,
            controller: self.controller,
            scu: self.scu,
            _mode: PhantomData,
        })
    }

    #[must_use]
    pub const fn state(&self) -> MonitorState {
        MonitorState::Uninitialized
    }
}

// ---------------------------------------------------------------------------
// Configured state
// ---------------------------------------------------------------------------

impl SpiMonitor<Configured> {
    fn scu_instance(&self) -> SpiMonitorInstance {
        match self.controller {
            SpiMonitorController::Spim0 => SpiMonitorInstance::Spim0,
            SpiMonitorController::Spim1 => SpiMonitorInstance::Spim1,
            SpiMonitorController::Spim2 => SpiMonitorInstance::Spim2,
            SpiMonitorController::Spim3 => SpiMonitorInstance::Spim3,
        }
    }

    /// Enable the monitor filter (SPIPF000 bit 2) and MISO multi-func pin.
    pub fn enable(&self) {
        self.scu
            .set_spim_miso_multi_func(self.scu_instance(), true);
        self.regs.set_filter_enable(true);
    }

    /// Disable the monitor filter (SPIPF000 bit 2) and MISO multi-func pin.
    pub fn disable(&self) {
        self.regs.set_filter_enable(false);
        self.scu
            .set_spim_miso_multi_func(self.scu_instance(), false);
    }

    /// Configure passthrough mode (SPIPF000 passthrough bit).
    ///
    /// When `PassthroughMode::Enabled`, SPI traffic bypasses the filter.
    pub fn set_passthrough(&self, mode: PassthroughMode) {
        self.regs
            .set_single_passthrough(matches!(mode, PassthroughMode::Enabled));
    }

    /// Select the external SPI mux routing.
    ///
    /// Platform code maps `Sel0`/`Sel1` to ROT vs BMC/PCH roles.
    ///
    /// Correctly uses SCU0F0 register (ext_mux_select_sig_of_spipfN bits)
    /// for each SPIPF instance.
    pub fn set_ext_mux(&self, sel: ExtMuxSel) {
        use crate::scu::types::{ScuExtMuxSelect, SpiMonitorInstance};

        let mux_sel = match sel {
            ExtMuxSel::Sel0 => ScuExtMuxSelect::Mux0,
            ExtMuxSel::Sel1 => ScuExtMuxSelect::Mux1,
        };

        let instance = match self.controller {
            SpiMonitorController::Spim0 => SpiMonitorInstance::Spim0,
            SpiMonitorController::Spim1 => SpiMonitorInstance::Spim1,
            SpiMonitorController::Spim2 => SpiMonitorInstance::Spim2,
            SpiMonitorController::Spim3 => SpiMonitorInstance::Spim3,
        };

        self.scu.set_spim_ext_mux(instance, mux_sel);
    }

    /// Query the current external SPI mux selection.
    #[must_use]
    pub fn get_ext_mux(&self) -> ExtMuxSel {
        let instance = match self.controller {
            SpiMonitorController::Spim0 => SpiMonitorInstance::Spim0,
            SpiMonitorController::Spim1 => SpiMonitorInstance::Spim1,
            SpiMonitorController::Spim2 => SpiMonitorInstance::Spim2,
            SpiMonitorController::Spim3 => SpiMonitorInstance::Spim3,
        };
        match self.scu.get_spim_ext_mux(instance) {
            ScuExtMuxSelect::Mux0 => ExtMuxSel::Sel0,
            ScuExtMuxSelect::Mux1 => ExtMuxSel::Sel1,
        }
    }

    /// Drain violation log entries into `buf`. Returns the filled slice.
    ///
    /// Available in `Configured` state for diagnostic use during bring-up.
    pub fn drain_log<'a>(&self, buf: &'a mut [ViolationLogEntry]) -> &'a [ViolationLogEntry] {
        drain_log_impl(&self.regs, buf)
    }

    /// Lock monitor policy registers and transition to `Locked`.
    ///
    /// Activates all write-protection bits to prevent further policy changes.
    /// Complete lock sequence:
    /// - Write-disable SPIPFWA/SPIPFRA (address filter tables)
    /// - Lock all command table entries
    /// - Write-disable SPIPF000, SPIPF004, SPIPF010, SPIPF014
    pub fn lock(self) -> Result<SpiMonitor<Locked>> {
        // Placeholder: This single bit write is incomplete.
        // Full lock requires SPIPF07C write-disable bits.
        self.regs.modify_ctrl(|bits| *bits |= CTRL_LOCK_BIT);

        Ok(SpiMonitor {
            regs: self.regs,
            controller: self.controller,
            scu: self.scu,
            _mode: PhantomData,
        })
    }

    #[must_use]
    pub const fn state(&self) -> MonitorState {
        MonitorState::Configured
    }
}

// ---------------------------------------------------------------------------
// Locked state
// ---------------------------------------------------------------------------

impl SpiMonitor<Locked> {
    /// Configure passthrough mode in locked state.
    ///
    /// Passthrough is intentionally available post-lock because it is used
    /// during mux ownership transitions at runtime (e.g., BMC boot-hold/release).
    pub fn set_passthrough(&self, mode: PassthroughMode) {
        self.regs
            .set_single_passthrough(matches!(mode, PassthroughMode::Enabled));
    }

    /// Select the external SPI mux routing in locked state.
    ///
    /// Available post-lock for mux ownership transitions at runtime (e.g., BMC boot-hold/release).
    /// Uses SCU0F0 register.
    pub fn set_ext_mux(&self, sel: ExtMuxSel) {
        let mux = match sel {
            ExtMuxSel::Sel0 => ScuExtMuxSelect::Mux0,
            ExtMuxSel::Sel1 => ScuExtMuxSelect::Mux1,
        };
        let instance = match self.controller {
            SpiMonitorController::Spim0 => SpiMonitorInstance::Spim0,
            SpiMonitorController::Spim1 => SpiMonitorInstance::Spim1,
            SpiMonitorController::Spim2 => SpiMonitorInstance::Spim2,
            SpiMonitorController::Spim3 => SpiMonitorInstance::Spim3,
        };
        self.scu.set_spim_ext_mux(instance, mux);
    }

    /// Query the current external SPI mux selection in locked state.
    #[must_use]
    pub fn get_ext_mux(&self) -> ExtMuxSel {
        let instance = match self.controller {
            SpiMonitorController::Spim0 => SpiMonitorInstance::Spim0,
            SpiMonitorController::Spim1 => SpiMonitorInstance::Spim1,
            SpiMonitorController::Spim2 => SpiMonitorInstance::Spim2,
            SpiMonitorController::Spim3 => SpiMonitorInstance::Spim3,
        };
        match self.scu.get_spim_ext_mux(instance) {
            ScuExtMuxSelect::Mux0 => ExtMuxSel::Sel0,
            ScuExtMuxSelect::Mux1 => ExtMuxSel::Sel1,
        }
    }

    /// Drain violation log entries into `buf`. Returns the filled slice.
    ///
    /// Caller is responsible for synchronization and log-pointer reset.
    pub fn drain_log<'a>(&self, buf: &'a mut [ViolationLogEntry]) -> &'a [ViolationLogEntry] {
        drain_log_impl(&self.regs, buf)
    }

    #[must_use]
    pub const fn lock_state(&self) -> LockState {
        LockState::Locked
    }

    #[must_use]
    pub const fn state(&self) -> MonitorState {
        MonitorState::Locked
    }
}

// ---------------------------------------------------------------------------
// State-independent accessors
// ---------------------------------------------------------------------------

impl<Mode> SpiMonitor<Mode> {
    #[must_use]
    pub fn regs(&self) -> &SpiMonitorRegisters {
        &self.regs
    }

    #[must_use]
    pub const fn controller(&self) -> SpiMonitorController {
        self.controller
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// SPIPF000 bit positions (from ast1060_pac).
#[allow(dead_code)]
const CTRL_MULTI_PASSTHROUGH_BIT: u32 = 1 << 1; // enbl_multiple_bit_passthrough() in SPIPF000[1]
#[allow(dead_code)]
const CTRL_SW_RESET_BIT: u32 = 1 << 15; // sweng_rst() in SPIPF000[15] (SW Engine Reset)
#[allow(dead_code)]
const CTRL_EXT_MUX_SEL_BIT: u32 = 1 << 2; // PLACEHOLDER - NOT in SPIPF000! See note below.
#[allow(dead_code)]
const CTRL_LOCK_BIT: u32 = 1 << 31; // PLACEHOLDER - NOT in SPIPF000! See note below.
                                    //
                                    // NOTE: CTRL_EXT_MUX_SEL and CTRL_LOCK are NOT in SPIPF000 register:
                                    // - ExtMux is controlled via SCU0F0 register (ext_mux_select_sig_of_spipfN bits)
                                    // - Lock is controlled via SPIPF07C write-disable bits and individual command
                                    //   table entry lock bits.

/// Shared drain-log implementation used by both `Configured` and `Locked`.
fn drain_log_impl<'a>(
    regs: &SpiMonitorRegisters,
    buf: &'a mut [ViolationLogEntry],
) -> &'a [ViolationLogEntry] {
    let log_base = regs.log_ram_base_addr();
    let max_entries = regs.read_log_max_sz() as usize / core::mem::size_of::<u32>();
    let write_idx = regs.read_log_idx_reg() as usize;

    let available = write_idx.min(max_entries);
    let count = available.min(buf.len());

    for i in 0..count {
        // SAFETY: log_base is a hardware RAM address validated by the PAC
        // base-address mapping. Offset stays within [0, max_entries) words.
        let word = unsafe { core::ptr::read_volatile((log_base as *const u32).add(i)) };
        buf[i] = ViolationLogEntry::parse(word);
    }

    &buf[..count]
}
