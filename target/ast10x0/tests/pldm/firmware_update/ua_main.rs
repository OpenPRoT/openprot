// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! PLDM Update Agent app (card B, the mock BMC).
//!
//! Stimulus for the firmware device on card A, not a second root of trust. It
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

use app_pldm_ua::handle;

/// This card's EID, matching the MCTP server app underneath it.
const UA_EID: u8 = 9;
/// The firmware device's EID on card A.
const FD_EID: u8 = 8;

/// Size of the demo image, in bytes. Must match the firmware device's.
const IMAGE_SIZE: u32 = 1024;

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
/// A clean run is ceil(IMAGE_SIZE / MAX_TRANSFER_SIZE) RequestFirmwareData plus
/// TransferComplete, VerifyComplete, and ApplyComplete.
const MAX_SERVED_REQUESTS: u32 = 64;

const UA_BUF_SIZE: usize = 1024;

/// The byte the demo image carries at `offset`. The firmware device generates
/// the same sequence and rejects anything that does not match.
fn expected_byte(offset: usize) -> u8 {
    (offset % 251) as u8
}

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
fn serve_fd_request(
    framed_buf: &mut [u8],
    req_total_len: usize,
    saw_apply_complete: &Cell<bool>,
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
            let mut chunk = [0u8; MAX_TRANSFER_SIZE];
            for (i, byte) in chunk[..length].iter_mut().enumerate() {
                *byte = expected_byte(offset + i);
            }
            let msg = RequestFirmwareDataResponse::new(instance_id, success, &chunk[..length]);
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

/// Drives the update; `Ok(true)` means the firmware device reported apply
/// complete and then accepted activation, which is this card's pass condition.
fn run_update(transport: &MctpPldmTransport<IpcMctpClient>) -> Result<bool, PldmServiceError> {
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
        IMAGE_SIZE, // max_transfer_size
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
        IMAGE_SIZE,
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

    pw_log::info!("UA: handing over {} bytes", IMAGE_SIZE as u32);

    let saw_apply_complete = Cell::new(false);
    for _ in 0..MAX_SERVED_REQUESTS {
        transport.respond_once(
            &mut listener,
            &mut buf,
            |framed_buf, req_total_len, _eid| {
                serve_fd_request(framed_buf, req_total_len, &saw_apply_complete)
            },
        )?;
        if saw_apply_complete.get() {
            pw_log::info!("UA: firmware device reported apply complete");
            break;
        }
    }

    if !saw_apply_complete.get() {
        pw_log::error!("UA: gave up after {} requests", MAX_SERVED_REQUESTS as u32);
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
    let transport = MctpPldmTransport::new(IpcMctpClient::new(handle::MCTP));

    if transport.stack().set_eid(UA_EID).is_err() {
        pw_log::error!("UA: set_eid failed");
        let _ = syscall::debug_shutdown(Err(Error::Internal));
        loop {}
    }

    pw_log::info!("UA: driving an update against EID {}", FD_EID as u32);
    // The harness watches both cards and requires a verdict from each, so this
    // card reports whether it saw the update through, not just card A.
    match run_update(&transport) {
        Ok(true) => {
            let _ = syscall::debug_shutdown(Ok(()));
        }
        Ok(false) => {
            let _ = syscall::debug_shutdown(Err(Error::Internal));
        }
        Err(PldmServiceError::Mctp(e)) => {
            pw_log::error!("UA: update flow failed, MCTP code {}", e.code as u32);
            let _ = syscall::debug_shutdown(Err(Error::Internal));
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
