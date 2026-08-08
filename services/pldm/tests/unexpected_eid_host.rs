// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! End-to-end host test wiring:
//! attacker EID + UA EID -> FirmwareDevice (direct MCTP transports) -> UA
//! all via in-memory channels/transports.

use core::cell::{Cell, RefCell};

use mctp::Eid;
use openprot_mctp_server::Server;
use openprot_pldm_service::firmware_device::FirmwareDevice;
use openprot_pldm_service::{MctpPldmTransport, PldmServiceError};
use pldm_common::codec::PldmCodec;
use pldm_common::message::control::{GetTidRequest, SetTidRequest};
use pldm_common::message::firmware_update::apply_complete::ApplyResult;
use pldm_common::message::firmware_update::get_fw_params::FirmwareParameters;
use pldm_common::message::firmware_update::get_status::ProgressPercent;
use pldm_common::message::firmware_update::transfer_complete::TransferResult;
use pldm_common::message::firmware_update::verify_complete::VerifyResult;
use pldm_common::protocol::base::PldmMsgType;
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
/// Security regression: the FD must only act on commands from the Update
/// Agent EID it was told to serve (`run_terminus`'s `remote_eid`), not from
/// any endpoint that happens to be on the bus.
#[test]
fn responder_ignores_commands_from_unexpected_eid() {
    const ATTACKER_EID: u8 = 99;

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

    // This test only exercises control commands, which never put the FD into
    // update mode, so the requester transport is never actually exercised; a
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
            Ok(()) => {}
            Err(PldmServiceError::Mctp(e)) if e.is_timeout() => {}
            Err(e) => panic!("firmware device failed: {e:?}"),
        };

    let mut buf = [0u8; 1024];
    // ---- Attacker (EID 99) sends SetTid(0x99); the FD must ignore it ----
    let set_tid = SetTidRequest::new(0, PldmMsgType::Request, 0x99);
    let req_len = set_tid.encode(&mut buf).expect("encode attacker SetTid");
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
        .expect("attacker send SetTid");
    // Run the FD; drop any response, since the attacker's queue isn't
    // pumped into `ua_server` and this send should have been dropped anyway.
    run_fd_once();
    fd_to_ua_packets.borrow_mut().clear();

    // ---- Legitimate UA (EID 8) queries the TID via GetTid ----
    let get_tid = GetTidRequest::new(1, PldmMsgType::Request);
    let req_len = get_tid.encode(&mut buf).expect("encode UA GetTid");
    let ua_handle = ua_server
        .borrow_mut()
        .req(FD_EID)
        .expect("UA allocate request handle to FD");
    ua_server
        .borrow_mut()
        .send(Some(ua_handle), 0x01, None, None, false, &buf[..req_len])
        .expect("UA send GetTid");
    run_fd_once();

    transfer(&fd_to_ua_packets, &mut ua_server.borrow_mut());
    fd_to_ua_packets.borrow_mut().clear();

    // If the attacker's SetTid(0x99) had been applied, this GetTid would
    // echo back 0x99 instead of the FD's untouched default TID.
    let mut resp = [0u8; 1024];
    let meta = ua_server
        .borrow_mut()
        .try_recv(ua_handle, &mut resp)
        .expect("GetTid response should be available");
    assert!(meta.payload_size >= 5, "GetTid response too short");
    assert_ne!(
        resp[4], 0x99,
        "FD acted on a SetTid from an unexpected EID ({ATTACKER_EID}); \
         the responder path must filter by the UA EID passed to run_terminus"
    );
}
