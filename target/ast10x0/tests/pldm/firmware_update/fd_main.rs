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

use flash_backend::{Backend, NoWaitBlocking};
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
use pldm_common::protocol::firmware_update::{ComponentResponseCode, Descriptor};
use pldm_common::util::fw_component::FirmwareComponent;
use pldm_interface::firmware_device::fd_ops::{ComponentOperation, FdOps, FdOpsError};
use pw_status::Error;
use userspace::{entry, syscall};
use util_error::ErrorCode;

use app_pldm_fd::handle;

/// This card's EID, matching the MCTP server app underneath it.
const FD_EID: u8 = 8;
/// The update agent's EID on card B.
const UA_EID: u8 = 9;

/// Size of the demo image, in bytes. Must match the update agent's blob.
const IMAGE_SIZE: usize = 1024;

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
}

impl DemoFdOps {
    fn new(flash: BlockingFlash<Backend, NoWaitBlocking>) -> Self {
        DemoFdOps {
            flash: RefCell::new(flash),
            bytes_received: Cell::new(0),
            corrupt: Cell::new(false),
            verified: Cell::new(false),
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
        _device_identifiers: &mut [Descriptor],
    ) -> Result<usize, FdOpsError> {
        Ok(0)
    }

    fn get_firmware_parms(
        &self,
        firmware_params: &mut FirmwareParameters,
    ) -> Result<(), FdOpsError> {
        *firmware_params = FirmwareParameters::default();
        Ok(())
    }

    fn get_xfer_size(&self, ua_transfer_size: usize) -> Result<usize, FdOpsError> {
        Ok(ua_transfer_size.min(MAX_TRANSFER_SIZE))
    }

    fn handle_component(
        &self,
        _component: &FirmwareComponent,
        _fw_params: &FirmwareParameters,
        _op: ComponentOperation,
    ) -> Result<ComponentResponseCode, FdOpsError> {
        Ok(ComponentResponseCode::CompCanBeUpdated)
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
        _estimated_time: &mut u16,
    ) -> Result<u8, FdOpsError> {
        Ok(0)
    }

    fn cancel_update_component(&self, _component: &FirmwareComponent) -> Result<(), FdOpsError> {
        Ok(())
    }
}

/// Bring up the FMC and clear the sector the image lands in.
fn init_flash() -> Result<BlockingFlash<Backend, NoWaitBlocking>, ErrorCode> {
    // SAFETY: this process is the sole owner of the FMC and its CS windows per
    // system.json5, the kernel target applied the FMC pinmux before any process
    // started, and this runs once.
    let driver = unsafe { Backend::new() }?;
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
    let flash = match init_flash() {
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
    // Returns when the update agent goes quiet for IDLE_TIMEOUT_MILLIS, which
    // is how a finished flow ends; the verdict is in `fd_ops`, not here.
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

    if fd_ops.image_is_good() {
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
