// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Mock board: a device table exercising every device archetype the
//! orchestrator manages. Not a real board — consumed by host tests and QEMU
//! runs until a hardware target declares its own table.

#![no_std]

use core::time::Duration;

use orchestrator_config::{BootCheckpoint, CommitPolicy, DeviceConfig};

/// The mock board's boot-signal vocabulary. The schema carries these
/// opaquely; only this board's `EvidenceReader` gives them meaning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MockSignal {
    /// A boot-complete GPIO line, by index.
    Gpio(u8),
    /// The device's MCTP endpoint answers as ready.
    MctpReady,
    /// The device sends a heartbeat message (latched; the reset path
    /// clears it).
    Heartbeat,
}

/// Declaration order is the boot order: the orchestrator releases devices
/// top to bottom, one at a time.
///
/// The mock board's reset controller addresses reset lines by plain index,
/// so the reset id type is `u8`.
pub const MANAGED_DEVICES: &[DeviceConfig<u8, MockSignal>] = &[
    // Direct-flash SPI device (BMC archetype): the eRoT fronts its flash.
    // Single checkpoint: it raises a boot-complete GPIO.
    DeviceConfig {
        name: "bmc",
        reset_signal: 7,
        checkpoints: &[BootCheckpoint {
            name: "boot-complete",
            signal: MockSignal::Gpio(12),
            timeout: Duration::from_secs(90),
            max_retries: 1,
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
                signal: MockSignal::MctpReady,
                timeout: Duration::from_secs(20),
                max_retries: 2,
            },
            BootCheckpoint {
                name: "heartbeat",
                signal: MockSignal::Heartbeat,
                timeout: Duration::from_secs(10),
                max_retries: 0,
            },
        ],
        commit_policy: CommitPolicy::LivenessAndAttestation,
    },
];

const _: () = orchestrator_config::validate(MANAGED_DEVICES);
