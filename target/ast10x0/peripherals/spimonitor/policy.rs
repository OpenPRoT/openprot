// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Declarative policy model for SPI monitor configuration.

use crate::spimonitor::types::{PrivilegeDirection, PrivilegeOp, RegionPolicy};

/// Maximum number of address filter regions per monitor instance.
pub const MAX_REGION_SLOTS: usize = 16;

/// Maximum number of command allow-list entries per monitor instance.
pub const MAX_CMD_SLOTS: usize = 32;

/// Policy payload applied to a monitor instance.
#[derive(Clone, Debug)]
pub struct MonitorPolicy {
    pub allow_commands: [u8; MAX_CMD_SLOTS],
    pub allow_command_count: usize,
    pub regions: [Option<RegionPolicy>; MAX_REGION_SLOTS],
    pub region_count: usize,
}

impl MonitorPolicy {
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            allow_commands: [0; MAX_CMD_SLOTS],
            allow_command_count: 0,
            regions: [None; MAX_REGION_SLOTS],
            region_count: 0,
        }
    }

    /// Add an address region entry. Returns `false` if the table is full.
    pub fn add_region(
        &mut self,
        start: u32,
        length: u32,
        direction: PrivilegeDirection,
        op: PrivilegeOp,
    ) -> bool {
        if self.region_count >= MAX_REGION_SLOTS {
            return false;
        }
        self.regions[self.region_count] = Some(RegionPolicy { start, length, direction, op });
        self.region_count += 1;
        true
    }
}
