// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! What the firmware device of the PLDM firmware update test reports about
//! itself, shared by the firmware, the updated image and the host test.

#![no_std]

/// The UUID descriptor that the device returns to `QueryDeviceIdentifiers`.
pub const DEVICE_UUID: [u8; 16] = [
    0x4f, 0x50, 0x52, 0x54, 0x76, 0x65, 0x65, 0x72, 0x70, 0x6c, 0x64, 0x6d, 0x64, 0x65, 0x6d, 0x6f,
];

/// Component classification of the MCU runtime image (Firmware, DSP0267).
pub const RUNTIME_CLASSIFICATION: u16 = 0x000a;

/// Component identifier of the MCU runtime image.
pub const RUNTIME_IDENTIFIER: u16 = 0x0001;

/// Component classification of the SoC manifest (Other, DSP0267).
pub const MANIFEST_CLASSIFICATION: u16 = 0x0001;

/// Component identifier of the SoC manifest.
pub const MANIFEST_IDENTIFIER: u16 = 0x0002;

/// The version string of the running image and of its SoC manifest.
pub const ACTIVE_VERSION: &str = "1.0.0";

/// The comparison stamp of the running image and of its SoC manifest. The
/// device accepts a component with a higher stamp.
pub const ACTIVE_COMPARISON_STAMP: u32 = 1;

/// Logged by the firmware device once it listens for PLDM requests.
pub const READY: &str = "pldm fd: waiting for request";

/// Logged by the updated image when it runs after a hitless update.
pub const UPDATED: &str = "pldm update: updated image running";
