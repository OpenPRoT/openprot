// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! What both QEMU Firmware Devices answer the same way.
//!
//! The two devices differ in what they do with an update. One gates every
//! step on an IPC decision from the RoT, the other decides locally. What a
//! device says about itself, and how it counts a download, are the same
//! either way.
//!
//! The device's own constants come in as arguments: the UUID, the
//! component id, the versions, the image size and the byte count. So each
//! device keeps its own identity and this crate checks the shared part by
//! itself.

#![cfg_attr(not(test), no_std)]

use pldm_common::message::firmware_update::get_fw_params::FirmwareParameters;
use pldm_common::message::firmware_update::get_status::ProgressPercent;
use pldm_common::protocol::firmware_update::{
    ComponentActivationMethods, ComponentClassification, ComponentParameterEntry, Descriptor,
    DescriptorType, FirmwareDeviceCapability, PldmFirmwareString, PldmFirmwareVersion,
};
use pldm_interface::firmware_device::fd_ops::FdOpsError;

/// How the version strings are encoded, in the name `PldmFirmwareString`
/// takes.
const VERSION_STRING_TYPE: &str = "ASCII";

/// Each device advertises one component, so the parameter table has one
/// entry and it is entry 0.
const COMPONENT_COUNT: u16 = 1;
const COMPONENT_CLASSIFICATION_INDEX: u8 = 0;

/// The device writes one descriptor, its UUID.
const IDENTIFIER_COUNT: usize = 1;

/// DSP0267 makes the component comparison stamp optional, and zero says it
/// is unused. A device with nothing to compare images by reports zero.
const COMPARISON_STAMP_UNUSED: u32 = 0;

/// The device advertises no capabilities during an update.
const NO_CAPABILITIES: FirmwareDeviceCapability = FirmwareDeviceCapability(0);

/// No activation method set yet. `set_self_contained` adds the one these
/// devices use.
const NO_ACTIVATION_METHODS: ComponentActivationMethods = ComponentActivationMethods(0);

/// A finished transfer, as the percentage `ProgressPercent` carries.
const COMPLETE_PERCENT: usize = 100;

/// `GetDeviceIdentifiers`: the device's UUID, in the first slot.
pub fn device_identifiers(
    device_identifiers: &mut [Descriptor],
    uuid: &[u8; 16],
) -> Result<usize, FdOpsError> {
    let uuid = Descriptor::new(DescriptorType::Uuid, uuid)
        .map_err(|_| FdOpsError::DeviceIdentifiersError)?;
    *device_identifiers
        .first_mut()
        .ok_or(FdOpsError::DeviceIdentifiersError)? = uuid;
    Ok(IDENTIFIER_COUNT)
}

/// `GetFirmwareParameters`: one firmware component, with self-contained
/// activation.
///
/// The activation method tells the Update Agent what has to happen after
/// an update for the new image to run: DSP0267's options range from
/// nothing up to a host reboot or a power cycle. Self-contained says the
/// ActivateFirmware command is the whole of it: the device switches to
/// the new image on its own, and the Update Agent has nothing further to
/// do.
///
/// `comp_version` is the component's own version and `image_set_version`
/// is the set it belongs to. Both are `&str`, so passing them the other
/// way round still compiles and the device then describes itself wrongly.
pub fn firmware_parameters(
    firmware_params: &mut FirmwareParameters,
    comp_identifier: u16,
    comp_version: &str,
    image_set_version: &str,
) -> Result<(), FdOpsError> {
    let comp_version = PldmFirmwareString::new(VERSION_STRING_TYPE, comp_version)
        .map_err(|_| FdOpsError::FirmwareParametersError)?;
    let image_set_version = PldmFirmwareString::new(VERSION_STRING_TYPE, image_set_version)
        .map_err(|_| FdOpsError::FirmwareParametersError)?;

    let mut activation = NO_ACTIVATION_METHODS;
    activation.set_self_contained(true);

    let component = ComponentParameterEntry::new(
        ComponentClassification::Firmware,
        comp_identifier,
        COMPONENT_CLASSIFICATION_INDEX,
        &PldmFirmwareVersion::new(COMPARISON_STAMP_UNUSED, &comp_version, None),
        &PldmFirmwareVersion::default(),
        activation,
        NO_CAPABILITIES,
    );

    *firmware_params = FirmwareParameters::new(
        NO_CAPABILITIES,
        COMPONENT_COUNT,
        &image_set_version,
        &PldmFirmwareString::default(),
        &[component],
    );
    Ok(())
}

/// `QueryDownloadProgress`: how much of the image has arrived, as a
/// percentage capped at 100.
///
/// Integer division floors, so a transfer one byte short of the image
/// reports 99 and never a premature 100. More than a whole image is an
/// honest count (a retransmitted window arrives twice) and caps at 100.
/// An image of no bytes, or a byte count large enough to overflow the
/// multiply, is an error: the first has no progress to report and the
/// second means the counter is corrupt.
pub fn download_progress(
    received: usize,
    image_size: usize,
    progress_percent: &mut ProgressPercent,
) -> Result<(), FdOpsError> {
    // Plain `*` panics on overflow in a debug build and wraps to a
    // small garbage value in a release one, and plain `/` panics on a
    // zero divisor. The checked forms answer `None` for both, and
    // `ok_or` turns that into the error: a byte count too large to
    // multiply means the counter is corrupt, and an invented percentage
    // would cover that up. The agent sees a failed completion and the
    // device keeps running, which a panic here would not allow.
    let percent = received
        .checked_mul(COMPLETE_PERCENT)
        .and_then(|scaled| scaled.checked_div(image_size))
        .ok_or(FdOpsError::FwDownloadError)?;
    progress_percent
        .set_value(percent.min(COMPLETE_PERCENT) as u8)
        .map_err(|_| FdOpsError::FwDownloadError)
}

#[cfg(test)]
mod tests {
    use super::*;

    const UUID: [u8; 16] = [7; 16];

    #[test]
    fn the_uuid_goes_in_the_first_slot() {
        let mut slots = [Descriptor::new_empty(); 2];

        assert!(matches!(device_identifiers(&mut slots, &UUID), Ok(1)));

        assert_eq!(
            slots[0],
            Descriptor::new(DescriptorType::Uuid, &UUID).unwrap()
        );
        assert_eq!(slots[1], Descriptor::new_empty(), "and only the first");
    }

    // The trait hands the callback a caller-sized buffer. Writing through
    // `first_mut` makes a buffer with no room an error; indexing slot 0
    // would make it a panic in the device's request handler.
    #[test]
    fn an_empty_descriptor_buffer_is_refused_not_a_panic() {
        assert!(matches!(
            device_identifiers(&mut [], &UUID),
            Err(FdOpsError::DeviceIdentifiersError)
        ));
    }

    #[test]
    fn the_parameters_describe_one_component() {
        let mut params = FirmwareParameters::default();

        firmware_parameters(&mut params, 0x0102, "v1", "set-v1").unwrap();

        let count = params.params_fixed.comp_count;
        assert_eq!(count, 1, "the table has the one entry");
        assert!(params.active_comp_image_set_ver_str.starts_with(b"set-v1"));
    }

    #[test]
    fn a_version_string_that_does_not_fit_is_an_error() {
        // Longer than PLDM_FWUP_IMAGE_SET_VER_STR_MAX_LEN, which
        // `PldmFirmwareString::new` refuses.
        let too_long = "x".repeat(256);

        assert!(matches!(
            firmware_parameters(&mut FirmwareParameters::default(), 1, &too_long, "set"),
            Err(FdOpsError::FirmwareParametersError)
        ));
    }

    #[test]
    fn progress_is_the_share_received() {
        let mut progress = ProgressPercent::default();

        download_progress(256, 1024, &mut progress).unwrap();

        assert_eq!(progress.value(), 25);
    }

    // `is_download_complete` keys on the byte count reaching the image
    // size, so a transfer that is one byte short must not report a
    // hundred: an agent reading that would call a partial image done.
    #[test]
    fn a_transfer_one_byte_short_is_not_a_whole_image() {
        let mut progress = ProgressPercent::default();

        download_progress(1023, 1024, &mut progress).unwrap();

        assert_eq!(progress.value(), 99);
    }

    #[test]
    fn progress_stops_at_a_whole_image() {
        let mut progress = ProgressPercent::default();

        download_progress(4096, 1024, &mut progress).unwrap();

        assert_eq!(progress.value(), 100);
    }

    // A count this size cannot come from summing chunk lengths, so it is
    // a corrupt counter, and an invented percentage would cover that up.
    #[test]
    fn a_byte_count_that_overflows_the_multiply_is_an_error() {
        assert!(matches!(
            download_progress(usize::MAX, 1024, &mut ProgressPercent::default()),
            Err(FdOpsError::FwDownloadError)
        ));
    }

    #[test]
    fn an_image_of_no_bytes_has_no_progress() {
        assert!(matches!(
            download_progress(0, 0, &mut ProgressPercent::default()),
            Err(FdOpsError::FwDownloadError)
        ));
    }
}
