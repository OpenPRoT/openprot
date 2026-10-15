// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! The `FdOps` methods both QEMU firmware devices answer the same way.
//!
//! A scenario's firmware device differs from the others in what it does
//! with an update: one gates every step on an IPC decision from the RoT,
//! the other answers locally. What it says about itself does not differ,
//! and neither does how it counts a download.
//!
//! These are macros rather than functions because the bodies read the
//! app's own constants and fields: a device keeps its own UUID, versions
//! and byte counter, and the macro expands where those names are in
//! scope.

#![no_std]

/// `get_device_identifiers` and `get_firmware_parms`: what the device
/// says it is.
///
/// Expands inside an `impl FdOps for ...` block. Reads `DEVICE_UUID`,
/// `ACTIVE_COMP_VERSION`, `ACTIVE_IMAGE_SET_VERSION` and
/// `COMP_IDENTIFIER` from the app, so each device keeps its own identity
/// while the shape of the answer is written once.
#[macro_export]
macro_rules! fd_ops_identity {
    () => {
        fn get_device_identifiers(
            &self,
            device_identifiers: &mut [Descriptor],
        ) -> Result<usize, FdOpsError> {
            let uuid = Descriptor::new(DescriptorType::Uuid, &DEVICE_UUID)
                .map_err(|_| FdOpsError::DeviceIdentifiersError)?;
            *device_identifiers
                .first_mut()
                .ok_or(FdOpsError::DeviceIdentifiersError)? = uuid;
            Ok(1)
        }

        fn get_firmware_parms(
            &self,
            firmware_params: &mut FirmwareParameters,
        ) -> Result<(), FdOpsError> {
            let comp_version = PldmFirmwareString::new("ASCII", ACTIVE_COMP_VERSION)
                .map_err(|_| FdOpsError::FirmwareParametersError)?;
            let image_set_version = PldmFirmwareString::new("ASCII", ACTIVE_IMAGE_SET_VERSION)
                .map_err(|_| FdOpsError::FirmwareParametersError)?;

            // Self-contained: the device applies the image itself, so the
            // agent's ActivateFirmware is all that is needed to finish.
            let mut activation = ComponentActivationMethods(0);
            activation.set_self_contained(true);

            let component = ComponentParameterEntry::new(
                ComponentClassification::Firmware,
                COMP_IDENTIFIER,
                0, // comp_classification_index
                &PldmFirmwareVersion::new(0, &comp_version, None),
                &PldmFirmwareVersion::default(),
                activation,
                FirmwareDeviceCapability(0),
            );

            *firmware_params = FirmwareParameters::new(
                FirmwareDeviceCapability(0),
                1, // comp_count
                &image_set_version,
                &PldmFirmwareString::default(),
                &[component],
            );
            Ok(())
        }
    };
}

/// `query_download_progress`, `get_xfer_size` and `is_download_complete`:
/// how far the transfer has come.
///
/// Expands inside an `impl FdOps for ...` block. Reads the app's
/// `IMAGE_SIZE` and `MAX_TRANSFER_SIZE` and the device's own
/// `bytes_received` counter.
#[macro_export]
macro_rules! fd_ops_download_progress {
    () => {
        fn query_download_progress(
            &self,
            _component: &FirmwareComponent,
            progress_percent: &mut ProgressPercent,
        ) -> Result<(), FdOpsError> {
            let pct = (self.bytes_received.get() * 100 / IMAGE_SIZE) as u8;
            progress_percent
                .set_value(pct.min(100))
                .map_err(|_| FdOpsError::FwDownloadError)?;
            Ok(())
        }

        fn get_xfer_size(&self, ua_transfer_size: usize) -> Result<usize, FdOpsError> {
            Ok(ua_transfer_size.min(MAX_TRANSFER_SIZE))
        }

        fn is_download_complete(&self, _component: &FirmwareComponent) -> bool {
            self.bytes_received.get() >= IMAGE_SIZE
        }
    };
}

/// `handle_pending_component` and `get_non_functional_component_info`:
/// the two a scenario device has nothing to say about.
///
/// Expands inside an `impl FdOps for ...` block. Nothing here has a
/// pending image to activate, and the new image goes to staging rather
/// than over the running one, so the component keeps working whatever
/// happens to the update.
#[macro_export]
macro_rules! fd_ops_nothing_pending {
    () => {
        fn handle_pending_component(
            &self,
            _component: &PendingComponent,
            _fw_params: &FirmwareParameters,
        ) -> Result<PendingComponentResult, FdOpsError> {
            Ok(PendingComponentResult::NotPermitted)
        }

        fn get_non_functional_component_info(
            &self,
        ) -> Result<
            (
                NonFunctioningComponentIndication,
                NonFunctioningComponentBitmap,
            ),
            FdOpsError,
        > {
            Ok((
                NonFunctioningComponentIndication::ComponentsFunctioning,
                NonFunctioningComponentBitmap::new(0),
            ))
        }
    };
}
