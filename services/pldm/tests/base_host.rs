// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! End-to-end host test wiring:
//! UA command -> FirmwareDevice (direct MCTP transports) -> UA
//! all via in-memory channels/transports.

use core::cell::{Cell, RefCell};

use mctp::Eid;
use openprot_mctp_server::Server;
use openprot_pldm_service::firmware_device::{FirmwareDevice, RunTerminusResult};
use openprot_pldm_service::{MctpPldmTransport, PldmServiceError};
use pldm_common::codec::PldmCodec;
use pldm_common::message::control::{GetPldmVersionRequest, GetTidRequest, SetTidRequest};
use pldm_common::message::firmware_update::apply_complete::ApplyResult;
use pldm_common::message::firmware_update::get_fw_params::FirmwareParameters;
use pldm_common::message::firmware_update::get_status::ProgressPercent;
use pldm_common::message::firmware_update::transfer_complete::TransferResult;
use pldm_common::message::firmware_update::verify_complete::VerifyResult;
use pldm_common::protocol::base::{PldmMsgType, PldmSupportedType, TransferOperationFlag};
use pldm_common::protocol::firmware_update::{ComponentResponseCode, Descriptor};
use pldm_common::util::fw_component::FirmwareComponent;
use pldm_interface::firmware_device::fd_ops::{ComponentOperation, FdOps, FdOpsError};

mod common;
use common::{transfer, BufferSender, DirectClientWithPump, FD_EID, TIMEOUT_MILLIS, UA_EID};

struct MockFdOps {
    component_accepted: Cell<bool>,
    download_bytes_received: Cell<usize>,
    verified: Cell<bool>,
    applied: Cell<bool>,
    activated: Cell<bool>,
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
        Ok(ua_transfer_size.min(512))
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
        Ok((0, 1024))
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
        self.download_bytes_received.get() >= 1024
    }

    fn query_download_progress(
        &self,
        _component: &FirmwareComponent,
        progress_percent: &mut ProgressPercent,
    ) -> Result<(), FdOpsError> {
        let pct = (self.download_bytes_received.get() * 100 / 1024) as u8;
        progress_percent
            .set_value(pct)
            .map_err(|_| FdOpsError::FwDownloadError)?;
        Ok(())
    }

    fn verify(
        &self,
        _component: &FirmwareComponent,
        _progress_percent: &mut ProgressPercent,
    ) -> Result<VerifyResult, FdOpsError> {
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
        self.activated.set(true);
        Ok(0)
    }

    fn cancel_update_component(&self, _component: &FirmwareComponent) -> Result<(), FdOpsError> {
        Ok(())
    }
}

#[test]
fn base_full_chain_via_firmware_device() {
    let fd_ops = MockFdOps {
        component_accepted: Cell::new(false),
        download_bytes_received: Cell::new(0),
        verified: Cell::new(false),
        applied: Cell::new(false),
        activated: Cell::new(false),
    };
    // In-memory MCTP endpoints: UA client side and FD responder side.
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

    // Responder transport: receives UA->FD commands directly over MCTP. Its
    // pre-recv pump delivers queued UA->FD packets into `fd_server` before
    // each receive attempt.
    let responder_client = DirectClientWithPump::new(&fd_server, || {
        transfer(&ua_to_fd_packets, &mut fd_server.borrow_mut());
        ua_to_fd_packets.borrow_mut().clear();
    });
    let responder_transport = MctpPldmTransport::new(responder_client);

    // This base test only exercises control commands (SetTid/GetTid/
    // GetPldmVersion), which never put the FD into update mode, so the
    // requester transport is never actually exercised; a client with a
    // no-op pump suffices.
    let requester_client = DirectClientWithPump::new(&fd_server, || {});
    let requester_transport = MctpPldmTransport::new(requester_client);

    let mut fd = FirmwareDevice::init(
        &fd_ops,
        &pldm_interface::config::PLDM_PROTOCOL_CAPABILITIES,
        responder_transport,
        requester_transport,
    );
    let mut fd_buf = [0u8; 1024];

    let mut ua_req_buf = [0u8; 1024];
    // ---- SetTid: verify the FD reports the TID we just set (0x42) ----
    let set_tid = SetTidRequest::new(0, PldmMsgType::Request, 0x42);

    ua_req_buf[0] = 0x01;
    let req_len = 1 + set_tid
        .encode(&mut ua_req_buf[1..])
        .expect("encode request_update");

    // Run one full UA->FD->UA request/response roundtrip.
    let req_handle = ua_server
        .borrow_mut()
        .req(FD_EID)
        .expect("allocate request handle to FD");
    ua_server
        .borrow_mut()
        .send(
            Some(req_handle),
            0x01,
            None,
            None,
            false,
            &ua_req_buf[1..req_len],
        )
        .expect("send request_update payload");

    // Runs `FirmwareDevice::run_terminus` until its inbound queue is drained.
    // `run_terminus` loops until its responder listener has nothing left, at
    // which point it returns Mctp(TimedOut); that terminating timeout means
    // "done", not a failure.
    let mut run_fd_once =
        || match fd.run_terminus(UA_EID, &mut fd_buf, TIMEOUT_MILLIS, TIMEOUT_MILLIS, &mut ()) {
            RunTerminusResult::Completed => {}
            RunTerminusResult::StoppedByError(PldmServiceError::Mctp(e)) if e.is_timeout() => {}
            RunTerminusResult::StoppedByError(e) => panic!("firmware device failed: {e:?}"),
        };

    // The responder transport's pre-recv pump delivers the queued UA->FD
    // packets into fd_server *after* its listener is registered. Delivering
    // them here would route the request before any listener exists, causing
    // it to be discarded.
    run_fd_once();

    transfer(&fd_to_ua_packets, &mut ua_server.borrow_mut());
    fd_to_ua_packets.borrow_mut().clear();

    let mut ua_resp_payload = [0u8; 1024];
    let resp_meta = ua_server
        .borrow_mut()
        .try_recv(req_handle, &mut ua_resp_payload)
        .expect("request_update response should be available");
    assert!(
        resp_meta.payload_size >= 4,
        "response should include PLDM header and completion code"
    );
    assert_eq!(
        ua_resp_payload[3], 0,
        "request_update completion code should be success"
    );

    // ---- GetTid: verify the FD reports the TID we just set (0x42) ----
    let get_tid = GetTidRequest::new(1, PldmMsgType::Request);
    ua_req_buf[0] = 0x01;
    let req_len = 1 + get_tid
        .encode(&mut ua_req_buf[1..])
        .expect("encode get_tid");

    let req_handle = ua_server
        .borrow_mut()
        .req(FD_EID)
        .expect("allocate get_tid request handle to FD");
    ua_server
        .borrow_mut()
        .send(
            Some(req_handle),
            0x01,
            None,
            None,
            false,
            &ua_req_buf[1..req_len],
        )
        .expect("send get_tid payload");

    run_fd_once();

    transfer(&fd_to_ua_packets, &mut ua_server.borrow_mut());
    fd_to_ua_packets.borrow_mut().clear();

    let mut ua_resp_payload = [0u8; 1024];
    let resp_meta = ua_server
        .borrow_mut()
        .try_recv(req_handle, &mut ua_resp_payload)
        .expect("get_tid response should be available");
    assert!(
        resp_meta.payload_size >= 5,
        "GetTid response should include header, completion code, and TID"
    );
    assert_eq!(
        ua_resp_payload[3], 0,
        "get_tid completion code should be success"
    );
    assert_eq!(
        ua_resp_payload[4], 0x42,
        "GetTid should return the TID set by SetTid"
    );

    // ---- GetPldmVersion: query the Base protocol version supported by the FD ----
    let get_version = GetPldmVersionRequest::new(
        2,
        PldmMsgType::Request,
        0,
        TransferOperationFlag::GetFirstPart,
        PldmSupportedType::Base,
    );
    ua_req_buf[0] = 0x01;
    let req_len = 1 + get_version
        .encode(&mut ua_req_buf[1..])
        .expect("encode get_pldm_version");

    let req_handle = ua_server
        .borrow_mut()
        .req(FD_EID)
        .expect("allocate get_pldm_version request handle to FD");
    ua_server
        .borrow_mut()
        .send(
            Some(req_handle),
            0x01,
            None,
            None,
            false,
            &ua_req_buf[1..req_len],
        )
        .expect("send get_pldm_version payload");

    run_fd_once();

    transfer(&fd_to_ua_packets, &mut ua_server.borrow_mut());
    fd_to_ua_packets.borrow_mut().clear();

    let mut ua_resp_payload = [0u8; 1024];
    let resp_meta = ua_server
        .borrow_mut()
        .try_recv(req_handle, &mut ua_resp_payload)
        .expect("get_pldm_version response should be available");
    // hdr(3) + completion(1) + next_transfer_handle(4) + transfer_rsp_flag(1) + version(4) = 13
    assert!(
        resp_meta.payload_size >= 13,
        "GetPldmVersion response should include header, completion code, and version data"
    );
    let resp_version: u32 = u32::from_le_bytes(
        ua_resp_payload[9..13]
            .try_into()
            .expect("version data should be 4 bytes"),
    );

    assert!(
        pldm_interface::config::PLDM_PROTOCOL_CAPABILITIES[0].protocol_version == resp_version,
        "Returned Version is incorrect"
    );
    assert_eq!(
        ua_resp_payload[3], 0,
        "get_pldm_version completion code should be success"
    );
}
