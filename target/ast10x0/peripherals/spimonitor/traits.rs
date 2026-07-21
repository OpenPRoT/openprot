// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Boot sequence traits for SPI monitor.
//!
//! This module provides the trait-based interface used by boot code to interact
//! with SPI monitor hardware during platform initialization.

use super::types::{
    BootError, BootResult, MuxSelect, PrivilegeDirection, PrivilegeOp, SpiMonitorId,
    SpiMonitorStatus,
};

/// Abstract hardware interface for SPI monitor boot operations.
///
/// Implementations of this trait provide register-level access to SPI monitor
/// blocks via PAC or other register models. Boot code uses this trait to remain
/// independent of the concrete register implementation.
pub trait SpiMonitorControl {
    /// Set monitor mux to ROT or Host control.
    fn set_mux(&mut self, instance: SpiMonitorId, mux: MuxSelect) -> BootResult<()>;

    /// Read current mux setting.
    fn read_mux(&self, instance: SpiMonitorId) -> BootResult<MuxSelect>;

    /// Soft reset monitor (clears status/logs, preserves policy).
    fn soft_reset(&mut self, instance: SpiMonitorId) -> BootResult<()>;

    /// Hardware reset monitor (full reset of all state).
    fn hardware_reset(&mut self, instance: SpiMonitorId) -> BootResult<()>;

    /// Configure an address privilege region.
    ///
    /// # Arguments
    /// * `instance` - Monitor instance to configure
    /// * `start_addr` - Start address (inclusive)
    /// * `end_addr` - End address (inclusive)
    /// * `direction` - Read or Write direction
    /// * `op` - Allow or Block access
    fn set_address_privilege(
        &mut self,
        instance: SpiMonitorId,
        start_addr: u32,
        end_addr: u32,
        direction: PrivilegeDirection,
        op: PrivilegeOp,
    ) -> BootResult<()>;

    /// Read number of configured address privilege regions.
    fn read_region_count(&self, instance: SpiMonitorId) -> BootResult<u32>;

    /// Read monitor status snapshot.
    fn read_status(&self, instance: SpiMonitorId) -> BootResult<SpiMonitorStatus>;

    /// Check if policy write-lock is supported by this monitor.
    fn supports_policy_lock(&self) -> bool {
        false
    }

    /// Lock policy tables to prevent further modification.
    ///
    /// Returns error if not supported by hardware.
    fn lock_policy(&mut self, _instance: SpiMonitorId) -> BootResult<()> {
        if !self.supports_policy_lock() {
            return Err(BootError::LockedOutFromMonitor);
        }
        Ok(())
    }

    /// Verify that policy is locked (if supported).
    ///
    /// No-op if policy lock is not supported.
    fn verify_policy_locked(&self, instance: SpiMonitorId) -> BootResult<()> {
        if !self.supports_policy_lock() {
            return Ok(());
        }
        let status = self.read_status(instance)?;
        if status.policy_locked {
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
    fn test_mux_select_conversion() {
        assert_eq!(ExtMuxSel::from(MuxSelect::RotControl), ExtMuxSel::Sel0);
        assert_eq!(ExtMuxSel::from(MuxSelect::HostControl), ExtMuxSel::Sel1);
    }

    #[test]
    fn test_mux_select_round_trip() {
        let orig = MuxSelect::RotControl;
        let ext = ExtMuxSel::from(orig);
        let restored: MuxSelect = ext.into();
        assert_eq!(orig, restored);
    }
}
