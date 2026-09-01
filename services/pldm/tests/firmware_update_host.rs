// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! End-to-end host test for a full PLDM firmware-update flow.
//!
//! This extends the wiring exercised by `base_host.rs` with the FD-initiated
//! request path. `FirmwareDevice` owns both MCTP transports directly: when it
//! enters update mode it drives the download / verify / apply state machine by
//! issuing PLDM requests (`RequestFirmwareData`, `TransferComplete`,
//! `VerifyComplete`, `ApplyComplete`) to the Update Agent through its
//! `requester_transport`, while UA->FD commands keep flowing through its
//! `responder_transport`:
//!
//! ```text
//!  UA cmd --> FirmwareDevice.run_terminus (responder_transport)
//!                                   |
//!                                   v (initiator mode)
//!                              requester_transport --> remote UA (over MCTP)
//! ```
//!
//! All transports are backed by in-memory MCTP `Server`s and packet queues.

use core::cell::{Cell, RefCell};

use mctp::Eid;
use mctp_lib::Sender;
use openprot_mctp_api::Handle;
use openprot_mctp_server::Server;
use openprot_orchestrator_pldm_adapter::UpdateRequestLatch;
use openprot_orchestrator_sm::Event;
use openprot_pldm_service::firmware_device::{FirmwareDevice, RunTerminusResult};
use openprot_pldm_service::{MctpPldmTransport, PldmServiceError};
use pldm_common::codec::{PldmCodec, PldmCodecWithLifetime};
use pldm_common::message::firmware_update::apply_complete::{ApplyCompleteResponse, ApplyResult};
use pldm_common::message::firmware_update::get_fw_params::FirmwareParameters;
use pldm_common::message::firmware_update::get_status::{
    GetStatusRequest, GetStatusResponse, ProgressPercent,
};
use pldm_common::message::firmware_update::pass_component::PassComponentTableRequest;
use pldm_common::message::firmware_update::request_fw_data::{
    RequestFirmwareDataRequest, RequestFirmwareDataResponse, MAX_TRANSFER_SIZE,
};
use pldm_common::message::firmware_update::request_update::RequestUpdateRequest;
use pldm_common::message::firmware_update::transfer_complete::{
    TransferCompleteResponse, TransferResult,
};
use pldm_common::message::firmware_update::update_component::UpdateComponentRequest;
use pldm_common::message::firmware_update::verify_complete::{
    VerifyCompleteResponse, VerifyResult,
};
use pldm_common::protocol::base::{
    PldmBaseCompletionCode, PldmMsgHeader, PldmMsgType, TransferRespFlag,
};
use pldm_common::protocol::firmware_update::{
    ComponentClassification, ComponentResponseCode, Descriptor, FirmwareDeviceState, FwUpdateCmd,
    FwUpdateCompletionCode, PldmFirmwareString, UpdateOptionFlags, VersionStringType,
    PLDM_FWUP_IMAGE_SET_VER_STR_MAX_LEN,
};
use pldm_common::util::fw_component::FirmwareComponent;
use pldm_interface::firmware_device::fd_ops::{ComponentOperation, FdOps, FdOpsError};

mod common;
use common::{transfer, BufferSender, DirectClientWithPump, FD_EID, TIMEOUT_MILLIS, UA_EID};

const PLDM_MSG_TYPE: u8 = 0x01;

/// Total firmware image size (bytes) advertised in `UpdateComponent`.
const IMAGE_SIZE: u32 = 1024;

// ---------------------------------------------------------------------------
// Fake firmware-device operations.
// ---------------------------------------------------------------------------

struct MockFdOps {
    component_accepted: Cell<bool>,
    download_bytes_received: Cell<usize>,
    verified: Cell<bool>,
    applied: Cell<bool>,
}

impl FdOps for MockFdOps {
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
        self.component_accepted.set(true);
        Ok(ComponentResponseCode::CompCanBeUpdated)
    }

    fn query_download_offset_and_length(
        &self,
        _component: &FirmwareComponent,
    ) -> Result<(usize, usize), FdOpsError> {
        Ok((0, IMAGE_SIZE as usize))
    }

    fn download_fw_data(
        &self,
        _offset: usize,
        data: &[u8],
        _component: &FirmwareComponent,
    ) -> Result<TransferResult, FdOpsError> {
        self.download_bytes_received
            .set(self.download_bytes_received.get() + data.len());
        Ok(TransferResult::TransferSuccess)
    }

    fn is_download_complete(&self, _component: &FirmwareComponent) -> bool {
        self.download_bytes_received.get() >= IMAGE_SIZE as usize
    }

    fn query_download_progress(
        &self,
        _component: &FirmwareComponent,
        progress_percent: &mut ProgressPercent,
    ) -> Result<(), FdOpsError> {
        let pct = (self.download_bytes_received.get() * 100 / IMAGE_SIZE as usize) as u8;
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
        // Leave `progress_percent` at its default (NOT_SUPPORTED), which the FD
        // treats as "done" so the VerifyComplete request is issued immediately.
        self.verified.set(true);
        Ok(VerifyResult::VerifySuccess)
    }

    fn apply(
        &self,
        _component: &FirmwareComponent,
        _progress_percent: &mut ProgressPercent,
    ) -> Result<ApplyResult, FdOpsError> {
        self.applied.set(true);
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

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build a fixed-size PLDM firmware version string.
fn fw_string(s: &str) -> PldmFirmwareString {
    let bytes = s.as_bytes();
    assert!(bytes.len() <= PLDM_FWUP_IMAGE_SET_VER_STR_MAX_LEN);
    let mut str_data = [0u8; PLDM_FWUP_IMAGE_SET_VER_STR_MAX_LEN];
    str_data[..bytes.len()].copy_from_slice(bytes);
    PldmFirmwareString {
        str_type: VersionStringType::Ascii as u8,
        str_len: bytes.len() as u8,
        str_data,
    }
}

/// Act as the remote Update Agent for a single FD-initiated PLDM request.
///
/// Reads one request off `ua_server`'s PLDM listener, produces the matching
/// success response (returning firmware bytes for `RequestFirmwareData`), and
/// sends it back to the firmware device.
fn serve_ua_fw_request<S: Sender, const N: usize>(
    ua_server: &RefCell<Server<S, N>>,
    listener: Handle,
) {
    let mut req = [0u8; 1024];
    let meta = match ua_server.borrow_mut().try_recv(listener, &mut req) {
        Some(m) => m,
        None => return,
    };
    let payload = &req[..meta.payload_size];

    let header = PldmMsgHeader::<[u8; 3]>::decode(payload).expect("decode FD request header");
    let instance_id = header.instance_id();
    let cmd = header.cmd_code();

    let success = PldmBaseCompletionCode::Success as u8;
    let mut resp = [0u8; 1024];
    let resp_len = match FwUpdateCmd::try_from(cmd) {
        Ok(FwUpdateCmd::RequestFirmwareData) => {
            let fw_req =
                RequestFirmwareDataRequest::decode(payload).expect("decode RequestFirmwareData");
            let length = fw_req.length as usize;
            assert!(length <= MAX_TRANSFER_SIZE, "requested chunk exceeds MTU");
            let data = [0xA5u8; MAX_TRANSFER_SIZE];
            let resp_msg = RequestFirmwareDataResponse::new(instance_id, success, &data[..length]);
            PldmCodecWithLifetime::encode(&resp_msg, &mut resp)
                .expect("encode RequestFirmwareData response")
        }
        Ok(FwUpdateCmd::TransferComplete) => TransferCompleteResponse::new(instance_id, success)
            .encode(&mut resp)
            .expect("encode TransferComplete response"),
        Ok(FwUpdateCmd::VerifyComplete) => VerifyCompleteResponse::new(instance_id, success)
            .encode(&mut resp)
            .expect("encode VerifyComplete response"),
        Ok(FwUpdateCmd::ApplyComplete) => ApplyCompleteResponse::new(instance_id, success)
            .encode(&mut resp)
            .expect("encode ApplyComplete response"),
        _ => panic!("unexpected FD-initiated request: cmd={cmd:#x}"),
    };

    ua_server
        .borrow_mut()
        .send(
            None,
            PLDM_MSG_TYPE,
            Some(meta.remote_eid),
            Some(meta.msg_tag),
            false,
            &resp[..resp_len],
        )
        .expect("send FD-initiated request response");
}

// ---------------------------------------------------------------------------
// Test
// ---------------------------------------------------------------------------

#[test]
fn firmware_update_full_flow_via_requester() {
    let fd_ops = MockFdOps {
        component_accepted: Cell::new(false),
        download_bytes_received: Cell::new(0),
        verified: Cell::new(false),
        applied: Cell::new(false),
    };

    // In-memory MCTP endpoints and packet queues.
    let ua_to_fd_packets = RefCell::new(Vec::new());
    let ua_sender = BufferSender {
        packets: &ua_to_fd_packets,
    };
    let ua_server: RefCell<Server<_, 16>> = RefCell::new(Server::new(Eid(UA_EID), 0, ua_sender));

    let fd_to_ua_packets = RefCell::new(Vec::new());
    let fd_sender = BufferSender {
        packets: &fd_to_ua_packets,
    };
    let fd_server: RefCell<Server<_, 16>> = RefCell::new(Server::new(Eid(FD_EID), 0, fd_sender));

    // Persistent UA-side listener for FD-initiated PLDM requests. Registered up
    // front so inbound requests route to it regardless of delivery ordering.
    let ua_fw_listener = ua_server
        .borrow_mut()
        .listener(PLDM_MSG_TYPE)
        .expect("register UA PLDM listener");

    // Requester transport: forwards FD-initiated requests to the remote UA.
    // Its MCTP client sends from `fd_server` (EID 42) to the UA and, before
    // each recv, pumps packets across and lets the UA answer the request.
    let requester_client = DirectClientWithPump::new(&fd_server, || {
        // Deliver the FD-originated request to the UA.
        transfer(&fd_to_ua_packets, &mut ua_server.borrow_mut());
        fd_to_ua_packets.borrow_mut().clear();
        // UA answers the request.
        serve_ua_fw_request(&ua_server, ua_fw_listener);
        // Deliver the UA response back to the FD.
        transfer(&ua_to_fd_packets, &mut fd_server.borrow_mut());
        ua_to_fd_packets.borrow_mut().clear();
    });
    let requester_transport = MctpPldmTransport::new(requester_client);

    // Responder transport: receives UA->FD commands on `fd_server` and hands
    // them to the FD. Its pre-recv pump delivers queued UA->FD packets into
    // `fd_server`.
    let responder_client = DirectClientWithPump::new(&fd_server, || {
        transfer(&ua_to_fd_packets, &mut fd_server.borrow_mut());
        ua_to_fd_packets.borrow_mut().clear();
    });
    let responder_transport = MctpPldmTransport::new(responder_client);

    let fd = RefCell::new(FirmwareDevice::init(
        &fd_ops,
        &pldm_interface::config::PLDM_PROTOCOL_CAPABILITIES,
        responder_transport,
        requester_transport,
    ));
    let fd_buf = RefCell::new([0u8; 1024]);

    // Orchestrator-facing latch: `run_terminus` marks it on each accepted
    // RequestUpdate; the assertions below drain it as `Event::UpdateRequest`.
    let update_events = RefCell::new(UpdateRequestLatch::new());

    // Run one full UA->FD->UA command round-trip and return the PLDM response
    // payload (without the MCTP framing byte).
    let ua_transact = |req_pldm: &[u8]| -> Vec<u8> {
        let req_handle = ua_server
            .borrow_mut()
            .req(FD_EID)
            .expect("allocate UA request handle to FD");
        ua_server
            .borrow_mut()
            .send(Some(req_handle), PLDM_MSG_TYPE, None, None, false, req_pldm)
            .expect("send UA command");

        // Drive the FD until its inbound queue drains; this also drives any
        // FD-initiated requests to the UA through `requester_transport`. A
        // terminating timeout means "done", not a failure.
        match fd.borrow_mut().run_terminus(
            UA_EID,
            &mut fd_buf.borrow_mut()[..],
            TIMEOUT_MILLIS,
            TIMEOUT_MILLIS,
            &mut *update_events.borrow_mut(),
        ) {
            RunTerminusResult::Completed => {}
            RunTerminusResult::StoppedByError(PldmServiceError::Mctp(e)) if e.is_timeout() => {}
            RunTerminusResult::StoppedByError(e) => panic!("firmware device failed: {e:?}"),
        }

        transfer(&fd_to_ua_packets, &mut ua_server.borrow_mut());
        fd_to_ua_packets.borrow_mut().clear();

        let mut resp = [0u8; 1024];
        let meta = ua_server
            .borrow_mut()
            .try_recv(req_handle, &mut resp)
            .expect("UA response should be available");
        let out = resp[..meta.payload_size].to_vec();
        let _ = ua_server.borrow_mut().unbind(req_handle);
        out
    };

    let mut buf = [0u8; 1024];
    let comp_ver = fw_string("v1.0");
    let mut instance_id = 0u8;

    // ---- RequestUpdate: move FD from Idle -> LearnComponents ----
    let req_update = RequestUpdateRequest::new(
        instance_id,
        PldmMsgType::Request,
        IMAGE_SIZE, // max_transfer_size
        1,          // num_of_comp
        1,          // max_outstanding_transfer_req
        0,          // pkg_data_len
        &comp_ver,
    );
    let len = req_update.encode(&mut buf).expect("encode RequestUpdate");
    let resp = ua_transact(&buf[..len]);
    assert_eq!(
        resp[3], 0,
        "RequestUpdate completion code should be success"
    );
    assert_eq!(
        update_events.borrow_mut().take(),
        Some(Event::UpdateRequest),
        "accepted RequestUpdate should latch exactly one orchestrator event"
    );
    assert_eq!(
        update_events.borrow_mut().take(),
        None,
        "the latch must not re-fire once drained"
    );

    // ---- Duplicate RequestUpdate: rejected, must not latch an event ----
    instance_id += 1;
    let dup_update = RequestUpdateRequest::new(
        instance_id,
        PldmMsgType::Request,
        IMAGE_SIZE,
        1,
        1,
        0,
        &comp_ver,
    );
    let len = dup_update
        .encode(&mut buf)
        .expect("encode duplicate RequestUpdate");
    let resp = ua_transact(&buf[..len]);
    assert_eq!(
        resp[3],
        FwUpdateCompletionCode::AlreadyInUpdateMode as u8,
        "second RequestUpdate should be rejected while in update mode"
    );
    assert_eq!(
        update_events.borrow_mut().take(),
        None,
        "a rejected RequestUpdate must not latch an orchestrator event"
    );

    // ---- PassComponentTable (Start+End): move to ReadyXfer ----
    instance_id += 1;
    let pass_comp = PassComponentTableRequest::new(
        instance_id,
        PldmMsgType::Request,
        TransferRespFlag::StartAndEnd,
        ComponentClassification::Firmware,
        0x0001, // comp_identifier
        0,      // comp_classification_index
        0,      // comp_comparison_stamp
        &comp_ver,
    );
    let len = pass_comp
        .encode(&mut buf)
        .expect("encode PassComponentTable");
    let resp = ua_transact(&buf[..len]);
    assert_eq!(
        resp[3], 0,
        "PassComponentTable completion code should be success"
    );
    assert!(
        fd_ops.component_accepted.get(),
        "FD should have accepted the passed component"
    );

    // ---- UpdateComponent: enter Download and issue the first RequestFirmwareData ----
    instance_id += 1;
    let update_comp = UpdateComponentRequest::new(
        instance_id,
        PldmMsgType::Request,
        ComponentClassification::Firmware,
        0x0001,     // comp_identifier
        0,          // comp_classification_index
        0,          // comp_comparison_stamp
        IMAGE_SIZE, // comp_image_size
        UpdateOptionFlags(0),
        &comp_ver,
    );
    let len = update_comp
        .encode(&mut buf)
        .expect("encode UpdateComponent");
    let resp = ua_transact(&buf[..len]);
    assert_eq!(
        resp[3], 0,
        "UpdateComponent completion code should be success"
    );

    // Because `FirmwareDevice` now owns both MCTP transports directly,
    // `run_terminus` autonomously drives the entire download / verify /
    // apply state machine to completion within the same call that
    // processed `UpdateComponent` above (issuing RequestFirmwareData,
    // TransferComplete, VerifyComplete, and ApplyComplete to the UA via
    // `requester_transport` in a tight loop, with nothing to interrupt it
    // since no other UA command is pending). A single GetStatus check
    // afterward should therefore already observe the FD back in ReadyXfer.
    instance_id += 1;
    let mut b = [0u8; 1024];
    let gs = GetStatusRequest::new(instance_id, PldmMsgType::Request);
    let n = gs.encode(&mut b).expect("encode GetStatus");
    let resp = ua_transact(&b[..n]);
    let status = GetStatusResponse::decode(&resp).expect("decode GetStatusResponse");
    assert_eq!(
        status.completion_code, 0,
        "GetStatus completion should be success"
    );
    assert_eq!(
        status.current_state,
        FirmwareDeviceState::ReadyXfer as u8,
        "FD should have returned to ReadyXfer once the update completed"
    );

    // ---- Final assertions: the whole FD-driven flow ran end to end ----
    assert_eq!(
        fd_ops.download_bytes_received.get(),
        IMAGE_SIZE as usize,
        "the full firmware image should have been downloaded"
    );
    assert!(fd_ops.verified.get(), "firmware should have been verified");
    assert!(fd_ops.applied.get(), "firmware should have been applied");
    assert_eq!(
        update_events.borrow_mut().take(),
        None,
        "no command after the accepted RequestUpdate should latch an event"
    );

    println!(
        "Firmware update host test completed: downloaded {} bytes, verified={}, applied={}",
        fd_ops.download_bytes_received.get(),
        fd_ops.verified.get(),
        fd_ops.applied.get()
    );
}
