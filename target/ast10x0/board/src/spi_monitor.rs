// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Board-level SPI Monitor orchestration.
//!
//! `Ast1060SpiMonitor` is the concrete implementation of the `Monitor` trait, orchestrating
//! both SCU (for external mux control) and SPIPF (for enforcement, filtering, and locks).
//!
//! This layer ensures no duplication: mux control delegates to SCU routing, and SPIPF
//! operations delegate to SPI monitor registers.

use ast10x0_peripherals::scu::registers::ScuRegisters;
use ast10x0_peripherals::scu::types::{ScuExtMuxSelect, SpiMonitorInstance};
use ast10x0_peripherals::spimonitor::registers::SpiMonitorRegisters;
use ast10x0_peripherals::spimonitor::traits::SpiMonitorControl;
use ast10x0_peripherals::spimonitor::types::{
    BootError, BootResult, MuxSelect, PrivilegeDirection, PrivilegeOp, SpiMonitorId,
    SpiMonitorStatus,
};

/// Board-level Monitor orchestrator.
///
/// Holds mutable references to both SCU and SPIPF register blocks, allowing it to
/// orchestrate mux operations (via SCU) and enforcement/filtering (via SPIPF) from
/// a single unified interface.
///
/// # Example
///
/// ```ignore
/// let mut board = Ast1060Board::init();
/// let mut monitor = board.monitor();
/// monitor.set_mux(SpiMonitorId::Spim0, MuxSelect::RotControl)?;
/// monitor.set_address_privilege(/*...*/)?;
/// monitor.lock_policy(SpiMonitorId::Spim0)?;
/// ```
pub struct Ast1060SpiMonitor<'a> {
    scu: &'a mut ScuRegisters,
    spipf: &'a mut [SpiMonitorRegisters; 4],
    read_blocked_region_count: u8,
    #[allow(dead_code)]
    write_blocked_region_count: u8,
}

impl<'a> Ast1060SpiMonitor<'a> {
    /// Create a new board-level Monitor with access to both SCU and SPIPF.
    pub fn new(
        scu: &'a mut ScuRegisters,
        spipf: &'a mut [SpiMonitorRegisters; 4],
        read_blocked_region_count: u8,
        write_blocked_region_count: u8,
    ) -> Self {
        Self {
            scu,
            spipf,
            read_blocked_region_count,
            write_blocked_region_count,
        }
    }

    /// Get the register accessor for the specified monitor instance.
    #[inline]
    fn regs(&self, instance: SpiMonitorId) -> &SpiMonitorRegisters {
        match instance {
            SpiMonitorId::Spim0 => &self.spipf[0],
            SpiMonitorId::Spim1 => &self.spipf[1],
            SpiMonitorId::Spim2 => &self.spipf[2],
            SpiMonitorId::Spim3 => &self.spipf[3],
        }
    }

    /// Get mutable reference to register accessor (for write operations).
    #[inline]
    fn regs_mut(&mut self, instance: SpiMonitorId) -> &mut SpiMonitorRegisters {
        match instance {
            SpiMonitorId::Spim0 => &mut self.spipf[0],
            SpiMonitorId::Spim1 => &mut self.spipf[1],
            SpiMonitorId::Spim2 => &mut self.spipf[2],
            SpiMonitorId::Spim3 => &mut self.spipf[3],
        }
    }

    /// Map SpiMonitorId to SCU SpiMonitorInstance for routing operations.
    fn instance_to_scu(instance: SpiMonitorId) -> SpiMonitorInstance {
        match instance {
            SpiMonitorId::Spim0 => SpiMonitorInstance::Spim0,
            SpiMonitorId::Spim1 => SpiMonitorInstance::Spim1,
            SpiMonitorId::Spim2 => SpiMonitorInstance::Spim2,
            SpiMonitorId::Spim3 => SpiMonitorInstance::Spim3,
        }
    }

    /// Map MuxSelect to SCU external mux select value.
    fn mux_to_scu(mux: MuxSelect) -> ScuExtMuxSelect {
        match mux {
            MuxSelect::RotControl => ScuExtMuxSelect::Mux0,
            MuxSelect::HostControl => ScuExtMuxSelect::Mux1,
        }
    }

    /// Map SCU external mux select back to MuxSelect.
    fn scu_to_mux(scu_mux: ScuExtMuxSelect) -> MuxSelect {
        match scu_mux {
            ScuExtMuxSelect::Mux0 => MuxSelect::RotControl,
            ScuExtMuxSelect::Mux1 => MuxSelect::HostControl,
        }
    }

    /// Extract enforcement active flag from CTRL register.
    /// Enforcement is active when passthrough bits are NOT set.
    /// - SPIPF000[1] = enbl_single_bit_passthrough
    /// - SPIPF000[2] = enbl_multiple_bit_passthrough
    /// When both bits are 0, enforcement is active and SPI commands are filtered.
    fn is_enforcement_active(ctrl: u32) -> bool {
        let pass_bits = (ctrl >> 1) & 0x3;
        pass_bits == 0 // Enforcement active when passthrough disabled
    }

    /// Extract policy lock flag from lock/status register.
    /// Policy is locked when write-disable bits are set in SPIPF07C.
    /// SPIPF07C bit 0: wr_dis_of_spipfwa (write disable for address privilege tables)
    fn is_policy_locked(lock_status: u32) -> bool {
        // Write-disable bit in SPIPF07C
        // When this bit is set, policy tables cannot be modified
        (lock_status >> 0) & 0x1 != 0
    }
}

impl<'a> SpiMonitorControl for Ast1060SpiMonitor<'a> {
    fn set_mux(&mut self, instance: SpiMonitorId, mux: MuxSelect) -> BootResult<()> {
        // External mux selection is controlled via SCU0F0 register.
        // Delegate to SCU routing layer which has the actual register access.
        let scu_instance = Self::instance_to_scu(instance);
        let scu_mux = Self::mux_to_scu(mux);
        self.scu.set_spim_ext_mux(scu_instance, scu_mux);
        Ok(())
    }

    fn read_mux(&self, instance: SpiMonitorId) -> BootResult<MuxSelect> {
        // Read from SCU0F0 register via SCU routing layer.
        let scu_instance = Self::instance_to_scu(instance);
        let scu_mux = self.scu.get_spim_ext_mux(scu_instance);
        Ok(Self::scu_to_mux(scu_mux))
    }

    fn soft_reset(&mut self, instance: SpiMonitorId) -> BootResult<()> {
        let regs = self.regs_mut(instance);
        // Soft reset clears status/logs but preserves policy.
        // NON-BLOCKING TODO 1: Verify soft reset bit position from AST10x0 datasheet.
        // Current: uses bit 7 in SPIPF000 as placeholder. May need adjustment.
        // NON-BLOCKING TODO 2: Implement polling/timeout after write.
        // Pattern: poll SPIPF000 until reset bit clears or timeout (e.g., 100 microseconds).
        // In aspeed-rust, hardware self-clears the bit after reset completes.
        let mut ctrl = regs.read_ctrl();
        ctrl |= 0x80; // Soft reset bit (placeholder - verify with datasheet)
        regs.write_ctrl(ctrl);
        // TODO: Poll until bit 7 clears, with timeout check
        Ok(())
    }

    fn hardware_reset(&mut self, instance: SpiMonitorId) -> BootResult<()> {
        let regs = self.regs_mut(instance);
        // Full hardware reset of all state (SPIPF and related SCU registers).
        // NON-BLOCKING TODO 1: Verify hardware reset bit and sequence from AST10x0 datasheet.
        // Current: uses bit 8 in SPIPF000 as placeholder.
        // NON-BLOCKING TODO 2: Implement polling/timeout after write.
        // Check if reset bit self-clears like soft_reset, or if a separate status poll is needed.
        let mut ctrl = regs.read_ctrl();
        ctrl |= 0x100; // Hardware reset bit (placeholder - verify with datasheet)
        regs.write_ctrl(ctrl);
        // TODO: Poll until bit 8 clears or wait for timeout, similar to soft_reset
        Ok(())
    }

    fn set_address_privilege(
        &mut self,
        instance: SpiMonitorId,
        start_addr: u32,
        end_addr: u32,
        _direction: PrivilegeDirection,
        _op: PrivilegeOp,
    ) -> BootResult<()> {
        if end_addr < start_addr {
            return Err(BootError::InvalidAddress);
        }

        let regs = self.regs_mut(instance);

        // Address filtering in AST10x0 uses 16KB blocks (ACCESS_BLOCK_UNIT from aspeed-rust).
        // Each SPIPFWA register controls 32 blocks per register (32 * 16KB = 512KB per register).
        // NON-BLOCKING TODO: Implement dynamic slot allocation instead of fixed slot 0.
        // Phase C work: currently allocates all regions to slot 0 for simplicity.
        // Phase D work: track used slots, allocate new regions to free slots, or reject if full.
        let slot = 0;

        // Address filter encoding: SPIPFWA stores address range in HW format.
        // Placeholder uses start/end field assignment; actual format needs datasheet verification.
        let entry = ((start_addr & 0xFFF0_0000) << 0) | ((end_addr & 0x0F_FFFF) << 0);
        regs.write_addr_filter_slot(slot, entry);

        Ok(())
    }

    fn read_region_count(&self, _instance: SpiMonitorId) -> BootResult<u32> {
        // Region count is tracked in memory (following aspeed-rust pattern).
        // aspeed-rust stores read_blocked_region_num and write_blocked_region_num as struct fields.
        // We return the read-blocked region count for now; write-blocked can be exposed via separate method if needed.
        Ok(self.read_blocked_region_count as u32)
    }

    fn read_status(&self, instance: SpiMonitorId) -> BootResult<SpiMonitorStatus> {
        let regs = self.regs(instance);
        let ctrl = regs.read_ctrl();
        let lock_status = regs.read_lock_status();

        // Read actual mux state from SCU (not placeholder from SPIPF)
        let scu_instance = Self::instance_to_scu(instance);
        let scu_mux = self.scu.get_spim_ext_mux(scu_instance);
        let mux = Self::scu_to_mux(scu_mux);

        Ok(SpiMonitorStatus {
            mux,
            policy_locked: Self::is_policy_locked(lock_status),
            enforcement_active: Self::is_enforcement_active(ctrl),
            violation_count: 0, // NON-BLOCKING TODO: Implement violation log register reading
                                // Future: read from SPIPF violation count register if available
        })
    }

    fn supports_policy_lock(&self) -> bool {
        // Check hardware capability from a known register or constant.
        // Placeholder: assume AST10x0 always supports policy lock.
        true
    }

    fn lock_policy(&mut self, instance: SpiMonitorId) -> BootResult<()> {
        if !self.supports_policy_lock() {
            return Err(BootError::LockedOutFromMonitor);
        }

        let regs = self.regs_mut(instance);
        // Policy lock is controlled by SPIPF07C register.
        // From aspeed-rust: bit 0 is `wr_dis_of_spipfwa` (write disable for address tables).
        let mut lock_status = regs.read_lock_status();
        lock_status |= 0x1; // Set wr_dis_of_spipfwa bit
        regs.write_lock_status(lock_status);
        Ok(())
    }

    fn verify_policy_locked(&self, instance: SpiMonitorId) -> BootResult<()> {
        if !self.supports_policy_lock() {
            return Ok(());
        }

        let regs = self.regs(instance);
        let lock_status = regs.read_lock_status();
        if Self::is_policy_locked(lock_status) {
            Ok(())
        } else {
            Err(BootError::PolicyVerificationFailed)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enforcement_flag() {
        assert!(!Ast1060SpiMonitor::is_enforcement_active(0));
        assert!(Ast1060SpiMonitor::is_enforcement_active(1 << 4));
    }

    #[test]
    fn test_lock_flag() {
        assert!(!Ast1060SpiMonitor::is_policy_locked(0));
        assert!(Ast1060SpiMonitor::is_policy_locked(1));
    }
}
