// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Built-in SPI monitor policy profiles.
//!
//! Profiles provide command allow-lists only. Region entries are platform
//! policy and must be added by the caller via `SpiMonitorPolicy::add_region`
//! using the PFM or provisioned manifest for the specific device.

use crate::spimonitor::policy::SpiMonitorPolicy;

/// Runtime profile: read-focused allow-list suitable for steady-state boot/runtime.
#[must_use]
pub const fn runtime_read_only() -> SpiMonitorPolicy {
    let mut p = SpiMonitorPolicy::empty();
    p.allow_commands[0] = 0x03; // READ
    p.allow_commands[1] = 0x0B; // FAST_READ
    p.allow_commands[2] = 0x9F; // RDID
    p.allow_command_count = 3;
    p
}

/// Update profile: expands allow-list for controlled erase/program flows.
#[must_use]
pub const fn firmware_update_window() -> SpiMonitorPolicy {
    let mut p = runtime_read_only();
    p.allow_commands[3] = 0x06; // WREN
    p.allow_commands[4] = 0x20; // SE
    p.allow_commands[5] = 0x02; // PP
    p.allow_command_count = 6;
    p
}
