// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Mock board: a device table exercising every device archetype the
//! orchestrator manages. Not a real board — consumed by host tests and QEMU
//! runs until a hardware target declares its own table.

#![no_std]

use core::time::Duration;

use orchestrator_config::{BootCheckpoint, BootSignal, CommitPolicy, ComponentId, DeviceConfig};

/// Declaration order is the boot order: the orchestrator releases devices
/// top to bottom, one at a time.
///
/// The mock board's reset controller and boot monitor both address
/// signals by plain index, so both id types are `u8`.
pub const MANAGED_DEVICES: [DeviceConfig<u8, u8>; 2] = [
    // Direct-flash SPI device (BMC archetype): the eRoT fronts its flash.
    // Single checkpoint: it raises a boot-complete GPIO.
    DeviceConfig {
        id: ComponentId::new(0),
        name: "bmc",
        reset_signal: 7,
        checkpoints: &[BootCheckpoint {
            name: "boot-complete",
            signal: BootSignal::GpioBootComplete(12),
            window: Duration::from_secs(90),
        }],
        commit_policy: CommitPolicy::Liveness,
    },
    // PLDM device (NIC archetype): self-updating, SPDM-capable. Two
    // checkpoints, exercising the multi-checkpoint path.
    DeviceConfig {
        id: ComponentId::new(1),
        name: "nic",
        reset_signal: 3,
        checkpoints: &[
            BootCheckpoint {
                name: "mctp-ready",
                signal: BootSignal::MctpReady,
                window: Duration::from_secs(20),
            },
            BootCheckpoint {
                name: "heartbeat",
                signal: BootSignal::Heartbeat,
                window: Duration::from_secs(10),
            },
        ],
        commit_policy: CommitPolicy::LivenessAndAttestation,
    },
];

const _: () = orchestrator_config::validate(&MANAGED_DEVICES);
