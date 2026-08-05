// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Mock board: a device table exercising every device archetype the
//! orchestrator manages. Not a real board — consumed by host tests and QEMU
//! runs until a hardware target declares its own table.

#![no_std]

use core::time::Duration;

use orchestrator_config::{BootCheckpoint, DeviceConfig, Slot, SlotId};

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
/// top to bottom, one at a time. This table is the authority — the
/// orchestrator's chain of trust is built from it, never beside it.
///
/// The mock board's reset controller addresses reset lines by plain index,
/// so the reset id type is `u8`.
pub const MANAGED_DEVICES: &[DeviceConfig<u8, MockSignal>] = &[
    // Direct-flash SPI device (BMC archetype): the eRoT fronts its flash.
    // Single checkpoint: it raises a boot-complete GPIO.
    DeviceConfig::new(
        "bmc",
        7,
        &[BootCheckpoint::new(
            "boot-complete",
            MockSignal::Gpio(12),
            Duration::from_secs(90),
        )],
        // Plain A/B: two equal writable slots, recovery falls back to the
        // other one. Real boards declare their own topology; a golden
        // slot is optional.
        &[
            Slot {
                id: SlotId(0),
                writable: true,
                bootable: true,
                role: None,
            },
            Slot {
                id: SlotId(1),
                writable: true,
                bootable: true,
                role: None,
            },
        ],
    ),
    // PLDM device (NIC archetype): self-updating, SPDM-capable. Two
    // checkpoints, exercising the multi-checkpoint path: transport up
    // first, then proof the workload is alive.
    DeviceConfig::new(
        "nic",
        3,
        &[
            BootCheckpoint::new("mctp-ready", MockSignal::MctpReady, Duration::from_secs(20)),
            BootCheckpoint::new("heartbeat", MockSignal::Heartbeat, Duration::from_secs(10)),
        ],
        // Self-updating device: it owns its boot selection, the eRoT never
        // sees its slot topology. No local ladder rungs, so recovery can
        // only escalate.
        &[],
    ),
];

/// Board-local checks the schema constructors cannot do — they know the
/// schema's shape, not this board's meanings. Const-fence pattern: a bad
/// signal fails the build.
const fn validate_signals(devices: &[DeviceConfig<u8, MockSignal>]) {
    let mut i = 0;
    while i < devices.len() {
        let checkpoints = devices[i].checkpoints();
        let mut c = 0;
        while c < checkpoints.len() {
            if let MockSignal::Gpio(line) = *checkpoints[c].signal() {
                // The mock ready-line bank packs 32 lines, SGPIO-style.
                assert!(line < 32, "gpio signal names a line outside the bank");
            }
            c += 1;
        }
        i += 1;
    }
}

const _: () = validate_signals(MANAGED_DEVICES);
