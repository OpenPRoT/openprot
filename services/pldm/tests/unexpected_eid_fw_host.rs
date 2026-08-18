// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! End-to-end host test wiring:
//! attacker EID + UA EID -> FirmwareDevice (direct MCTP transports) -> UA
//! all via in-memory channels/transports.

use core::cell::{Cell, RefCell};

use mctp::Eid;
use openprot_mctp_server::Server;
use openprot_pldm_service::firmware_device::{FirmwareDevice, RunTerminusResult};
use openprot_pldm_service::{MctpPldmTransport, PldmServiceError};
use pldm_common::codec::PldmCodec;
use pldm_common::message::firmware_update::apply_complete::ApplyResult;
use pldm_common::message::firmware_update::get_fw_params::FirmwareParameters;
use pldm_common::message::firmware_update::get_status::{
    GetStatusRequest, GetStatusResponse, ProgressPercent,
};
use pldm_common::message::firmware_update::request_update::RequestUpdateRequest;
use pldm_common::message::firmware_update::transfer_complete::TransferResult;
use pldm_common::message::firmware_update::verify_complete::VerifyResult;
use pldm_common::protocol::base::PldmMsgType;
use pldm_common::protocol::firmware_update::{
    ComponentResponseCode, Descriptor, FirmwareDeviceState,
};
use pldm_common::protocol::firmware_update::{
    PldmFirmwareString, VersionStringType, PLDM_FWUP_IMAGE_SET_VER_STR_MAX_LEN,
};
use pldm_common::util::fw_component::FirmwareComponent;
use pldm_interface::firmware_device::fd_ops::{ComponentOperation, FdOps, FdOpsError};

mod common;
use common::{transfer, BufferSender, DirectClientWithPump, FD_EID, TIMEOUT_MILLIS, UA_EID};

const PLDM_MSG_TYPE: u8 = 0x01;
/// Total firmware image size (bytes) advertised in `UpdateComponent`.
const IMAGE_SIZE: u32 = 1024;

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

/// Security regression: the FD must only act on commands from the Update
/// Agent EID it was told to serve (`run_terminus`'s `remote_eid`), not from
/// any endpoint that happens to be on the bus.
#[test]
fn responder_ignores_fw_commands_from_unexpected_eid() {
    const ATTACKER_EID: u8 = 99;
    let comp_ver = fw_string("v1.0");

    let fd_ops = MockFdOps {
        component_accepted: Cell::new(false),
        download_bytes_received: Cell::new(0),
        verified: Cell::new(false),
        applied: Cell::new(false),
        activated: Cell::new(false),
    };

    // In-memory MCTP endpoints: legitimate UA, attacker, and the FD responder.
    let ua_to_fd_packets = RefCell::new(Vec::new());
    let ua_server: RefCell<Server<_, 16>> = RefCell::new(Server::new(
        Eid(UA_EID),
        0,
        BufferSender {
            packets: &ua_to_fd_packets,
        },
    ));

    let attacker_to_fd_packets = RefCell::new(Vec::new());
    let attacker_server: RefCell<Server<_, 16>> = RefCell::new(Server::new(
        Eid(ATTACKER_EID),
        0,
        BufferSender {
            packets: &attacker_to_fd_packets,
        },
    ));

    let fd_to_ua_packets = RefCell::new(Vec::new());
    let fd_server: RefCell<Server<_, 16>> = RefCell::new(Server::new(
        Eid(FD_EID),
        0,
        BufferSender {
            packets: &fd_to_ua_packets,
        },
    ));

    // Responder transport: receives UA->FD and attacker->FD commands directly
    // over MCTP. Its pre-recv pump delivers BOTH queues into `fd_server`
    // before each receive attempt, so whichever is pending is seen.
    let responder_client = DirectClientWithPump::new(&fd_server, || {
        transfer(&ua_to_fd_packets, &mut fd_server.borrow_mut());
        ua_to_fd_packets.borrow_mut().clear();
        transfer(&attacker_to_fd_packets, &mut fd_server.borrow_mut());
        attacker_to_fd_packets.borrow_mut().clear();
    });
    let responder_transport = MctpPldmTransport::new(responder_client);

    // This test exercises FW Update commands, but never puts the FD into
    // update mode, so the requester transport should never actually be exercised; a
    // client with a no-op pump suffices.
    let requester_transport = MctpPldmTransport::new(DirectClientWithPump::new(&fd_server, || {}));

    let mut fd = FirmwareDevice::init(
        &fd_ops,
        &pldm_interface::config::PLDM_PROTOCOL_CAPABILITIES,
        responder_transport,
        requester_transport,
    );
    let mut fd_buf = [0u8; 1024];

    // Runs `FirmwareDevice::run_terminus` until its inbound queue is drained.
    // `run_terminus` loops until its responder listener has nothing left, at
    // which point it returns Mctp(TimedOut); that terminating timeout means
    // "done", not a failure. `UA_EID` is the only EID `run_terminus` is told
    // to serve, so commands from `ATTACKER_EID` must be ignored below.
    let mut run_fd_once =
        || match fd.run_terminus(UA_EID, &mut fd_buf, TIMEOUT_MILLIS, TIMEOUT_MILLIS) {
            RunTerminusResult::Completed => {}
            RunTerminusResult::StoppedByError(PldmServiceError::Mctp(e)) if e.is_timeout() => {}
            RunTerminusResult::StoppedByError(e) => panic!("firmware device failed: {e:?}"),
        };

    let mut buf = [0u8; 1024];
    // ---- Attacker (EID 99) sends RequestUpdate; the FD must ignore it and FD states must not move----
    // ---- RequestUpdate: move FD from Idle -> LearnComponents ----
    let request_update = RequestUpdateRequest::new(
        0,
        PldmMsgType::Request,
        IMAGE_SIZE, // max_transfer_size
        1,          // num_of_comp
        1,          // max_outstanding_transfer_req
        0,          // pkg_data_len
        &comp_ver,
    );
    let req_len = request_update
        .encode(&mut buf)
        .expect("encode attacker RequestUpdate");
    let attacker_handle = attacker_server
        .borrow_mut()
        .req(FD_EID)
        .expect("attacker allocate request handle to FD");
    attacker_server
        .borrow_mut()
        .send(
            Some(attacker_handle),
            0x01,
            None,
            None,
            false,
            &buf[..req_len],
        )
        .expect("attacker send RequestUpdate");
    // Run the FD; drop any response, since the attacker's queue isn't
    // pumped into `ua_server` and this send should have been dropped anyway.
    run_fd_once();
    fd_to_ua_packets.borrow_mut().clear();

    // ---- Legitimate UA (EID 8) queries the FD state via GetStatus ----
    // The FD should still be in Idle, since it ignored the attacker's RequestUpdate.
    let gs_req = GetStatusRequest::new(0, PldmMsgType::Request);
    let gs_req_len = gs_req.encode(&mut buf).expect("encode GetStatus");
    // payload (without the MCTP framing byte).
    let req_handle = ua_server
        .borrow_mut()
        .req(FD_EID)
        .expect("allocate UA request handle to FD");
    ua_server
        .borrow_mut()
        .send(
            Some(req_handle),
            PLDM_MSG_TYPE,
            None,
            None,
            false,
            &buf[..gs_req_len],
        )
        .expect("send UA command");
    run_fd_once();

    transfer(&fd_to_ua_packets, &mut ua_server.borrow_mut());
    fd_to_ua_packets.borrow_mut().clear();

    let mut resp = [0u8; 1024];
    ua_server
        .borrow_mut()
        .try_recv(req_handle, &mut resp)
        .expect("UA response should be available");

    let status = GetStatusResponse::decode(&resp).expect("decode GetStatusResponse");
    assert_eq!(
        status.completion_code, 0,
        "GetStatus completion should be success"
    );
    assert_eq!(
        status.current_state,
        FirmwareDeviceState::Idle as u8,
        "FD should stay at Idle, ignoring the attacker's RequestUpdate."
    );
}
