// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Mock board: a device table exercising every device archetype the
//! orchestrator manages. Not a real board — consumed by host tests and QEMU
//! runs until a hardware target declares its own table.

#![no_std]

use core::convert::Infallible;
use core::time::Duration;

use orchestrator_capabilities::BootStatus;
use orchestrator_config::{BootCheckpoint, CommitPolicy, DeviceConfig};

/// The mock board's device context: the signal state every checkpoint
/// check reads. Stands in for real drivers until the mock platform grows
/// them; the reset path is responsible for clearing latched fields (see
/// `BootStatus`).
#[derive(Debug, Default)]
pub struct MockBoard {
    /// bmc boot-complete line.
    pub bmc_ready: bool,
    /// nic MCTP endpoint answers as ready.
    pub nic_mctp_ready: bool,
    /// nic heartbeat observed (latched).
    pub nic_heartbeat: bool,
}

const fn up(ready: bool) -> BootStatus {
    if ready {
        BootStatus::Booted
    } else {
        BootStatus::Booting
    }
}

/// Declaration order is the boot order: the orchestrator releases devices
/// top to bottom, one at a time.
///
/// The mock board's reset controller addresses reset lines by plain index,
/// so the reset id type is `u8`.
pub const MANAGED_DEVICES: &[DeviceConfig<u8, MockBoard, Infallible>] = &[
    // Direct-flash SPI device (BMC archetype): the eRoT fronts its flash.
    // Single checkpoint: it raises a boot-complete GPIO.
    DeviceConfig {
        name: "bmc",
        reset_signal: 7,
        checkpoints: &[BootCheckpoint {
            name: "boot-complete",
            timeout: Duration::from_secs(90),
            max_retries: 1,
            passed: |b| Ok(up(b.bmc_ready)),
        }],
        commit_policy: CommitPolicy::Liveness,
    },
    // PLDM device (NIC archetype): self-updating, SPDM-capable. Two
    // checkpoints, exercising the multi-checkpoint path: transport up
    // first, then proof the workload is alive.
    DeviceConfig {
        name: "nic",
        reset_signal: 3,
        checkpoints: &[
            BootCheckpoint {
                name: "mctp-ready",
                timeout: Duration::from_secs(20),
                max_retries: 2,
                passed: |b| Ok(up(b.nic_mctp_ready)),
            },
            BootCheckpoint {
                name: "heartbeat",
                timeout: Duration::from_secs(10),
                max_retries: 0,
                passed: |b| Ok(up(b.nic_heartbeat)),
            },
        ],
        commit_policy: CommitPolicy::LivenessAndAttestation,
    },
];

const _: () = orchestrator_config::validate(MANAGED_DEVICES);
