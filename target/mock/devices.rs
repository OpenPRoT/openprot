// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Mock board: a device table exercising every device archetype the
//! orchestrator manages. Not a real board — consumed by host tests and QEMU
//! runs until a hardware target declares its own table.

#![no_std]

use core::time::Duration;

use orchestrator_config::{BootCheckpoint, BootSignal, CommitPolicy, DeviceConfig, Slot, SlotId};

/// Declaration order is the boot order: the orchestrator releases devices
/// top to bottom, one at a time.
///
/// The mock board's reset controller and boot monitor both address
/// signals by plain index, so both id types are `u8`.
pub const MANAGED_DEVICES: &[DeviceConfig<u8, u8>] = &[
    // Direct-flash SPI device (BMC archetype): the eRoT fronts its flash.
    // Single checkpoint: it raises a boot-complete GPIO.
    DeviceConfig {
        name: "bmc",
        reset_signal: 7,
        checkpoints: &[BootCheckpoint {
            name: "boot-complete",
            signal: BootSignal::GpioBootComplete(12),
            window: Duration::from_secs(90),
        }],
        commit_policy: CommitPolicy::Liveness,
        // Plain A/B: two equal writable slots, recovery falls back to the
        // other one. Real boards declare their own topology; a golden
        // slot is optional.
        slots: &[
            Slot::new(SlotId(0), true, true, None),
            Slot::new(SlotId(1), true, true, None),
        ],
    },
    // PLDM device (NIC archetype): self-updating, SPDM-capable. Two
    // checkpoints, exercising the multi-checkpoint path.
    DeviceConfig {
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
        // Self-updating device: it owns its boot selection, the eRoT never
        // sees its slot topology. No local ladder rungs, so recovery can
        // only escalate.
        slots: &[],
    },
];

const _: () = orchestrator_config::validate(MANAGED_DEVICES);
