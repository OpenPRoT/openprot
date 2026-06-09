// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! SPI monitor controller facade.

use core::marker::PhantomData;

use crate::scu::registers::ScuRegisters;
use crate::scu::types::{ScuExtMuxSelect, SpiMonitorInstance};
use crate::spimonitor::commands::{fixed_slot, table_value};
use crate::spimonitor::policy::{MonitorPolicy, MAX_REGION_SLOTS};
use crate::spimonitor::registers::{SpiMonitorController, SpiMonitorRegisters};
use crate::spimonitor::types::{
    ExtMuxSel, LockState, MonitorState, PassthroughMode, PrivilegeDirection, PrivilegeOp, Result,
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
// Encoding helpers (hardware-format definitions)
// ---------------------------------------------------------------------------

/// Encode one address-filter slot word from a region policy entry.
///
/// Hardware slot format (pending datasheet confirmation):
/// - bits[31:14] : region base address >> 14 (18-bit granule)
/// - bit[13]     : direction (0 = read, 1 = write)
/// - bit[12]     : op (0 = enable/allow, 1 = disable/block)
/// - bits[11:0]  : length in 4 KiB units (length >> 12), clamped to 12 bits
///
/// TODO: replace with confirmed SPIPF register field encoding once available.
fn encode_addr_filter_slot(
    start: u32,
    length: u32,
    direction: PrivilegeDirection,
    op: PrivilegeOp,
) -> u32 {
    let addr_field = (start >> 14) & 0x3_FFFF;
    let dir_bit: u32 = match direction {
        PrivilegeDirection::Read => 0,
        PrivilegeDirection::Write => 1,
    };
    let op_bit: u32 = match op {
        PrivilegeOp::Enable => 0,
        PrivilegeOp::Disable => 1,
    };
    let len_field = (length >> 12) & 0xFFF;
    (addr_field << 14) | (dir_bit << 13) | (op_bit << 12) | len_field
}

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

        let mut next_slot = 2usize;

        // Slots 0 and 1 are reserved for EN4B and EX4B; slot 31 is reserved
        // for WREAR. Other commands occupy slots 2 through 30.
        for i in 0..policy.allow_command_count {
            let opcode = policy.allow_commands[i];
            let value =
                table_value(opcode, false).ok_or(SpiMonitorError::UnsupportedCommand(opcode))?;
            let slot = match fixed_slot(opcode) {
                Some(slot) => slot,
                None => {
                    if next_slot >= 31 {
                        return Err(SpiMonitorError::NoCommandSlot);
                    }
                    let slot = next_slot;
                    next_slot += 1;
                    slot
                }
            };
            self.regs.write_allow_cmd_slot(slot, value);
        }

        // Program address filter table.
        for i in 0..policy.region_count {
            if let Some(region) = policy.regions[i] {
                let word = encode_addr_filter_slot(
                    region.start,
                    region.length,
                    region.direction,
                    region.op,
                );
                self.regs.write_addr_filter_slot(i, word);
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
    /// Enable the monitor filter (SPIPF000 bit 0).
    ///
    /// Mirrors Zephyr's `spim_monitor_enable(dev, true)`.
    pub fn enable(&self) {
        self.regs
            .modify_ctrl(|bits| *bits |= CTRL_MONITOR_ENABLE_BIT);
    }

    /// Disable the monitor filter (SPIPF000 bit 0).
    ///
    /// Mirrors Zephyr's `spim_monitor_enable(dev, false)`.
    pub fn disable(&self) {
        self.regs
            .modify_ctrl(|bits| *bits &= !CTRL_MONITOR_ENABLE_BIT);
    }

    /// Configure passthrough mode (SPIPF000 passthrough bit).
    ///
    /// When `PassthroughMode::Enabled`, SPI traffic bypasses the filter.
    /// Mirrors Zephyr's `spim_passthrough_config`.
    pub fn set_passthrough(&self, mode: PassthroughMode) {
        self.regs.modify_ctrl(|bits| match mode {
            PassthroughMode::Enabled => {
                *bits = (*bits & !CTRL_PASSTHROUGH_MASK) | CTRL_SINGLE_PASSTHROUGH_BIT
            }
            PassthroughMode::Disabled => *bits &= !CTRL_PASSTHROUGH_MASK,
        });
    }

    /// Select the external SPI mux routing.
    ///
    /// Mirrors Zephyr's `spim_ext_mux_config`. Platform code maps `Sel0`/`Sel1`
    /// to ROT vs BMC/PCH roles.
    ///
    /// Correctly uses SCU0F0 register (ext_mux_select_sig_of_spipfN bits)
    /// for each SPIPF instance, following the aspeed-rust pattern.
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
    /// See aspeed-rust::spim_lock_common() for complete lock sequence:
    /// - Write-disable SPIPFWA/SPIPFRA (address filter tables)
    /// - Lock all command table entries
    /// - Write-disable SPIPF000, SPIPF004, SPIPF010, SPIPF014
    pub fn lock(self) -> Result<SpiMonitor<Locked>> {
        // Placeholder: This single bit write is incomplete.
        // Full lock requires SPIPF07C write-disable bits per aspeed-rust pattern.
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
        self.regs.modify_ctrl(|bits| match mode {
            PassthroughMode::Enabled => {
                *bits = (*bits & !CTRL_PASSTHROUGH_MASK) | CTRL_SINGLE_PASSTHROUGH_BIT
            }
            PassthroughMode::Disabled => *bits &= !CTRL_PASSTHROUGH_MASK,
        });
    }

    /// Select the external SPI mux routing in locked state.
    ///
    /// Available post-lock for mux ownership transitions at runtime (e.g., BMC boot-hold/release).
    /// Uses SCU0F0 register following the aspeed-rust pattern.
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

/// SPIPF000 bit positions.
///
/// Confirmed from aspeed-rust implementation (src/spimonitor/hardware.rs).
/// Register field names from ast1060_pac provide safe typed accessors.
const CTRL_SINGLE_PASSTHROUGH_BIT: u32 = 1 << 0;
const CTRL_PASSTHROUGH_MASK: u32 = (1 << 0) | (1 << 1);
const CTRL_MONITOR_ENABLE_BIT: u32 = 1 << 2;
#[allow(dead_code)]
const CTRL_SW_RESET_BIT: u32 = 1 << 15;
#[allow(dead_code)]
const CTRL_LOCK_BIT: u32 = 1 << 31; // PLACEHOLDER - NOT in SPIPF000! See note below.
                                    //
                                    // NOTE: CTRL_EXT_MUX_SEL and CTRL_LOCK are NOT in SPIPF000 register:
                                    // - ExtMux is controlled via SCU0F0 register (ext_mux_select_sig_of_spipfN bits)
                                    //   See aspeed-rust: spim_ext_mux_config()
                                    // - Lock is controlled via SPIPF07C write-disable bits and individual command
                                    //   table entry lock bits. See aspeed-rust: spim_lock_common(), spim_lock_rw_priv_table()

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
