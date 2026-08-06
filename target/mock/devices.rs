// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Mock board: a device table exercising every device archetype the
//! orchestrator manages. Not a real board — consumed by host tests and QEMU
//! runs until a hardware target declares its own table.

#![no_std]

use core::time::Duration;

use orchestrator_config::{BootCheckpoint, DeviceConfig};

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
    },
];

/// Board-local checks the generic `validate` cannot do — it knows the
/// schema's shape, not this board's meanings. Same const-fence pattern:
/// a bad signal fails the build.
const fn validate_signals(devices: &[DeviceConfig<u8, MockSignal>]) {
    let mut i = 0;
    while i < devices.len() {
        let mut c = 0;
        while c < devices[i].checkpoints.len() {
            if let MockSignal::Gpio(line) = devices[i].checkpoints[c].signal {
                // The mock ready-line bank packs 32 lines, SGPIO-style.
                assert!(line < 32, "gpio signal names a line outside the bank");
            }
            c += 1;
        }
        i += 1;
    }
}

const _: () = {
    orchestrator_config::validate(MANAGED_DEVICES);
    validate_signals(MANAGED_DEVICES);
};
