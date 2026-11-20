// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! The firmware device operations of the PLDM firmware update test.
//!
//! An update has two components: the SoC manifest and the MCU runtime image.
//! The device stages both in a part of the emulator's external test SRAM,
//! which the MCU reads and writes as memory and Caliptra reads over AXI:
//!
//! ```text
//! offset 0          the MCU runtime image, at its staging address
//! MANIFEST_OFFSET   a SET_AUTH_MANIFEST request: its header, then the manifest
//! ```
//!
//! The device verifies the manifest with Caliptra (`VERIFY_AUTH_MANIFEST`,
//! which checks its signatures) and the image against the digest that the
//! staged manifest gives for the MCU image. The update agent therefore sends
//! the manifest first.
//!
//! [`UpdateOps::switch_image`] runs after the device has answered
//! `ActivateFirmware`. It makes the staged manifest Caliptra's manifest and
//! asks Caliptra to activate the staged image. Caliptra then requests an MCU
//! reset, copies the image to MCU SRAM, authorizes it and releases the MCU.
//!
//! The staging memory exists only on the emulator.

use core::cell::Cell;
use core::mem::{offset_of, size_of};
use core::ptr::with_exposed_provenance_mut;

use caliptra_auth_man_types::{
    AuthManifestImageMetadata, AuthManifestPreamble, AUTH_MANIFEST_IMAGE_METADATA_MAX_COUNT,
};
use openprot_hal_blocking::digest::owned::{DigestInit, DigestOp};
use openprot_hal_blocking::digest::Sha2_384;
use pldm_common::message::firmware_update::apply_complete::ApplyResult;
use pldm_common::message::firmware_update::get_fw_params::FirmwareParameters;
use pldm_common::message::firmware_update::get_status::ProgressPercent;
use pldm_common::message::firmware_update::request_fw_data::MAX_TRANSFER_SIZE;
use pldm_common::message::firmware_update::transfer_complete::TransferResult;
use pldm_common::message::firmware_update::verify_complete::VerifyResult;
use pldm_common::protocol::base::PldmBaseCompletionCode;
use pldm_common::protocol::firmware_update::{
    ComponentActivationMethods, ComponentClassification, ComponentParameterEntry,
    ComponentResponseCode, Descriptor, DescriptorType, FirmwareDeviceCapability,
    FwUpdateCompletionCode, PldmFdTime, PldmFirmwareString, PldmFirmwareVersion,
};
use pldm_common::util::fw_component::FirmwareComponent;
use pldm_interface::firmware_device::fd_ops::{ComponentOperation, FdOps, FdOpsError};
use pldm_update_ids::{
    ACTIVE_COMPARISON_STAMP, ACTIVE_VERSION, DEVICE_UUID, MANIFEST_CLASSIFICATION,
    MANIFEST_IDENTIFIER, RUNTIME_CLASSIFICATION, RUNTIME_IDENTIFIER,
};
use userspace::time::{self, Clock, Duration, Instant, SystemClock};
use veer_mailbox::{
    auth_manifest_header, AuthManifestCommand, AxiAddr, Mailbox, AUTH_MANIFEST_HEADER_SIZE,
    MAX_AUTH_MANIFEST_SIZE,
};
use veer_mci::{Mci, Notifications};
use veer_sha::CaliptraSha;

/// Address of the staging memory for the MCU: the upper half of the
/// emulator's external test SRAM, which is at 0x8000_0000. The MCU ROM hides
/// the first 64 KiB of that SRAM from the MCU. `system.json5.tmpl` maps the
/// staging memory for the process.
pub const STAGING_BASE: usize = 0x8008_0000;

/// Address of the staging memory for Caliptra, on its AXI bus, where the
/// external test SRAM is at 0xb00c_0000. The SoC manifest of the update
/// gives this address as the staging address of the MCU image.
const STAGING_AXI_BASE: AxiAddr = AxiAddr::new(0xb014_0000);

/// Size in bytes of the staging memory.
pub const STAGING_SIZE: usize = 0x8_0000;

/// The largest MCU runtime image that the device stages.
const IMAGE_MAX: usize = 0x4_0000;

/// Offset of the manifest request in the staging memory.
const MANIFEST_OFFSET: usize = IMAGE_MAX;

const _: () =
    assert!(MANIFEST_OFFSET + AUTH_MANIFEST_HEADER_SIZE + MAX_AUTH_MANIFEST_SIZE < STAGING_SIZE);

/// Identifier of the MCU runtime image in a SoC manifest.
const MCU_IMAGE_ID: u32 = 2;

/// Size in bytes of a SHA-384 digest.
const DIGEST_SIZE: usize = 48;

/// Number of image bytes that one SHA command hashes.
const HASH_PART: usize = 1024;

/// Seconds that the device reports for the activation.
const ACTIVATION_SECONDS: u16 = 5;

/// How long the device waits for Caliptra's reset request.
const RESET_REQUEST_TIMEOUT_MS: u64 = 10_000;

/// A component of the update.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Manifest,
    Runtime,
}

impl Kind {
    fn of(component: &FirmwareComponent) -> Option<Self> {
        match (component.comp_classification, component.comp_identifier) {
            (MANIFEST_CLASSIFICATION, MANIFEST_IDENTIFIER) => Some(Self::Manifest),
            (RUNTIME_CLASSIFICATION, RUNTIME_IDENTIFIER) => Some(Self::Runtime),
            _ => None,
        }
    }

    /// Offset of the component in the staging memory.
    const fn offset(self) -> usize {
        match self {
            Self::Manifest => MANIFEST_OFFSET + AUTH_MANIFEST_HEADER_SIZE,
            Self::Runtime => 0,
        }
    }

    /// The largest component that the device stages.
    const fn max_size(self) -> usize {
        match self {
            Self::Manifest => MAX_AUTH_MANIFEST_SIZE,
            Self::Runtime => IMAGE_MAX,
        }
    }
}

/// Progress of the update of one component.
#[derive(Clone, Copy, Default)]
struct Staged {
    /// Size of the component in bytes, from `UpdateComponent`.
    size: usize,
    /// Number of bytes received, from offset 0.
    received: usize,
    verified: bool,
    applied: bool,
}

/// Why [`UpdateOps::switch_image`] returned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwitchError {
    /// Caliptra did not accept the staged manifest.
    Manifest,
    /// Caliptra did not accept `ACTIVATE_FIRMWARE`.
    Activation,
    /// Caliptra did not request the MCU reset.
    NoResetRequest,
    /// The MCU was not reset.
    NoReset,
}

/// The firmware device operations.
pub struct UpdateOps<'m> {
    mailbox: &'m Mailbox,
    manifest: Cell<Staged>,
    runtime: Cell<Staged>,
    activation_requested: Cell<bool>,
}

/// Milliseconds since the kernel started.
fn now_ms() -> u64 {
    SystemClock::now().ticks().saturating_mul(1000) / SystemClock::TICKS_PER_SEC
}

fn sleep_ms(ms: u64) {
    let deadline = SystemClock::now()
        .checked_add_duration(Duration::from_millis(ms))
        .unwrap_or(Instant::MAX);
    let _ = time::sleep_until(deadline);
}

/// Reads `buf.len()` bytes at `offset` of the staging memory.
fn read_staging(offset: usize, buf: &mut [u8]) -> Option<()> {
    let end = offset.checked_add(buf.len())?;
    if end > STAGING_SIZE {
        return None;
    }
    let base = with_exposed_provenance_mut::<u8>(STAGING_BASE);
    for (index, byte) in buf.iter_mut().enumerate() {
        // SAFETY: the system config maps the staging memory for this
        // process, and the range was checked against its size.
        *byte = unsafe { base.add(offset + index).read_volatile() };
    }
    Some(())
}

/// Writes `data` at `offset` of the staging memory.
fn write_staging(offset: usize, data: &[u8]) -> Option<()> {
    let end = offset.checked_add(data.len())?;
    if end > STAGING_SIZE {
        return None;
    }
    let base = with_exposed_provenance_mut::<u8>(STAGING_BASE);
    for (index, &byte) in data.iter().enumerate() {
        // SAFETY: as in `read_staging`.
        unsafe { base.add(offset + index).write_volatile(byte) };
    }
    Some(())
}

fn read_staging_u32(offset: usize) -> Option<u32> {
    let mut bytes = [0u8; 4];
    read_staging(offset, &mut bytes)?;
    Some(u32::from_le_bytes(bytes))
}

impl<'m> UpdateOps<'m> {
    pub fn new(mailbox: &'m Mailbox) -> Self {
        Self {
            mailbox,
            manifest: Cell::new(Staged::default()),
            runtime: Cell::new(Staged::default()),
            activation_requested: Cell::new(false),
        }
    }

    /// Whether the update agent has asked the device to activate the update.
    pub fn activation_requested(&self) -> bool {
        self.activation_requested.get()
    }

    fn staged(&self, kind: Kind) -> &Cell<Staged> {
        match kind {
            Kind::Manifest => &self.manifest,
            Kind::Runtime => &self.runtime,
        }
    }

    /// Sends `command` to Caliptra for the staged manifest.
    fn manifest_command(&self, command: AuthManifestCommand) -> Option<()> {
        let size = self.manifest.get().size;
        let manifest = Kind::Manifest.offset();
        // Caliptra reads the request as words, so the bytes after the
        // manifest up to a multiple of 4 are part of it.
        let padding = [0u8; 3];
        write_staging(
            manifest + size,
            padding.get(..size.next_multiple_of(4) - size)?,
        )?;
        let header = auth_manifest_header(command, size, |offset, buf| {
            read_staging(manifest + offset, buf).is_some()
        })?;
        write_staging(MANIFEST_OFFSET, &header)?;
        let request = STAGING_AXI_BASE.checked_add(MANIFEST_OFFSET as u64)?;
        self.mailbox.auth_manifest(command, request, size).ok()
    }

    /// Returns the digest that the staged manifest gives for the MCU image.
    fn manifest_image_digest(&self) -> Option<[u8; DIGEST_SIZE]> {
        let manifest = Kind::Manifest.offset();
        let size = self.manifest.get().size;
        // The image metadata follows the preamble: a count, then the
        // entries.
        let count_offset = size_of::<AuthManifestPreamble>();
        let count = read_staging_u32(manifest + count_offset)? as usize;
        for index in 0..count.min(AUTH_MANIFEST_IMAGE_METADATA_MAX_COUNT) {
            let entry = count_offset + 4 + index * size_of::<AuthManifestImageMetadata>();
            if entry + size_of::<AuthManifestImageMetadata>() > size {
                return None;
            }
            let id =
                read_staging_u32(manifest + entry + offset_of!(AuthManifestImageMetadata, fw_id))?;
            if id == MCU_IMAGE_ID {
                let mut digest = [0u8; DIGEST_SIZE];
                read_staging(
                    manifest + entry + offset_of!(AuthManifestImageMetadata, digest),
                    &mut digest,
                )?;
                return Some(digest);
            }
        }
        None
    }

    /// Returns the SHA-384 digest of the staged image.
    fn image_digest(&self) -> Option<[u8; DIGEST_SIZE]> {
        let size = self.runtime.get().size;
        let mut op = CaliptraSha::new(self.mailbox).init(Sha2_384).ok()?;
        let mut buf = [0u8; HASH_PART];
        let mut offset = 0usize;
        while offset < size {
            let part = buf.get_mut(..(size - offset).min(HASH_PART))?;
            read_staging(Kind::Runtime.offset() + offset, part)?;
            op = op.update(part).ok()?;
            offset += part.len();
        }
        let (digest, _) = op.finalize().ok()?;
        Some(zerocopy::transmute!(digest.value))
    }

    fn verify_manifest(&self) -> VerifyResult {
        match self.manifest_command(AuthManifestCommand::Verify) {
            Some(()) => {
                pw_log::info!("pldm fd: Caliptra verified the staged SoC manifest");
                VerifyResult::VerifySuccess
            }
            None => {
                pw_log::error!("pldm fd: Caliptra rejected the staged SoC manifest");
                VerifyResult::VerifyErrorVerificationFailure
            }
        }
    }

    fn verify_runtime(&self) -> VerifyResult {
        if !self.manifest.get().verified {
            pw_log::error!("pldm fd: no verified SoC manifest for the image");
            return VerifyResult::VerifyFailedFdSecurityChecks;
        }
        let (Some(expected), Some(digest)) = (self.manifest_image_digest(), self.image_digest())
        else {
            pw_log::error!("pldm fd: no digest for the staged image");
            return VerifyResult::VerifyGenericError;
        };
        if digest == expected {
            pw_log::info!("pldm fd: the staged image matches the SoC manifest");
            VerifyResult::VerifySuccess
        } else {
            pw_log::error!("pldm fd: the staged image does not match the SoC manifest");
            VerifyResult::VerifyErrorVerificationFailure
        }
    }

    /// Replaces the running image with the staged one. Returns only if that
    /// fails.
    pub fn switch_image(&self, mci: &mut Mci) -> SwitchError {
        if self.manifest_command(AuthManifestCommand::Set).is_none() {
            return SwitchError::Manifest;
        }
        pw_log::info!("pldm fd: Caliptra uses the staged SoC manifest");
        let size = self.runtime.get().size as u32;
        if self.mailbox.start_mcu_activation(size).is_err() {
            return SwitchError::Activation;
        }
        pw_log::info!("pldm fd: asked Caliptra to activate {} bytes", size);

        let start = now_ms();
        while !mci
            .pending_notifications()
            .contains(Notifications::CPTRA_MCU_RESET_REQUEST)
        {
            if now_ms().saturating_sub(start) > RESET_REQUEST_TIMEOUT_MS {
                return SwitchError::NoResetRequest;
            }
            sleep_ms(1);
        }
        pw_log::info!("pldm fd: Caliptra requested the MCU reset");
        mci.acknowledge_notifications(Notifications::CPTRA_MCU_RESET_REQUEST);
        mci.request_mcu_reset();
        // The MCI halts the MCU and resets it.
        sleep_ms(RESET_REQUEST_TIMEOUT_MS);
        SwitchError::NoReset
    }
}

/// Returns `s` as a PLDM version string.
fn version_string(s: &str) -> Result<PldmFirmwareString, FdOpsError> {
    PldmFirmwareString::new("ASCII", s).map_err(|_| FdOpsError::FirmwareParametersError)
}

impl FdOps for UpdateOps<'_> {
    fn get_device_identifiers(
        &self,
        device_identifiers: &mut [Descriptor],
    ) -> Result<usize, FdOpsError> {
        let slot = device_identifiers
            .first_mut()
            .ok_or(FdOpsError::DeviceIdentifiersError)?;
        *slot = Descriptor::new(DescriptorType::Uuid, &DEVICE_UUID)
            .map_err(|_| FdOpsError::DeviceIdentifiersError)?;
        Ok(1)
    }

    fn get_firmware_parms(
        &self,
        firmware_params: &mut FirmwareParameters,
    ) -> Result<(), FdOpsError> {
        let active = version_string(ACTIVE_VERSION)?;
        let none = version_string("")?;
        let active_version = PldmFirmwareVersion::new(ACTIVE_COMPARISON_STAMP, &active, None);
        let pending_version = PldmFirmwareVersion::new(0, &none, None);
        let mut methods = ComponentActivationMethods(0);
        methods.set_self_contained(true);
        let capabilities = FirmwareDeviceCapability(0);
        let table = [
            ComponentParameterEntry::new(
                ComponentClassification::Other,
                MANIFEST_IDENTIFIER,
                0,
                &active_version,
                &pending_version,
                methods,
                capabilities,
            ),
            ComponentParameterEntry::new(
                ComponentClassification::Firmware,
                RUNTIME_IDENTIFIER,
                0,
                &active_version,
                &pending_version,
                methods,
                capabilities,
            ),
        ];
        *firmware_params =
            FirmwareParameters::new(capabilities, table.len() as u16, &active, &none, &table);
        Ok(())
    }

    fn get_xfer_size(&self, ua_transfer_size: usize) -> Result<usize, FdOpsError> {
        Ok(ua_transfer_size.min(MAX_TRANSFER_SIZE))
    }

    fn handle_component(
        &self,
        component: &FirmwareComponent,
        fw_params: &FirmwareParameters,
        op: ComponentOperation,
    ) -> Result<ComponentResponseCode, FdOpsError> {
        let Some(kind) = Kind::of(component) else {
            return Ok(ComponentResponseCode::CompNotSupported);
        };
        let code = component.evaluate_update_eligibility(fw_params);
        if op == ComponentOperation::UpdateComponent
            && code == ComponentResponseCode::CompCanBeUpdated
        {
            let size = component.comp_image_size.unwrap_or(0) as usize;
            if size == 0 || size > kind.max_size() {
                return Ok(ComponentResponseCode::CompPrerequisitesNotMet);
            }
            self.staged(kind).set(Staged {
                size,
                ..Staged::default()
            });
        }
        Ok(code)
    }

    fn query_download_offset_and_length(
        &self,
        component: &FirmwareComponent,
    ) -> Result<(usize, usize), FdOpsError> {
        let kind = Kind::of(component).ok_or(FdOpsError::FwDownloadError)?;
        let staged = self.staged(kind).get();
        Ok((staged.received, staged.size.saturating_sub(staged.received)))
    }

    fn download_fw_data(
        &self,
        offset: usize,
        data: &[u8],
        component: &FirmwareComponent,
    ) -> Result<TransferResult, FdOpsError> {
        let kind = Kind::of(component).ok_or(FdOpsError::FwDownloadError)?;
        let mut staged = self.staged(kind).get();
        // A part can extend past the end of the component, as padding.
        let len = data.len().min(staged.size.saturating_sub(offset));
        let data = data.get(..len).ok_or(FdOpsError::FwDownloadError)?;
        write_staging(kind.offset() + offset, data).ok_or(FdOpsError::FwDownloadError)?;
        staged.received = staged.received.max(offset + len);
        self.staged(kind).set(staged);
        Ok(TransferResult::TransferSuccess)
    }

    fn is_download_complete(&self, component: &FirmwareComponent) -> bool {
        Kind::of(component).is_some_and(|kind| {
            let staged = self.staged(kind).get();
            staged.received >= staged.size
        })
    }

    fn query_download_progress(
        &self,
        component: &FirmwareComponent,
        progress_percent: &mut ProgressPercent,
    ) -> Result<(), FdOpsError> {
        let kind = Kind::of(component).ok_or(FdOpsError::FwDownloadError)?;
        let staged = self.staged(kind).get();
        let percent = (staged.received.saturating_mul(100) / staged.size.max(1)).min(100) as u8;
        progress_percent
            .set_value(percent)
            .map_err(|_| FdOpsError::FwDownloadError)
    }

    fn verify(
        &self,
        component: &FirmwareComponent,
        _progress_percent: &mut ProgressPercent,
    ) -> Result<VerifyResult, FdOpsError> {
        let kind = Kind::of(component).ok_or(FdOpsError::VerifyError)?;
        let result = match kind {
            Kind::Manifest => self.verify_manifest(),
            Kind::Runtime => self.verify_runtime(),
        };
        let mut staged = self.staged(kind).get();
        staged.verified = result == VerifyResult::VerifySuccess;
        self.staged(kind).set(staged);
        Ok(result)
    }

    fn apply(
        &self,
        component: &FirmwareComponent,
        _progress_percent: &mut ProgressPercent,
    ) -> Result<ApplyResult, FdOpsError> {
        let kind = Kind::of(component).ok_or(FdOpsError::ApplyError)?;
        let mut staged = self.staged(kind).get();
        if !staged.verified {
            return Ok(ApplyResult::ApplyGenericError);
        }
        // The component is already staged where Caliptra reads it. The
        // activation applies both components.
        staged.applied = true;
        self.staged(kind).set(staged);
        Ok(ApplyResult::ApplySuccess)
    }

    fn activate(
        &self,
        _self_contained_activation: u8,
        estimated_time: &mut u16,
    ) -> Result<u8, FdOpsError> {
        if !(self.manifest.get().applied && self.runtime.get().applied) {
            return Ok(FwUpdateCompletionCode::IncompleteUpdate as u8);
        }
        *estimated_time = ACTIVATION_SECONDS;
        self.activation_requested.set(true);
        Ok(PldmBaseCompletionCode::Success as u8)
    }

    fn cancel_update_component(&self, component: &FirmwareComponent) -> Result<(), FdOpsError> {
        if let Some(kind) = Kind::of(component) {
            self.staged(kind).set(Staged::default());
        }
        Ok(())
    }

    fn now(&self) -> PldmFdTime {
        now_ms()
    }
}
