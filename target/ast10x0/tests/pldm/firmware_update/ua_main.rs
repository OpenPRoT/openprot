// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! PLDM Update Agent app (the mock BMC).
//!
//! Stimulus for the firmware device on the RoT, not a second root of trust. It
//! walks the whole DSP0267 sequence: it identifies the device with
//! `QueryDeviceIdentifiers` and `GetFirmwareParameters`, hands over an image
//! with `RequestUpdate`, `PassComponentTable` and `UpdateComponent`, answers
//! the requests the firmware device raises on its own while it pulls the image
//! down, and finishes with `ActivateFirmware`.
//!
//! There is no Update Agent in the PLDM service (it is a firmware device only),
//! so the sequence below is written out by hand.

#![no_main]
#![no_std]

use core::cell::Cell;

use app_pldm_ua_regions::{take_mmaps, FmcCs0Window, FmcRegs, Spi1Cs0Window, Spi1Regs};
use ast10x0_peripherals::create_pins;
use ast10x0_peripherals::gpio::{bind_gpio, GpioBlock, OutputPin};
use flash_backend::NoWaitBlocking;
use hal_flash::{BlockingFlash, Flash, FlashAddress};
use openprot_mctp_client_ipc::IpcMctpClient;
use openprot_pldm_service::error::PldmMemError;
use openprot_pldm_service::{MctpPldmTransport, PldmServiceError};
use pldm_common::codec::{PldmCodec, PldmCodecWithLifetime};
use pldm_common::message::firmware_update::activate_fw::{
    ActivateFirmwareRequest, SelfContainedActivationRequest,
};
use pldm_common::message::firmware_update::apply_complete::ApplyCompleteResponse;
use pldm_common::message::firmware_update::get_fw_params::{
    GetFirmwareParametersRequest, GetFirmwareParametersResponse,
};
use pldm_common::message::firmware_update::pass_component::PassComponentTableRequest;
use pldm_common::message::firmware_update::query_devid::{
    QueryDeviceIdentifiersRequest, QueryDeviceIdentifiersResponse,
};
use pldm_common::message::firmware_update::request_fw_data::{
    RequestFirmwareDataRequest, RequestFirmwareDataResponse, MAX_TRANSFER_SIZE,
};
use pldm_common::message::firmware_update::request_update::RequestUpdateRequest;
use pldm_common::message::firmware_update::transfer_complete::TransferCompleteResponse;
use pldm_common::message::firmware_update::update_component::UpdateComponentRequest;
use pldm_common::message::firmware_update::verify_complete::VerifyCompleteResponse;
use pldm_common::protocol::base::{
    PldmBaseCompletionCode, PldmMsgHeader, PldmMsgType, TransferRespFlag,
};
use pldm_common::protocol::firmware_update::{
    ComponentClassification, DescriptorType, FwUpdateCmd, PldmFirmwareString, UpdateOptionFlags,
    VersionStringType, PLDM_FWUP_IMAGE_SET_VER_STR_MAX_LEN,
};
use pw_status::Error;
use userspace::{entry, syscall};
use util_error::ErrorCode;
use util_region::Region;

use app_pldm_ua::handle;

/// The SPI1 backend bound to this process's register mapping.
type Backend = flash_backend::Ast10x0Spi1FlashDriver<Spi1Regs>;

/// The FMC backend for this board's own boot flash.
type BootBackend = flash_backend::Ast10x0FmcCs0FlashDriver<FmcRegs>;

/// This board's EID, matching the MCTP server app underneath it.
const UA_EID: u8 = 9;
/// The firmware device's EID on the RoT.
const FD_EID: u8 = 8;

/// Where the firmware device stages the image in the shared flash. Must match
/// the firmware device's.
const IMAGE_BASE: u32 = 0x10_0000;

/// How much of the staged image is read back at a time.
const READBACK_CHUNK: usize = 256;

/// Where the boot ROM looks for the Aspeed secure-boot header. Only the image
/// size at `header[8..12]` matters on an unfused part.
const SB_HEADER_OFFSET: usize = 0x400;

/// Which generation of this image is running. In `.rodata`, so `&raw const` is
/// its offset in the image on this non-XIP part; a `.data` static would resolve
/// to the RAM copy instead. The bump applied on the way out over PLDM is what
/// the next boot prints, so it can only have come from the round trip.
#[used]
pub static BOOT_VERSION: u32 = 0;

/// Read the version out of the running image. Volatile because these bytes are
/// patched after compilation, so the initializer above is not the truth and the
/// compiler must not fold reads of it back to zero.
fn boot_version() -> u32 {
    // SAFETY: reads a live static in this image.
    unsafe { core::ptr::read_volatile(&raw const BOOT_VERSION) }
}

/// The UUID this agent expects the firmware device to report. It only updates
/// a device it recognises.
const DEVICE_UUID: [u8; 16] = [
    0x4f, 0x50, 0x52, 0x54, 0x00, 0x01, 0x40, 0x00, 0xa0, 0x00, 0xde, 0xad, 0xbe, 0xef, 0x00, 0x01,
];

/// Comparison stamp offered for the new image. Must beat the stamp the device
/// reports for its running firmware or the component is refused.
const COMP_COMPARISON_STAMP: u32 = 1;

/// How long each UA-initiated request waits for the firmware device's reply.
const REQUEST_TIMEOUT_MILLIS: u32 = 5_000;
/// How long the UA waits for each firmware-device-initiated request.
const SERVE_TIMEOUT_MILLIS: u32 = 30_000;

/// Upper bound on firmware-device-initiated requests served before giving up.
/// A clean run is ceil(image_size / MAX_TRANSFER_SIZE) RequestFirmwareData plus
/// TransferComplete, VerifyComplete, and ApplyComplete.
fn max_served_requests(image_size: u32) -> u32 {
    image_size.div_ceil(MAX_TRANSFER_SIZE as u32) + 3
}

const UA_BUF_SIZE: usize = 1024;

/// Build a fixed-size PLDM firmware version string.
fn fw_string(s: &str) -> PldmFirmwareString {
    let bytes = s.as_bytes();
    let mut str_data = [0u8; PLDM_FWUP_IMAGE_SET_VER_STR_MAX_LEN];
    let len = bytes.len().min(PLDM_FWUP_IMAGE_SET_VER_STR_MAX_LEN);
    str_data[..len].copy_from_slice(&bytes[..len]);
    PldmFirmwareString {
        str_type: VersionStringType::Ascii as u8,
        str_len: len as u8,
        str_data,
    }
}

/// Answer one firmware-device-initiated request in place.
///
/// `framed_buf[0]` is the MCTP type byte and the request occupies
/// `framed_buf[1..req_total_len]`; the response is written back over
/// `framed_buf[1..]`. Returns the total response length including the type
/// byte, and sets `saw_apply_complete` once the device reports it is done.
///
/// The bytes handed over are this board's own firmware, read back out of the
/// boot flash the ROM loaded it from. Reading SRAM instead would need a mapping
/// of the whole image, which the MPU cannot express without covering the kernel.
fn serve_fd_request(
    framed_buf: &mut [u8],
    req_total_len: usize,
    saw_apply_complete: &Cell<bool>,
    boot: &mut BlockingFlash<BootBackend, NoWaitBlocking>,
    image_size: u32,
) -> Result<usize, PldmServiceError> {
    let success = PldmBaseCompletionCode::Success as u8;

    // Decode to owned values first so the response can be written back over
    // the same buffer.
    let (instance_id, cmd, fw_window) = {
        let payload = &framed_buf[1..req_total_len];
        let Ok(header) = PldmMsgHeader::<[u8; 3]>::decode(payload) else {
            pw_log::error!("UA: could not decode FD request header");
            return Ok(0);
        };
        let cmd = header.cmd_code();
        let fw_window = if cmd == FwUpdateCmd::RequestFirmwareData as u8 {
            match RequestFirmwareDataRequest::decode(payload) {
                Ok(req) => Some((req.offset as usize, req.length as usize)),
                Err(_) => {
                    pw_log::error!("UA: could not decode RequestFirmwareData");
                    return Ok(0);
                }
            }
        } else {
            None
        };
        (header.instance_id(), cmd, fw_window)
    };

    let resp = &mut framed_buf[1..];
    let resp_len = match FwUpdateCmd::try_from(cmd) {
        Ok(FwUpdateCmd::RequestFirmwareData) => {
            let Some((offset, length)) = fw_window else {
                return Ok(0);
            };
            if length > MAX_TRANSFER_SIZE {
                pw_log::error!("UA: FD asked for {} bytes, over the MTU", length as u32);
                return Ok(0);
            }
            if offset
                .checked_add(length)
                .is_none_or(|end| end > image_size as usize)
            {
                pw_log::error!("UA: FD asked for bytes past the end of the image");
                return Ok(0);
            }
            let mut chunk_buf = [0u8; MAX_TRANSFER_SIZE];
            let chunk = &mut chunk_buf[..length];
            if let Err(e) = boot.read(FlashAddress::new(offset as u32), chunk) {
                pw_log::error!(
                    "UA: boot read at {} failed: {:08x}",
                    offset as u32,
                    e.0.get() as u32
                );
                return Ok(0);
            }
            overlay_version(offset, chunk);
            let msg = RequestFirmwareDataResponse::new(instance_id, success, chunk);
            PldmCodecWithLifetime::encode(&msg, resp)
        }
        Ok(FwUpdateCmd::TransferComplete) => {
            TransferCompleteResponse::new(instance_id, success).encode(resp)
        }
        Ok(FwUpdateCmd::VerifyComplete) => {
            VerifyCompleteResponse::new(instance_id, success).encode(resp)
        }
        Ok(FwUpdateCmd::ApplyComplete) => {
            saw_apply_complete.set(true);
            ApplyCompleteResponse::new(instance_id, success).encode(resp)
        }
        _ => {
            pw_log::error!("UA: unexpected FD request, cmd={}", cmd as u32);
            return Ok(0);
        }
    };

    match resp_len {
        Ok(len) => Ok(len + 1),
        Err(_) => {
            pw_log::error!("UA: could not encode response to cmd={}", cmd as u32);
            Ok(0)
        }
    }
}

/// Bring up SPI1 so the staged image can be read back.
///
/// Read-only: nothing here erases or programs. The firmware device owns the
/// contents; this agent only checks them.
fn init_flash(
    spi1_regs: Region<Spi1Regs>,
    spi1_cs0_window: Region<Spi1Cs0Window>,
) -> Result<BlockingFlash<Backend, NoWaitBlocking>, ErrorCode> {
    let driver = Backend::new(spi1_regs, spi1_cs0_window)?;
    let mut flash = BlockingFlash {
        driver,
        blocking: NoWaitBlocking,
    };
    let (capacity, sector, _) = flash.geometry()?;
    pw_log::info!(
        "UA: SPI1 CS0 is {} bytes, {} byte sectors",
        capacity.get() as u32,
        sector.get() as u32
    );
    // The geometry above is pinned, so this is the first call that says whether
    // the bus actually reaches the part.
    let jedec = flash.driver.jedec_id()?;
    pw_log::info!(
        "UA: SPI1 CS0 JEDEC ID {:02x} {:02x} {:02x}",
        jedec[0] as u32,
        jedec[1] as u32,
        jedec[2] as u32
    );
    Ok(flash)
}

/// Read the image the firmware device staged and check it against what this
/// agent handed over.
///
/// Runs before the boot flash is erased, so it is still the same source the
/// chunks were served from.
fn staged_image_matches(
    flash: &mut BlockingFlash<Backend, NoWaitBlocking>,
    boot: &mut BlockingFlash<BootBackend, NoWaitBlocking>,
    image_size: u32,
) -> bool {
    let mut buf = [0u8; READBACK_CHUNK];
    let mut src = [0u8; READBACK_CHUNK];
    for base in (0..image_size as usize).step_by(READBACK_CHUNK) {
        let n = (image_size as usize - base).min(READBACK_CHUNK);
        let buf = &mut buf[..n];
        let src = &mut src[..n];
        if let Err(e) = flash.read(FlashAddress::new(IMAGE_BASE + base as u32), buf) {
            pw_log::error!(
                "UA: read at {} failed: {:08x}",
                base as u32,
                e.0.get() as u32
            );
            return false;
        }
        if let Err(e) = boot.read(FlashAddress::new(base as u32), src) {
            pw_log::error!(
                "UA: boot read at {} failed: {:08x}",
                base as u32,
                e.0.get() as u32
            );
            return false;
        }
        overlay_version(base, src);
        if buf != src {
            pw_log::error!("UA: staged bytes differ at {}", base as u32);
            return false;
        }
    }
    pw_log::info!("UA: read back {} staged bytes intact", image_size as u32);
    true
}

/// Bring up the FMC so this board's own boot flash can be rewritten.
fn init_boot_flash(
    fmc_regs: Region<FmcRegs>,
    fmc_cs0_window: Region<FmcCs0Window>,
) -> Result<BlockingFlash<BootBackend, NoWaitBlocking>, ErrorCode> {
    let driver = BootBackend::new(fmc_regs, fmc_cs0_window)?;
    let mut flash = BlockingFlash {
        driver,
        blocking: NoWaitBlocking,
    };
    let (capacity, sector, _) = flash.geometry()?;
    pw_log::info!(
        "UA: FMC CS0 is {} bytes, {} byte sectors",
        capacity.get() as u32,
        sector.get() as u32
    );
    let jedec = flash.driver.jedec_id()?;
    pw_log::info!(
        "UA: FMC CS0 JEDEC ID {:02x} {:02x} {:02x}",
        jedec[0] as u32,
        jedec[1] as u32,
        jedec[2] as u32
    );
    Ok(flash)
}

/// Build the header the boot ROM reads at `SB_HEADER_OFFSET`.
fn sb_header(img_size: usize) -> [u8; 32] {
    let mut header = [0u8; 32];
    header[8..12].copy_from_slice(&(img_size as u32).to_le_bytes());
    header
}

/// Overlay `header` onto `out` wherever this chunk straddles `SB_HEADER_OFFSET`.
fn overlay_header(offset: usize, header: &[u8; 32], out: &mut [u8]) {
    let start = SB_HEADER_OFFSET.max(offset);
    let end = (SB_HEADER_OFFSET + header.len()).min(offset + out.len());
    if start < end {
        out[start - offset..end - offset]
            .copy_from_slice(&header[start - SB_HEADER_OFFSET..end - SB_HEADER_OFFSET]);
    }
}

/// Overlay the incremented version onto `out` wherever this chunk straddles the
/// `BOOT_VERSION` field. Applied only to bytes on their way out over PLDM, so
/// the higher number can reach this board's flash by the round trip alone.
fn overlay_version(offset: usize, out: &mut [u8]) {
    let bytes = (boot_version() + 1).to_le_bytes();
    let field = &raw const BOOT_VERSION as usize;
    let start = field.max(offset);
    let end = (field + bytes.len()).min(offset + out.len());
    if start < end {
        out[start - offset..end - offset].copy_from_slice(&bytes[start - field..end - field]);
    }
}

/// Copy the staged image out of the shared flash into this board's boot flash,
/// so the next reset runs what the firmware device just delivered.
///
/// Safe to erase while running: the part is non-XIP and this image was uploaded
/// over UART into SRAM, so nothing is being fetched from the flash below.
fn copy_staged_to_boot_flash(
    staging: &mut BlockingFlash<Backend, NoWaitBlocking>,
    boot: &mut BlockingFlash<BootBackend, NoWaitBlocking>,
    image_size: u32,
) -> bool {
    let sector = match boot.geometry() {
        Ok((_, sector, _)) => sector,
        Err(e) => {
            pw_log::error!("UA: boot flash geometry failed: {:08x}", e.0.get() as u32);
            return false;
        }
    };

    let erase_len = (image_size as usize).next_multiple_of(sector.get());
    for base in (0..erase_len).step_by(sector.get()) {
        if let Err(e) = boot.erase(FlashAddress::new(base as u32), sector) {
            pw_log::error!(
                "UA: boot flash erase at {} failed: {:08x}",
                base as u32,
                e.0.get() as u32
            );
            return false;
        }
    }
    pw_log::info!("UA: erased {} bytes of boot flash", erase_len as u32);

    let header = sb_header(image_size as usize);
    let mut buf = [0u8; READBACK_CHUNK];
    let mut check = [0u8; READBACK_CHUNK];
    for base in (0..image_size as usize).step_by(READBACK_CHUNK) {
        let n = (image_size as usize - base).min(READBACK_CHUNK);
        let buf = &mut buf[..n];
        if let Err(e) = staging.read(FlashAddress::new(IMAGE_BASE + base as u32), buf) {
            pw_log::error!(
                "UA: staged read at {} failed: {:08x}",
                base as u32,
                e.0.get() as u32
            );
            return false;
        }
        overlay_header(base, &header, buf);
        if let Err(e) = boot.program(FlashAddress::new(base as u32), buf) {
            pw_log::error!(
                "UA: boot flash program at {} failed: {:08x}",
                base as u32,
                e.0.get() as u32
            );
            return false;
        }
        let check = &mut check[..n];
        if let Err(e) = boot.read(FlashAddress::new(base as u32), check) {
            pw_log::error!(
                "UA: boot flash read at {} failed: {:08x}",
                base as u32,
                e.0.get() as u32
            );
            return false;
        }
        if check != buf {
            pw_log::error!("UA: boot flash differs at {}", base as u32);
            return false;
        }
    }

    pw_log::info!("UA: wrote {} bytes to boot flash", image_size as u32);
    true
}

/// Read the image length the boot ROM uses out of this board's own boot flash.
fn boot_image_size(
    boot: &mut BlockingFlash<BootBackend, NoWaitBlocking>,
) -> Result<u32, ErrorCode> {
    let mut len = [0u8; 4];
    boot.read(FlashAddress::new((SB_HEADER_OFFSET + 8) as u32), &mut len)?;
    Ok(u32::from_le_bytes(len))
}

/// Drives the update; `Ok(true)` means the firmware device reported apply
/// complete and then accepted activation, which is this card's pass condition.
fn run_update(
    transport: &MctpPldmTransport<IpcMctpClient>,
    flash: &mut BlockingFlash<Backend, NoWaitBlocking>,
    boot: &mut BlockingFlash<BootBackend, NoWaitBlocking>,
    image_size: u32,
) -> Result<bool, PldmServiceError> {
    // Registered before UpdateComponent is sent: the firmware device starts
    // issuing RequestFirmwareData the moment it answers that command, and the
    // MCTP stack drops inbound requests with no listener bound.
    let mut listener = transport.responder_listener(SERVE_TIMEOUT_MILLIS)?;

    let comp_ver = fw_string("v1.0");
    let mut buf = [0u8; UA_BUF_SIZE];
    let mut instance_id = 0u8;

    // Returns the completion code and the length of the PLDM response, which
    // starts at buf[1].
    let transact = |pldm_len: usize, buf: &mut [u8]| -> Result<(u8, usize), PldmServiceError> {
        let resp_len = transport.send_request(FD_EID, pldm_len, buf, REQUEST_TIMEOUT_MILLIS)?;
        // The completion code follows the 3-byte PLDM header.
        Ok((if resp_len > 3 { buf[4] } else { 0xff }, resp_len))
    };

    // ---- QueryDeviceIdentifiers: confirm which device answered ----
    let query_devid = QueryDeviceIdentifiersRequest::new(instance_id, PldmMsgType::Request);
    let len = query_devid
        .encode(&mut buf[1..])
        .map_err(|_| PldmServiceError::PldmMem(PldmMemError::BufferTooSmall))?;
    let (cc, resp_len) = transact(len, &mut buf)?;
    if cc != 0 {
        pw_log::error!("UA: QueryDeviceIdentifiers rejected, cc={}", cc as u32);
        return Ok(false);
    }
    let Ok(devid) = QueryDeviceIdentifiersResponse::decode(&buf[1..1 + resp_len]) else {
        pw_log::error!("UA: could not decode QueryDeviceIdentifiers response");
        return Ok(false);
    };
    let descriptor = devid.initial_descriptor;
    if descriptor.descriptor_type != DescriptorType::Uuid as u16
        || descriptor.descriptor_data[..DEVICE_UUID.len()] != DEVICE_UUID
    {
        pw_log::error!("UA: device identifier does not match, refusing to update");
        return Ok(false);
    }
    pw_log::info!("UA: device identified by UUID");

    // ---- GetFirmwareParameters: learn which component to offer ----
    instance_id += 1;
    let get_params = GetFirmwareParametersRequest::new(instance_id, PldmMsgType::Request);
    let len = get_params
        .encode(&mut buf[1..])
        .map_err(|_| PldmServiceError::PldmMem(PldmMemError::BufferTooSmall))?;
    let (cc, resp_len) = transact(len, &mut buf)?;
    if cc != 0 {
        pw_log::error!("UA: GetFirmwareParameters rejected, cc={}", cc as u32);
        return Ok(false);
    }
    let Ok(params) = GetFirmwareParametersResponse::decode(&buf[1..1 + resp_len]) else {
        pw_log::error!("UA: could not decode GetFirmwareParameters response");
        return Ok(false);
    };
    let comp_count = params.parms.params_fixed.comp_count;
    if comp_count != 1 {
        pw_log::error!(
            "UA: device reports {} components, expected 1",
            comp_count as u32
        );
        return Ok(false);
    }
    let entry = &params.parms.comp_param_table[0].comp_param_entry_fixed;
    if entry.comp_classification != ComponentClassification::Firmware as u16 {
        pw_log::error!(
            "UA: component is not firmware, classification {}",
            entry.comp_classification as u32
        );
        return Ok(false);
    }
    // The device is the source of truth for the identifier; this agent offers
    // an update for whatever it reported.
    let comp_identifier = entry.comp_identifier;
    let comp_classification_index = entry.comp_classification_index;
    pw_log::info!(
        "UA: device offers component {} for update",
        comp_identifier as u32
    );

    // ---- RequestUpdate: move the firmware device out of Idle ----
    instance_id += 1;
    let req_update = RequestUpdateRequest::new(
        instance_id,
        PldmMsgType::Request,
        image_size, // max_transfer_size
        1,          // num_of_comp
        1,          // max_outstanding_transfer_req
        0,          // pkg_data_len
        &comp_ver,
    );
    let len = req_update
        .encode(&mut buf[1..])
        .map_err(|_| PldmServiceError::PldmMem(PldmMemError::BufferTooSmall))?;
    let (cc, _) = transact(len, &mut buf)?;
    if cc != 0 {
        pw_log::error!("UA: RequestUpdate rejected, cc={}", cc as u32);
        return Ok(false);
    }

    // ---- PassComponentTable: describe the single component ----
    instance_id += 1;
    let pass_comp = PassComponentTableRequest::new(
        instance_id,
        PldmMsgType::Request,
        TransferRespFlag::StartAndEnd,
        ComponentClassification::Firmware,
        comp_identifier,
        comp_classification_index,
        COMP_COMPARISON_STAMP,
        &comp_ver,
    );
    let len = pass_comp
        .encode(&mut buf[1..])
        .map_err(|_| PldmServiceError::PldmMem(PldmMemError::BufferTooSmall))?;
    let (cc, _) = transact(len, &mut buf)?;
    if cc != 0 {
        pw_log::error!("UA: PassComponentTable rejected, cc={}", cc as u32);
        return Ok(false);
    }

    // ---- UpdateComponent: the firmware device starts pulling the image ----
    instance_id += 1;
    let update_comp = UpdateComponentRequest::new(
        instance_id,
        PldmMsgType::Request,
        ComponentClassification::Firmware,
        comp_identifier,
        comp_classification_index,
        COMP_COMPARISON_STAMP,
        image_size,
        UpdateOptionFlags(0),
        &comp_ver,
    );
    let len = update_comp
        .encode(&mut buf[1..])
        .map_err(|_| PldmServiceError::PldmMem(PldmMemError::BufferTooSmall))?;
    let (cc, _) = transact(len, &mut buf)?;
    if cc != 0 {
        pw_log::error!("UA: UpdateComponent rejected, cc={}", cc as u32);
        return Ok(false);
    }

    pw_log::info!("UA: handing over {} bytes", image_size as u32);

    let max_requests = max_served_requests(image_size);
    let saw_apply_complete = Cell::new(false);
    for _ in 0..max_requests {
        transport.respond_once(
            &mut listener,
            &mut buf,
            |framed_buf, req_total_len, _eid| {
                serve_fd_request(
                    framed_buf,
                    req_total_len,
                    &saw_apply_complete,
                    boot,
                    image_size,
                )
            },
        )?;
        if saw_apply_complete.get() {
            pw_log::info!("UA: firmware device reported apply complete");
            break;
        }
    }

    if !saw_apply_complete.get() {
        pw_log::error!("UA: gave up after {} requests", max_requests as u32);
        return Ok(false);
    }

    // Read and copy before ActivateFirmware: once that is sent the firmware
    // device pulses its reset line and the harness resets this board.
    if !staged_image_matches(flash, boot, image_size) {
        return Ok(false);
    }
    if !copy_staged_to_boot_flash(flash, boot, image_size) {
        return Ok(false);
    }

    // ---- ActivateFirmware: the device runs the new image and goes Idle ----
    instance_id += 1;
    let activate = ActivateFirmwareRequest::new(
        instance_id,
        PldmMsgType::Request,
        SelfContainedActivationRequest::ActivateSelfContainedComponents,
    );
    let len = activate
        .encode(&mut buf[1..])
        .map_err(|_| PldmServiceError::PldmMem(PldmMemError::BufferTooSmall))?;
    let (cc, _) = transact(len, &mut buf)?;
    if cc != 0 {
        pw_log::error!("UA: ActivateFirmware rejected, cc={}", cc as u32);
        return Ok(false);
    }

    pw_log::info!("UA: firmware activated, update complete");
    Ok(true)
}

#[entry]
fn entry() {
    pw_log::info!("Hello from BMC version {}", boot_version() as u32);

    // SAFETY: mints this process's memory mappings once, at its entry point.
    let mmaps = unsafe { take_mmaps() };
    // SAFETY: sole pin creation site in this binary, at boot; the pins! table is this chip's true pin map.
    let pins = unsafe { create_pins() };
    let gpio = GpioBlock::new(mmaps.gpio_regs);
    // GPIOH5: this board's booted line, driven high once this image is running and
    // left there, so a reset shows up to the RoT as a low gap followed by a rise.
    // Jumper: this pin -> RoT's GPIOH4.
    let mut alive_pin = bind_gpio(pins.scu414_29, &gpio).into_output();
    let _ = alive_pin.set_high();

    let mut flash = match init_flash(mmaps.spi1_regs, mmaps.spi1_cs0_window) {
        Ok(flash) => flash,
        Err(e) => {
            pw_log::error!("UA: flash init failed: {:08x}", e.0.get() as u32);
            let _ = syscall::debug_shutdown(Err(Error::Internal));
            loop {}
        }
    };

    let mut boot = match init_boot_flash(mmaps.fmc_regs, mmaps.fmc_cs0_window) {
        Ok(flash) => flash,
        Err(e) => {
            pw_log::error!("UA: boot flash init failed: {:08x}", e.0.get() as u32);
            let _ = syscall::debug_shutdown(Err(Error::Internal));
            loop {}
        }
    };

    let image_size = match boot_image_size(&mut boot) {
        Ok(size) => size,
        Err(e) => {
            pw_log::error!("UA: boot header read failed: {:08x}", e.0.get() as u32);
            let _ = syscall::debug_shutdown(Err(Error::Internal));
            loop {}
        }
    };
    pw_log::info!("UA: own image is {} bytes", image_size as u32);

    let transport = MctpPldmTransport::new(IpcMctpClient::new(handle::MCTP));

    if transport.stack().set_eid(UA_EID).is_err() {
        pw_log::error!("UA: set_eid failed");
        let _ = syscall::debug_shutdown(Err(Error::Internal));
        loop {}
    }

    pw_log::info!("UA: driving an update against EID {}", FD_EID as u32);
    // Only failures report a verdict here. On success this board's verdict comes
    // from the delivered image once the reset boots it out of flash.
    match run_update(&transport, &mut flash, &mut boot, image_size) {
        Ok(true) => {
            pw_log::info!("UA: update staged and copied, waiting for the reset");
        }
        Ok(false) => {
            let _ = syscall::debug_shutdown(Err(Error::Internal));
        }
        Err(PldmServiceError::Mctp(e)) => {
            // On the boot that follows activation the firmware device has already
            // finished and shut down, so nothing answers and there is nothing to do.
            pw_log::info!(
                "UA: no firmware device answered, MCTP code {}",
                e.code as u32
            );
            let _ = syscall::debug_shutdown(Ok(()));
        }
        Err(_) => {
            pw_log::error!("UA: update flow failed on a PLDM error");
            let _ = syscall::debug_shutdown(Err(Error::Internal));
        }
    }

    #[expect(clippy::empty_loop)]
    loop {}
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    pw_log::error!("UA: panic");
    let _ = syscall::debug_shutdown(Err(Error::Internal));
    loop {}
}
