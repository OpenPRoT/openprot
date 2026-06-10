// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! SPI monitor controller facade.

use core::marker::PhantomData;

use crate::scu::registers::ScuRegisters;
use crate::scu::types::{ScuExtMuxSelect, SpiMonitorInstance};
use crate::spimonitor::commands::{fixed_slot, table_value, LOCKED as COMMAND_LOCKED};
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
        if self.regs.read_lock_status() & LOCK_STATUS_REQUIRED != 0 {
            return Err(SpiMonitorError::Locked);
        }
        for slot in 0..COMMAND_TABLE_SLOTS {
            if self.regs.read_allow_cmd_slot(slot) & COMMAND_LOCKED != 0 {
                return Err(SpiMonitorError::Locked);
            }
        }

        let general_slot_limit = if policy.allow_commands[..policy.allow_command_count]
            .contains(&0xc5)
        {
            LAST_GENERAL_COMMAND_SLOT_EXCLUSIVE
        } else {
            COMMAND_TABLE_SLOTS
        };
        let mut general_command_count = 0usize;
        for opcode in policy.allow_commands[..policy.allow_command_count]
            .iter()
            .copied()
        {
            if table_value(opcode, false).is_none() {
                return Err(SpiMonitorError::UnsupportedCommand(opcode));
            }
            if fixed_slot(opcode).is_none() {
                general_command_count += 1;
            }
        }
        if general_command_count > general_slot_limit - FIRST_GENERAL_COMMAND_SLOT {
            return Err(SpiMonitorError::NoCommandSlot);
        }
        for region in policy.regions[..policy.region_count].iter().flatten() {
            validate_privilege_region(region.start, region.length)?;
        }

        for slot in 0..COMMAND_TABLE_SLOTS {
            self.regs.write_allow_cmd_slot(slot, 0);
        }

        let mut next_slot = FIRST_GENERAL_COMMAND_SLOT;
        // Slots 0 and 1 are reserved for EN4B and EX4B; slot 31 is reserved
        // for WREAR when it is present. Otherwise slot 31 can hold a generic
        // command, allowing the supplied 32-command Zephyr policy to fit.
        for i in 0..policy.allow_command_count {
            let opcode = policy.allow_commands[i];
            let value =
                table_value(opcode, false).ok_or(SpiMonitorError::UnsupportedCommand(opcode))?;
            let slot = match fixed_slot(opcode) {
                Some(slot) => slot,
                None => {
                    if next_slot >= general_slot_limit {
                        return Err(SpiMonitorError::NoCommandSlot);
                    }
                    let slot = next_slot;
                    next_slot += 1;
                    slot
                }
            };
            self.regs.write_allow_cmd_slot(slot, value);
            if self.regs.read_allow_cmd_slot(slot) != value {
                return Err(SpiMonitorError::VerificationFailed);
            }
        }

        // Empty policy means unrestricted access, matching the Zephyr driver:
        // initialize both 256 MiB privilege maps to all-allowed, then apply
        // the requested deny/allow regions.
        initialize_privilege_table(&self.regs, PrivilegeDirection::Read)?;
        initialize_privilege_table(&self.regs, PrivilegeDirection::Write)?;

        for i in 0..policy.region_count {
            if let Some(region) = policy.regions[i] {
                configure_privilege_region(
                    &self.regs,
                    region.start,
                    region.length,
                    region.direction,
                    region.op,
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

    /// Pulse the SPIPF software-reset bit for at least 5 microseconds.
    pub fn software_reset(&self) {
        self.regs
            .modify_ctrl(|bits| *bits |= CTRL_SW_RESET_BIT);
        delay_cycles(SW_RESET_DELAY_CYCLES);
        self.regs
            .modify_ctrl(|bits| *bits &= !CTRL_SW_RESET_BIT);
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

    /// Configure a caller-owned, static DMA buffer for violation logging.
    ///
    /// The hardware stores one 32-bit violation record per entry.
    pub fn configure_log(&self, buffer: &'static mut [u32]) -> Result<()> {
        if buffer.is_empty() || buffer.len() > MAX_LOG_ENTRIES {
            return Err(SpiMonitorError::InvalidLogBuffer);
        }
        let address = buffer.as_mut_ptr() as usize;
        if address & 0x3 != 0 || address > u32::MAX as usize {
            return Err(SpiMonitorError::InvalidLogBuffer);
        }
        buffer.fill(0);
        self.regs
            .write_log_config(address as u32, buffer.len() as u32);
        Ok(())
    }

    /// Use push-pull signaling for the monitor output path.
    pub fn set_push_pull(&self, enable: bool) {
        self.regs.modify_ctrl2(|bits| {
            if enable {
                *bits |= CTRL2_PUSH_PULL;
            } else {
                *bits &= !CTRL2_PUSH_PULL;
            }
        });
    }

    /// Enable command, write, and read violation interrupts in SPIPF004.
    ///
    /// Platform code must install and enable the corresponding NVIC handler.
    pub fn enable_violation_interrupts(&self) {
        self.regs
            .modify_ctrl2(|bits| *bits |= CTRL2_VIOLATION_IRQ_ENABLE_MASK);
    }

    /// Return currently pending command/write/read violation status bits.
    #[must_use]
    pub fn pending_violations(&self) -> u32 {
        self.regs.read_ctrl2() & CTRL2_VIOLATION_STATUS_MASK
    }

    /// Acknowledge all currently pending violation status bits.
    pub fn acknowledge_violations(&self) -> u32 {
        let pending = self.pending_violations();
        if pending != 0 {
            self.regs
                .modify_ctrl2(|bits| *bits |= pending);
        }
        pending
    }

    /// Lock monitor policy registers and transition to `Locked`.
    ///
    /// Activates all write-protection bits to prevent further policy changes.
    /// See aspeed-rust::spim_lock_common() for complete lock sequence:
    /// - Write-disable SPIPFWA/SPIPFRA (address filter tables)
    /// - Lock all command table entries
    /// - Write-disable SPIPF000, SPIPF004, SPIPF010, SPIPF014
    pub fn lock(self) -> Result<SpiMonitor<Locked>> {
        for slot in 0..COMMAND_TABLE_SLOTS {
            let value = self.regs.read_allow_cmd_slot(slot);
            self.regs
                .write_allow_cmd_slot(slot, value | COMMAND_LOCKED);
        }

        self.regs
            .modify_ctrl(|bits| *bits |= CTRL_BLOCK_FIFO_LOCK | CTRL_SW_RESET_LOCK);
        self.regs.modify_lock_status(|bits| {
            *bits |= LOCK_CTRL
                | LOCK_IRQ_CTRL
                | LOCK_LOG_BASE
                | LOCK_LOG_CTRL
                | LOCK_WRITE_PRIVILEGE
                | LOCK_READ_PRIVILEGE;
        });

        let lock_status = self.regs.read_lock_status();
        if lock_status & LOCK_STATUS_REQUIRED != LOCK_STATUS_REQUIRED {
            return Err(SpiMonitorError::LockFailed);
        }
        for slot in 0..COMMAND_TABLE_SLOTS {
            if self.regs.read_allow_cmd_slot(slot) & COMMAND_LOCKED == 0 {
                return Err(SpiMonitorError::LockFailed);
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
        MonitorState::Configured
    }
}

// ---------------------------------------------------------------------------
// Locked state
// ---------------------------------------------------------------------------

impl SpiMonitor<Locked> {
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
const CTRL_BLOCK_FIFO_LOCK: u32 = 1 << 22;
const CTRL_SW_RESET_LOCK: u32 = 1 << 23;
const CTRL2_VIOLATION_STATUS_MASK: u32 = 0x7;
const CTRL2_VIOLATION_IRQ_ENABLE_MASK: u32 = 0x7 << 16;
const CTRL2_PUSH_PULL: u32 = 1 << 31;

const COMMAND_TABLE_SLOTS: usize = 32;
const FIRST_GENERAL_COMMAND_SLOT: usize = 2;
const LAST_GENERAL_COMMAND_SLOT_EXCLUSIVE: usize = 31;
const PRIVILEGE_TABLE_WORDS: usize = 512;
const PRIVILEGE_BLOCK_SIZE: u32 = 16 * 1024;
const PRIVILEGE_ADDRESS_LIMIT: u32 = 256 * 1024 * 1024;
const PRIVILEGE_READ_SELECT: u32 = 0x5200_0000;
const PRIVILEGE_WRITE_SELECT: u32 = 0x5700_0000;
const MAX_LOG_ENTRIES: usize = 0x7_ffff;
const SW_RESET_DELAY_CYCLES: u32 = 1_000;

const LOCK_CTRL: u32 = 1 << 0;
const LOCK_IRQ_CTRL: u32 = 1 << 1;
const LOCK_LOG_BASE: u32 = 1 << 4;
const LOCK_LOG_CTRL: u32 = 1 << 5;
const LOCK_WRITE_PRIVILEGE: u32 = 1 << 30;
const LOCK_READ_PRIVILEGE: u32 = 1 << 31;
const LOCK_STATUS_REQUIRED: u32 = LOCK_CTRL
    | LOCK_IRQ_CTRL
    | LOCK_LOG_BASE
    | LOCK_LOG_CTRL
    | LOCK_WRITE_PRIVILEGE
    | LOCK_READ_PRIVILEGE;

fn select_privilege_table(regs: &SpiMonitorRegisters, direction: PrivilegeDirection) {
    let selection = match direction {
        PrivilegeDirection::Read => PRIVILEGE_READ_SELECT,
        PrivilegeDirection::Write => PRIVILEGE_WRITE_SELECT,
    };
    regs.modify_ctrl(|bits| *bits = (*bits & 0x00ff_ffff) | selection);
}

fn initialize_privilege_table(
    regs: &SpiMonitorRegisters,
    direction: PrivilegeDirection,
) -> Result<()> {
    select_privilege_table(regs, direction);
    for index in 0..PRIVILEGE_TABLE_WORDS {
        regs.write_addr_filter_slot(index, u32::MAX);
    }
    if regs.read_addr_filter_slot(0) != u32::MAX
        || regs.read_addr_filter_slot(PRIVILEGE_TABLE_WORDS - 1) != u32::MAX
    {
        return Err(SpiMonitorError::VerificationFailed);
    }
    Ok(())
}

fn configure_privilege_region(
    regs: &SpiMonitorRegisters,
    start: u32,
    length: u32,
    direction: PrivilegeDirection,
    op: PrivilegeOp,
) -> Result<()> {
    validate_privilege_region(start, length)?;
    let end = start
        .checked_add(length)
        .ok_or(SpiMonitorError::InvalidLength)?
        .min(PRIVILEGE_ADDRESS_LIMIT);
    let aligned_start = start / PRIVILEGE_BLOCK_SIZE * PRIVILEGE_BLOCK_SIZE;
    let aligned_end = end
        .checked_add(PRIVILEGE_BLOCK_SIZE - 1)
        .ok_or(SpiMonitorError::InvalidLength)?
        / PRIVILEGE_BLOCK_SIZE
        * PRIVILEGE_BLOCK_SIZE;
    let mut block = aligned_start / PRIVILEGE_BLOCK_SIZE;
    let end_block = aligned_end / PRIVILEGE_BLOCK_SIZE;

    select_privilege_table(regs, direction);
    while block < end_block {
        let word_index = (block / 32) as usize;
        let bit_index = block % 32;
        let remaining = end_block - block;
        let updated = if bit_index == 0 && remaining >= 32 {
            block += 32;
            match op {
                PrivilegeOp::Enable => u32::MAX,
                PrivilegeOp::Disable => 0,
            }
        } else {
            block += 1;
            let value = regs.read_addr_filter_slot(word_index);
            match op {
                PrivilegeOp::Enable => value | (1 << bit_index),
                PrivilegeOp::Disable => value & !(1 << bit_index),
            }
        };
        regs.write_addr_filter_slot(word_index, updated);
        if regs.read_addr_filter_slot(word_index) != updated {
            return Err(SpiMonitorError::VerificationFailed);
        }
    }
    Ok(())
}

fn validate_privilege_region(start: u32, length: u32) -> Result<()> {
    if start >= PRIVILEGE_ADDRESS_LIMIT {
        return Err(SpiMonitorError::InvalidAddress);
    }
    if length == 0 || start.checked_add(length).is_none() {
        return Err(SpiMonitorError::InvalidLength);
    }
    Ok(())
}

fn delay_cycles(cycles: u32) {
    for _ in 0..cycles {
        core::hint::spin_loop();
    }
}

/// Shared drain-log implementation used by both `Configured` and `Locked`.
fn drain_log_impl<'a>(
    regs: &SpiMonitorRegisters,
    buf: &'a mut [ViolationLogEntry],
) -> &'a [ViolationLogEntry] {
    let log_base = regs.log_ram_base_addr();
    let max_entries = regs.read_log_capacity_entries() as usize;
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
