// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Mock board: a device table exercising every device archetype the
//! orchestrator manages. Not a real board — consumed by host tests and QEMU
//! runs until a hardware target declares its own table.

#![no_std]

use core::time::Duration;

use fwmanager_api::config::{
    BootCheckpoint, BootSignal, CommitPolicy, DeviceConfig, RecoveryPolicy, SlotDesc, SlotId,
    SlotRole,
};

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
            SlotDesc {
                id: SlotId(0),
                writable: true,
                bootable: true,
                role: None,
            },
            SlotDesc {
                id: SlotId(1),
                writable: true,
                bootable: true,
                role: None,
            },
        ],
        recovery_policy: RecoveryPolicy::Ladder,
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
        recovery_policy: RecoveryPolicy::EscalateOnly,
    },
    // Passive downstream SPI device (symbiont archetype): the eRoT fronts
    // the device's flash and releases it blind — it produces no boot
    // evidence, so there are no checkpoints and nothing gates commit
    // beyond readback verification. Its flash still carries a full
    // layout; with no boot signal, the ladder is walked on verification
    // failures only.
    DeviceConfig {
        name: "cpld",
        reset_signal: 9,
        checkpoints: &[],
        commit_policy: CommitPolicy::None,
        slots: &[
            SlotDesc {
                id: SlotId(0),
                writable: true,
                bootable: true,
                role: None,
            },
            SlotDesc {
                id: SlotId(1),
                writable: true,
                bootable: true,
                role: None,
            },
            // Golden: recovery role and never written after provisioning.
            SlotDesc {
                id: SlotId(2),
                writable: false,
                bootable: true,
                role: Some(SlotRole::Recovery),
            },
        ],
        recovery_policy: RecoveryPolicy::Ladder,
    },
];

const _: () = fwmanager_api::config::validate(MANAGED_DEVICES);
