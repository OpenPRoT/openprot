// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Mock board: a device table exercising every device archetype the
//! orchestrator manages. Not a real board — consumed by host tests and QEMU
//! runs until a hardware target declares its own table.

#![no_std]

use fwmanager_api::config::{CommitPolicy, DeviceConfig};

/// Declaration order is the boot order: the orchestrator releases devices
/// top to bottom, one at a time.
pub const MANAGED_DEVICES: &[DeviceConfig] = &[
    // Direct-flash SPI device (BMC archetype): the eRoT fronts its flash.
    DeviceConfig {
        name: "bmc",
        reset_line: 7,
        boot_timeout: core::time::Duration::from_secs(90),
        commit_policy: CommitPolicy::Liveness,
    },
    // PLDM device (NIC archetype): self-updating, SPDM-capable.
    DeviceConfig {
        name: "nic",
        reset_line: 3,
        boot_timeout: core::time::Duration::from_secs(30),
        commit_policy: CommitPolicy::LivenessAndAttestation,
    },
];

const _: () = fwmanager_api::config::validate(MANAGED_DEVICES);
