// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! PLDM Firmware Device app (card A, the RoT).
//!
//! Runs the real [`FirmwareDevice`] state machine against the update agent on
//! card B over MCTP/I2C2, streams the image it is handed into SPI NOR on FMC
//! chip select 1, and reads it back. The demo's claim is that the bytes the
//! update agent sent are the bytes now sitting in flash.

#![no_main]
#![no_std]

use core::cell::{Cell, RefCell};

use flash_backend::NoWaitBlocking;
use hal_flash::{BlockingFlash, Flash, FlashAddress};
use openprot_mctp_client_ipc::IpcMctpClient;
use openprot_pldm_service::firmware_device::{FirmwareDevice, RunTerminusResult};
use openprot_pldm_service::{MctpPldmTransport, PldmServiceError};
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
    PldmFirmwareString, PldmFirmwareVersion,
};
use pldm_common::util::fw_component::FirmwareComponent;
use pldm_interface::firmware_device::fd_ops::{ComponentOperation, FdOps, FdOpsError};
use pw_status::Error;
use userspace::{entry, syscall};
use util_error::ErrorCode;

use app_pldm_fd::handle;
use app_pldm_fd_regions::{take_mmaps, FmcRegs, Mmaps};

/// The FMC backend bound to this process's register mapping.
type Backend = flash_backend::Backend<FmcRegs>;

/// This card's EID, matching the MCTP server app underneath it.
const FD_EID: u8 = 8;
/// The update agent's EID on card B.
const UA_EID: u8 = 9;

/// Size of the demo image, in bytes. Must match the update agent's blob.
const IMAGE_SIZE: usize = 1024;

/// This device's UUID, the one descriptor QueryDeviceIdentifiers reports. The
/// update agent holds the same value and refuses to update anything else.
const DEVICE_UUID: [u8; 16] = [
    0x4f, 0x50, 0x52, 0x54, 0x00, 0x01, 0x40, 0x00, 0xa0, 0x00, 0xde, 0xad, 0xbe, 0xef, 0x00, 0x01,
];

/// The single updatable component this device advertises. The update agent
/// learns it from GetFirmwareParameters rather than assuming it.
const COMP_IDENTIFIER: u16 = 0x0001;

/// Version strings the device reports as running today. The update agent must
/// offer something newer for the component to be accepted.
const ACTIVE_COMP_VERSION: &str = "v0.9";
const ACTIVE_IMAGE_SET_VERSION: &str = "openprot-demo-v0.9";

/// Where the image lands on CS1. Scratch region, sector-aligned, and clobbered
/// without backup — persisting the image is the point of the test.
const IMAGE_BASE: u32 = 0x10_0000;

/// Readback chunk used by `verify`; one SPI NOR page.
const READBACK_CHUNK: usize = 256;

/// How long the FD waits for an update agent command before giving up and
/// reporting the result. This card is flashed first, so it must outlast card
/// B's UART upload (~50s for a 512 KiB image at 115200 baud) plus its boot.
const IDLE_TIMEOUT_MILLIS: u32 = 90_000;
/// How long each FD-initiated request waits for the update agent's reply.
const REQUESTER_TIMEOUT_MILLIS: u32 = 5_000;

const FD_BUF_SIZE: usize = 1024;

/// The byte the demo image carries at `offset`.
///
/// Offset-dependent so a duplicated, reordered, or truncated chunk fails
/// verification instead of slipping through. The update agent generates the
/// same sequence.
fn expected_byte(offset: usize) -> u8 {
    (offset % 251) as u8
}

/// Firmware-device operations for the demo: program the image into CS1 as it
/// arrives, then read it back.
///
/// `FdOps` takes `&self` throughout, so the flash handle lives behind a
/// `RefCell`.
struct DemoFdOps {
    flash: RefCell<BlockingFlash<Backend, NoWaitBlocking>>,
    bytes_received: Cell<usize>,
    corrupt: Cell<bool>,
    verified: Cell<bool>,
    activated: Cell<bool>,
}

impl DemoFdOps {
    fn new(flash: BlockingFlash<Backend, NoWaitBlocking>) -> Self {
        DemoFdOps {
            flash: RefCell::new(flash),
            bytes_received: Cell::new(0),
            corrupt: Cell::new(false),
            verified: Cell::new(false),
            activated: Cell::new(false),
        }
    }

    /// True once the whole image was read back out of flash intact.
    fn image_is_good(&self) -> bool {
        self.verified.get()
    }
}

impl FdOps for DemoFdOps {
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

        // Self-contained: the device applies the image itself, so the update
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

    fn get_xfer_size(&self, ua_transfer_size: usize) -> Result<usize, FdOpsError> {
        Ok(ua_transfer_size.min(MAX_TRANSFER_SIZE))
    }

    fn handle_component(
        &self,
        component: &FirmwareComponent,
        fw_params: &FirmwareParameters,
        _op: ComponentOperation,
    ) -> Result<ComponentResponseCode, FdOpsError> {
        // Matches the offered component against what this device advertised:
        // an unknown identifier, or a version no newer than the running one,
        // is refused.
        let code = component.evaluate_update_eligibility(fw_params);
        if code != ComponentResponseCode::CompCanBeUpdated {
            pw_log::error!("FD: component refused, code {}", code as u32);
        }
        Ok(code)
    }

    fn query_download_offset_and_length(
        &self,
        _component: &FirmwareComponent,
    ) -> Result<(usize, usize), FdOpsError> {
        // The state machine keeps no cursor of its own: whatever this returns is
        // the offset of the next RequestFirmwareData, verbatim.
        let done = self.bytes_received.get();
        Ok((done, IMAGE_SIZE - done))
    }

    fn download_fw_data(
        &self,
        offset: usize,
        data: &[u8],
        _component: &FirmwareComponent,
    ) -> Result<TransferResult, FdOpsError> {
        let done = self.bytes_received.get();
        if offset != done {
            // A retry re-delivers a window already programmed, and NOR cannot
            // set bits back to 1, so reprogramming it would corrupt the image.
            return Ok(TransferResult::TransferSuccess);
        }
        for (i, byte) in data.iter().enumerate() {
            if *byte != expected_byte(offset + i) {
                self.corrupt.set(true);
                pw_log::error!("FD: byte {} is wrong", (offset + i) as u32);
                return Ok(TransferResult::FdAbortedTransfer);
            }
        }
        if let Err(e) = self
            .flash
            .borrow_mut()
            .program(FlashAddress::new(IMAGE_BASE + offset as u32), data)
        {
            self.corrupt.set(true);
            pw_log::error!(
                "FD: program at {} failed: {:08x}",
                offset as u32,
                e.0.get() as u32
            );
            return Ok(TransferResult::FdAbortedTransfer);
        }
        self.bytes_received.set(done + data.len());
        Ok(TransferResult::TransferSuccess)
    }

    fn is_download_complete(&self, _component: &FirmwareComponent) -> bool {
        self.bytes_received.get() >= IMAGE_SIZE
    }

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

    fn verify(
        &self,
        _component: &FirmwareComponent,
        _progress_percent: &mut ProgressPercent,
    ) -> Result<VerifyResult, FdOpsError> {
        if self.corrupt.get() || self.bytes_received.get() < IMAGE_SIZE {
            pw_log::error!(
                "FD: image incomplete, {} bytes",
                self.bytes_received.get() as u32
            );
            return Ok(VerifyResult::VerifyGenericError);
        }

        let mut flash = self.flash.borrow_mut();
        let mut buf = [0u8; READBACK_CHUNK];
        for base in (0..IMAGE_SIZE).step_by(READBACK_CHUNK) {
            if let Err(e) = flash.read(FlashAddress::new(IMAGE_BASE + base as u32), &mut buf) {
                pw_log::error!(
                    "FD: read at {} failed: {:08x}",
                    base as u32,
                    e.0.get() as u32
                );
                return Ok(VerifyResult::VerifyGenericError);
            }
            for (i, byte) in buf.iter().enumerate() {
                if *byte != expected_byte(base + i) {
                    pw_log::error!("FD: flash byte {} is wrong", (base + i) as u32);
                    return Ok(VerifyResult::VerifyGenericError);
                }
            }
        }

        self.verified.set(true);
        pw_log::info!("FD: image verified in flash, {} bytes", IMAGE_SIZE as u32);
        Ok(VerifyResult::VerifySuccess)
    }

    fn apply(
        &self,
        _component: &FirmwareComponent,
        _progress_percent: &mut ProgressPercent,
    ) -> Result<ApplyResult, FdOpsError> {
        Ok(ApplyResult::ApplySuccess)
    }

    fn activate(
        &self,
        _self_contained_activation: u8,
        estimated_time: &mut u16,
    ) -> Result<u8, FdOpsError> {
        // The image is already in flash; nothing is deferred, so the update
        // agent is told to expect no wait.
        *estimated_time = 0;
        self.activated.set(true);
        pw_log::info!("FD: activated");
        Ok(PldmBaseCompletionCode::Success as u8)
    }

    fn cancel_update_component(&self, _component: &FirmwareComponent) -> Result<(), FdOpsError> {
        Ok(())
    }
}

/// Bring up the FMC and clear the sector the image lands in.
fn init_flash(mmaps: Mmaps) -> Result<BlockingFlash<Backend, NoWaitBlocking>, ErrorCode> {
    // The kernel target applied the FMC pinmux before any process started.
    let driver = Backend::new(mmaps.fmc_regs, mmaps.fmc_cs0_window, mmaps.fmc_cs1_window)?;
    let mut flash = BlockingFlash {
        driver,
        blocking: NoWaitBlocking,
    };
    let (capacity, sector, _) = flash.geometry()?;
    pw_log::info!(
        "FD: CS1 is {} bytes, {} byte sectors",
        capacity.get() as u32,
        sector.get() as u32
    );
    flash.erase(FlashAddress::new(IMAGE_BASE), sector)?;
    Ok(flash)
}

#[entry]
fn entry() {
    // SAFETY: mints this process's memory mappings once, at its entry point.
    let mmaps = unsafe { take_mmaps() };
    let flash = match init_flash(mmaps) {
        Ok(flash) => flash,
        Err(e) => {
            pw_log::error!("FD: flash init failed: {:08x}", e.0.get() as u32);
            let _ = syscall::debug_shutdown(Err(Error::Internal));
            loop {}
        }
    };
    let fd_ops = DemoFdOps::new(flash);

    // Both transports talk to the same MCTP server over the same IPC channel.
    // Nothing is ever in flight on both at once: run_terminus alternates its
    // initiator and responder phases from this single thread.
    let responder_transport = MctpPldmTransport::new(IpcMctpClient::new(handle::MCTP));
    let requester_transport = MctpPldmTransport::new(IpcMctpClient::new(handle::MCTP));

    if responder_transport.stack().set_eid(FD_EID).is_err() {
        pw_log::error!("FD: set_eid failed");
        let _ = syscall::debug_shutdown(Err(Error::Internal));
        loop {}
    }

    let mut fd = FirmwareDevice::init(
        &fd_ops,
        &pldm_interface::config::PLDM_PROTOCOL_CAPABILITIES,
        responder_transport,
        requester_transport,
    );

    pw_log::info!("FD: waiting for the update agent at EID {}", UA_EID as u32);

    let mut buf = [0u8; FD_BUF_SIZE];
    // Returns as soon as ActivateFirmware puts the device back in Idle; an
    // idle timeout arrives as an error instead.
    match fd.run_terminus(
        UA_EID,
        &mut buf,
        IDLE_TIMEOUT_MILLIS,
        REQUESTER_TIMEOUT_MILLIS,
        &mut (),
    ) {
        RunTerminusResult::Completed => {}
        // A plain idle timeout arrives here too, so the code distinguishes
        // "the update agent never spoke" from a real transport fault.
        RunTerminusResult::StoppedByError(PldmServiceError::Mctp(e)) => {
            pw_log::error!("FD: run_terminus stopped, MCTP code {}", e.code as u32);
        }
        RunTerminusResult::StoppedByError(_) => {
            pw_log::error!("FD: run_terminus stopped on a PLDM error");
        }
    }

    if fd_ops.image_is_good() && fd_ops.activated.get() {
        pw_log::info!("FD: update flow complete");
        let _ = syscall::debug_shutdown(Ok(()));
    } else {
        pw_log::error!(
            "FD: update flow did not complete, {} bytes received",
            fd_ops.bytes_received.get() as u32
        );
        let _ = syscall::debug_shutdown(Err(Error::Internal));
    }
    loop {}
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    pw_log::error!("FD: panic");
    let _ = syscall::debug_shutdown(Err(Error::Internal));
    loop {}
}
