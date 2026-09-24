// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Mock board: a device table exercising every device archetype the
//! orchestrator manages. Not a real board — consumed by host tests and QEMU
//! runs until a hardware target declares its own table.

#![no_std]

use core::time::Duration;

use orchestrator_config::{
    assert_retry_reaches_every_image, BootCheckpoint, DeviceConfig, Golden, ImageLayout, Region,
    Slot, SlotId,
};

/// The mock board's boot-probe vocabulary. The schema carries these
/// opaquely; only this board's `EvidenceReader` gives them meaning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MockProbe {
    /// A boot-complete GPIO line, by index.
    Gpio(u8),
    /// The device's MCTP endpoint answers as ready.
    MctpReady,
    /// The device sends a heartbeat message (latched; the reset path
    /// clears it).
    Heartbeat,
}

/// The BMC's images: two 2 MiB slots at the bottom of its firmware
/// partition, the golden image above them.
const BMC_LAYOUT: ImageLayout = ImageLayout::new(
    const {
        &[
            Slot::new(SlotId(0), Region::new(0x00_0000, 0x20_0000)),
            Slot::new(SlotId(1), Region::new(0x20_0000, 0x20_0000)),
        ]
    },
    Golden::new(Region::new(0x40_0000, 0x20_0000)),
);

/// Declaration order is the boot order: the orchestrator releases devices
/// top to bottom, one at a time. This table is the authority — the
/// orchestrator's chain of trust is built from it, never beside it.
///
/// The mock board's reset controller addresses reset lines by plain index,
/// so the reset id type is `u8`.
pub const MANAGED_DEVICES: &[DeviceConfig<u8, MockProbe>] = &[
    // Direct-flash SPI device (BMC archetype): the eRoT fronts its flash.
    // Single checkpoint: it raises a boot-complete GPIO.
    DeviceConfig::new(
        "bmc",
        7,
        &[BootCheckpoint::new(
            "boot-complete",
            MockProbe::Gpio(12),
            Duration::from_secs(90),
        )],
        // A/B plus the golden image, in the flash the eRoT owns. Offsets
        // are counted from the start of the BMC's flash area; real
        // boards declare their own.
        Some(BMC_LAYOUT),
    ),
    // PLDM device (NIC archetype): self-updating, SPDM-capable. Two
    // checkpoints, exercising the multi-checkpoint path: transport up
    // first, then proof the workload is alive.
    DeviceConfig::new(
        "nic",
        3,
        &[
            BootCheckpoint::new("mctp-ready", MockProbe::MctpReady, Duration::from_secs(20)),
            BootCheckpoint::new("heartbeat", MockProbe::Heartbeat, Duration::from_secs(10)),
        ],
        // Self-updating device: it owns its images and its boot selection,
        // so the eRoT addresses no byte range for it and declares no
        // layout.
        None,
    ),
];

/// Board-local checks the schema constructors cannot do — they know the
/// schema's shape, not this board's meanings. Const-fence pattern: a bad
/// probe fails the build.
const fn validate_probes(devices: &[DeviceConfig<u8, MockProbe>]) {
    let mut i = 0;
    while i < devices.len() {
        let checkpoints = devices[i].checkpoints();
        let mut c = 0;
        while c < checkpoints.len() {
            if let MockProbe::Gpio(line) = *checkpoints[c].probe() {
                // The mock ready-line bank packs 32 lines, SGPIO-style.
                assert!(line < 32, "gpio probe names a line outside the bank");
            }
            c += 1;
        }
        i += 1;
    }
}

const _: () = validate_probes(MANAGED_DEVICES);

/// How many times the orchestrator restores a component before it gives up.
/// Four, because the BMC has three images and the attempt that restores the
/// last one is never booted, so three would stop one boot short of the
/// golden image.
pub const MAX_RETRY: u8 = 4;

const _: () = assert_retry_reaches_every_image(MAX_RETRY, MANAGED_DEVICES);
