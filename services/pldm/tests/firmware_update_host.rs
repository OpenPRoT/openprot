// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! End-to-end host test for a full PLDM firmware-update flow.
//!
//! This extends the wiring exercised by `base_host.rs`
//! (`PldmResponder::run_responder` + `FirmwareDevice::run_terminus`) with the
//! FD-initiated request path (`PldmRequester::run_requester`).  When the FD
//! enters update mode it drives the download / verify / apply state machine by
//! issuing PLDM requests (`RequestFirmwareData`, `TransferComplete`,
//! `VerifyComplete`, `ApplyComplete`) back to the Update Agent.  Those requests
//! flow:
//!
//! ```text
//!  UA cmd --> PldmResponder --> FirmwareDevice.run_terminus
//!                                   |
//!                                   v (initiator mode)
//!                              PldmRequester.run_requester --> remote UA (over MCTP)
//! ```
//!
//! All transports are backed by in-memory MCTP `Server`s and packet queues.

use core::cell::{Cell, RefCell};

use mctp::{Eid, Tag};
use mctp_lib::fragment::{Fragmenter, SendOutput};
use mctp_lib::Sender;
use openprot_mctp_api::{Handle, MctpClient, MctpError, RecvMetadata, ResponseCode};
use openprot_mctp_server::Server;
use openprot_pldm_service::firmware_device::{
    FdUaCmdChannel, FdUaRspChannel, FirmwareDevice, UaFdCmdChannel, UaFdRspChannel,
};
use openprot_pldm_service::{MctpPldmTransport, PldmRequester, PldmResponder, PldmServiceError};
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
    PldmFirmwareString, UpdateOptionFlags, VersionStringType, PLDM_FWUP_IMAGE_SET_VER_STR_MAX_LEN,
};
use pldm_common::util::fw_component::FirmwareComponent;
use pldm_interface::firmware_device::fd_ops::{ComponentOperation, FdOps, FdOpsError};

const FD_EID: u8 = 42;
const UA_EID: u8 = 8;
const TIMEOUT_MILLIS: u32 = 0;
const PLDM_MSG_TYPE: u8 = 0x01;

/// Total firmware image size (bytes) advertised in `UpdateComponent`.
const IMAGE_SIZE: u32 = 1024;

// ---------------------------------------------------------------------------
// In-memory MCTP plumbing (shared shape with base_host.rs)
// ---------------------------------------------------------------------------

struct BufferSender<'a> {
    packets: &'a RefCell<Vec<Vec<u8>>>,
}

impl Sender for BufferSender<'_> {
    fn send_vectored(
        &mut self,
        mut fragmenter: Fragmenter,
        payload: &[&[u8]],
    ) -> mctp::Result<Tag> {
        loop {
            // Fragmenter needs the payload MTU (255) plus the 4-byte header.
            let mut buf = [0u8; 255 + 4];
            match fragmenter.fragment_vectored(payload, &mut buf) {
                SendOutput::Packet(p) => self.packets.borrow_mut().push(p.to_vec()),
                SendOutput::Complete { tag, .. } => return Ok(tag),
                SendOutput::Error { err, .. } => return Err(err),
            }
        }
    }

    fn get_mtu(&self) -> usize {
        255
    }
}

fn transfer<S: Sender, const N: usize>(packets: &RefCell<Vec<Vec<u8>>>, dest: &mut Server<S, N>) {
    let pkts = packets.borrow();
    for pkt in pkts.iter() {
        dest.inbound(pkt).expect("inbound should accept packet");
    }
}

struct DirectClientWithPump<'a, S: Sender, const N: usize, F: FnMut()> {
    server: &'a RefCell<Server<S, N>>,
    pre_recv_pump: RefCell<F>,
}

impl<'a, S: Sender, const N: usize, F: FnMut()> DirectClientWithPump<'a, S, N, F> {
    fn new(server: &'a RefCell<Server<S, N>>, pre_recv_pump: F) -> Self {
        Self {
            server,
            pre_recv_pump: RefCell::new(pre_recv_pump),
        }
    }
}

impl<S: Sender, const N: usize, F: FnMut()> MctpClient for DirectClientWithPump<'_, S, N, F> {
    fn req(&self, eid: u8) -> Result<Handle, MctpError> {
        self.server.borrow_mut().req(eid)
    }

    fn listener(&self, msg_type: u8) -> Result<Handle, MctpError> {
        self.server.borrow_mut().listener(msg_type)
    }

    fn get_eid(&self) -> u8 {
        self.server.borrow().get_eid()
    }

    fn set_eid(&self, eid: u8) -> Result<(), MctpError> {
        self.server.borrow_mut().set_eid(eid)
    }

    fn recv(
        &self,
        handle: Handle,
        _timeout_millis: u32,
        buf: &mut [u8],
    ) -> Result<RecvMetadata, MctpError> {
        (self.pre_recv_pump.borrow_mut())();

        self.server
            .borrow_mut()
            .try_recv(handle, buf)
            .ok_or(MctpError::from_code(ResponseCode::TimedOut))
    }

    fn send(
        &self,
        handle: Option<Handle>,
        msg_type: u8,
        eid: Option<u8>,
        tag: Option<u8>,
        integrity_check: bool,
        buf: &[u8],
    ) -> Result<u8, MctpError> {
        self.server
            .borrow_mut()
            .send(handle, msg_type, eid, tag, integrity_check, buf)
    }

    fn drop_handle(&self, handle: Handle) {
        let _ = self.server.borrow_mut().unbind(handle);
    }
}

// ---------------------------------------------------------------------------
// One-shot IPC channels used to bridge the service loops into direct calls.
// ---------------------------------------------------------------------------

/// FD command channel (`fd_cmd`): carries UA->FD commands into `run_terminus`.
struct OneShotFdRsp {
    req: RefCell<Option<Vec<u8>>>,
    resp: RefCell<Option<Vec<u8>>>,
    served: Cell<bool>,
}

impl OneShotFdRsp {
    fn new() -> Self {
        Self {
            req: RefCell::new(None),
            resp: RefCell::new(None),
            served: Cell::new(false),
        }
    }

    fn load_req(&self, req: &[u8]) {
        *self.req.borrow_mut() = Some(req.to_vec());
        *self.resp.borrow_mut() = None;
        self.served.set(false);
    }

    fn take_resp(&self) -> Result<Vec<u8>, PldmServiceError> {
        self.resp.borrow_mut().take().ok_or(PldmServiceError::Ipc)
    }
}

impl FdUaRspChannel for OneShotFdRsp {
    fn recv(&self, buf: &mut [u8], _timeout_millis: u32) -> Result<usize, PldmServiceError> {
        if self.served.get() {
            return Err(PldmServiceError::Ipc);
        }

        let req = self.req.borrow_mut().take().ok_or(PldmServiceError::Ipc)?;
        if req.len() > buf.len() {
            return Err(PldmServiceError::Overflow);
        }

        buf[..req.len()].copy_from_slice(&req);
        self.served.set(true);
        Ok(req.len())
    }

    fn try_recv(&self, buf: &mut [u8]) -> Result<Option<usize>, PldmServiceError> {
        // One-shot: after its single request is served, signal `run_terminus`
        // to stop looping by returning `Ipc` (tolerated by the bridge below).
        if self.served.get() {
            return Err(PldmServiceError::Ipc);
        }

        let Some(req) = self.req.borrow_mut().take() else {
            return Err(PldmServiceError::Ipc);
        };
        if req.len() > buf.len() {
            return Err(PldmServiceError::Overflow);
        }

        buf[..req.len()].copy_from_slice(&req);
        self.served.set(true);
        Ok(Some(req.len()))
    }

    fn respond(&self, buf: &[u8]) -> Result<(), PldmServiceError> {
        *self.resp.borrow_mut() = Some(buf.to_vec());
        Ok(())
    }

    fn wait_readable(&self, _timeout_millis: u32) -> Result<(), PldmServiceError> {
        // In-memory one-shot channel: nothing to wait on.
        Ok(())
    }
}

/// UA request channel (`fw_req` peer): carries FD-initiated requests into
/// `run_requester`, which forwards them over MCTP to the remote UA.
struct OneShotUaReq {
    req: RefCell<Option<Vec<u8>>>,
    resp: RefCell<Option<Vec<u8>>>,
    served: Cell<bool>,
}

impl OneShotUaReq {
    fn new() -> Self {
        Self {
            req: RefCell::new(None),
            resp: RefCell::new(None),
            served: Cell::new(false),
        }
    }

    fn load_req(&self, req: &[u8]) {
        *self.req.borrow_mut() = Some(req.to_vec());
        *self.resp.borrow_mut() = None;
        self.served.set(false);
    }

    fn take_resp(&self) -> Result<Vec<u8>, PldmServiceError> {
        self.resp.borrow_mut().take().ok_or(PldmServiceError::Ipc)
    }
}

impl UaFdRspChannel for OneShotUaReq {
    fn recv(&self, buf: &mut [u8]) -> Result<usize, PldmServiceError> {
        // One-shot: after the queued request is served, return `Ipc` so
        // `run_requester` exits its loop (tolerated by the bridge below).
        if self.served.get() {
            return Err(PldmServiceError::Ipc);
        }

        let req = self.req.borrow_mut().take().ok_or(PldmServiceError::Ipc)?;
        if req.len() > buf.len() {
            return Err(PldmServiceError::Overflow);
        }

        buf[..req.len()].copy_from_slice(&req);
        self.served.set(true);
        Ok(req.len())
    }

    fn respond(&self, buf: &[u8]) -> Result<(), PldmServiceError> {
        *self.resp.borrow_mut() = Some(buf.to_vec());
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Fake firmware-device operations.
// ---------------------------------------------------------------------------

struct FakeFdOps {
    component_accepted: Cell<bool>,
    download_bytes_received: Cell<usize>,
    verified: Cell<bool>,
    applied: Cell<bool>,
}

impl FdOps for FakeFdOps {
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

/// Client-side bridge: `FirmwareDevice.run_terminus` calls this to issue an
/// FD-initiated request; it drives `PldmRequester.run_requester` to forward the
/// request to the remote UA over MCTP and return the response.
struct RequesterBridge<'a, C: MctpClient> {
    chan: &'a OneShotUaReq,
    requester: &'a RefCell<PldmRequester>,
    transport: &'a MctpPldmTransport<C>,
    buf: &'a RefCell<[u8; 1024]>,
    remote_eid: u8,
}

impl<C: MctpClient> FdUaCmdChannel for RequesterBridge<'_, C> {
    fn transact(&self, req: &[u8], resp: &mut [u8]) -> Result<usize, PldmServiceError> {
        self.chan.load_req(req);
        match self.requester.borrow_mut().run_requester(
            self.chan,
            self.transport,
            self.remote_eid,
            &mut self.buf.borrow_mut()[..],
            TIMEOUT_MILLIS,
        ) {
            Ok(()) | Err(PldmServiceError::Ipc) => {}
            Err(e) => return Err(e),
        }

        let out = self.chan.take_resp()?;
        if out.len() > resp.len() {
            return Err(PldmServiceError::Overflow);
        }
        resp[..out.len()].copy_from_slice(&out);
        Ok(out.len())
    }
}

/// Server-side bridge: `PldmResponder.run_responder` calls this to forward a
/// UA->FD command into `FirmwareDevice.run_terminus` and return its response.
struct ResponderToFdBridge<'a, T: FdUaCmdChannel> {
    chan: &'a OneShotFdRsp,
    fd: &'a RefCell<FirmwareDevice<'a, FakeFdOps>>,
    fw_req: &'a T,
    fd_buf: &'a RefCell<[u8; 1024]>,
}

impl<T: FdUaCmdChannel> UaFdCmdChannel for ResponderToFdBridge<'_, T> {
    fn transact(&self, req: &[u8], resp: &mut [u8]) -> Result<usize, PldmServiceError> {
        self.chan.load_req(req);
        match self.fd.borrow_mut().run_terminus(
            self.chan,
            self.fw_req,
            &mut self.fd_buf.borrow_mut()[..],
            TIMEOUT_MILLIS,
        ) {
            Ok(()) | Err(PldmServiceError::Ipc) => {}
            Err(e) => return Err(e),
        }

        let out = self.chan.take_resp()?;
        if out.len() > resp.len() {
            return Err(PldmServiceError::Overflow);
        }
        resp[..out.len()].copy_from_slice(&out);
        Ok(out.len())
    }
}

// ---------------------------------------------------------------------------
// Test
// ---------------------------------------------------------------------------

#[test]
fn firmware_update_full_flow_via_requester() {
    let fd_ops = FakeFdOps {
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

    let fd = RefCell::new(FirmwareDevice::init(
        &fd_ops,
        &pldm_interface::config::PLDM_PROTOCOL_CAPABILITIES,
    ));
    let fd_buf = RefCell::new([0u8; 1024]);

    let fd_rsp_bridge_chan = OneShotFdRsp::new();
    let ua_req_bridge_chan = OneShotUaReq::new();

    // Requester side: forwards FD-initiated requests to the remote UA. Its MCTP
    // client sends from `fd_server` (EID 42) to the UA and, before each recv,
    // pumps packets across and lets the UA answer the request.
    let requester = RefCell::new(PldmRequester::new());
    let requester_buf = RefCell::new([0u8; 1024]);
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

    let fd_to_req_bridge = RequesterBridge {
        chan: &ua_req_bridge_chan,
        requester: &requester,
        transport: &requester_transport,
        buf: &requester_buf,
        remote_eid: UA_EID,
    };

    let responder_bridge = ResponderToFdBridge {
        chan: &fd_rsp_bridge_chan,
        fd: &fd,
        fw_req: &fd_to_req_bridge,
        fd_buf: &fd_buf,
    };

    // Responder side: receives UA->FD commands on `fd_server` and hands them to
    // the FD. Its pre-recv pump delivers queued UA->FD packets into `fd_server`.
    let responder_client = DirectClientWithPump::new(&fd_server, || {
        transfer(&ua_to_fd_packets, &mut fd_server.borrow_mut());
        ua_to_fd_packets.borrow_mut().clear();
    });
    let responder_transport = MctpPldmTransport::new(responder_client);
    let responder = RefCell::new(PldmResponder::new());
    let responder_buf = RefCell::new([0u8; 1024]);

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

        // Drive the responder until its inbound queue drains. A terminating
        // timeout means "done", not a failure.
        match responder.borrow_mut().run_responder(
            &responder_transport,
            &responder_bridge,
            &mut responder_buf.borrow_mut()[..],
            TIMEOUT_MILLIS,
        ) {
            Ok(()) => {}
            Err(PldmServiceError::Mctp(e)) if e.is_timeout() => {}
            Err(e) => panic!("responder failed: {e:?}"),
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

    // ---- GetStatus x3: pump the FD-driven download/verify/apply state machine ----
    // Each GetStatus advances the FD initiator state machine by issuing
    // FD-initiated requests (RequestFirmwareData / TransferComplete /
    // VerifyComplete / ApplyComplete) to the UA via run_requester.
    let get_state = |instance_id: u8, transact: &dyn Fn(&[u8]) -> Vec<u8>| -> u8 {
        let mut b = [0u8; 1024];
        let gs = GetStatusRequest::new(instance_id, PldmMsgType::Request);
        let n = gs.encode(&mut b).expect("encode GetStatus");
        let resp = transact(&b[..n]);
        let status = GetStatusResponse::decode(&resp).expect("decode GetStatusResponse");
        assert_eq!(
            status.completion_code, 0,
            "GetStatus completion should be success"
        );
        status.current_state
    };

    instance_id += 1;
    let state1 = get_state(instance_id, &ua_transact);
    assert_eq!(
        state1,
        FirmwareDeviceState::Download as u8,
        "FD should be downloading firmware"
    );

    instance_id += 1;
    let state2 = get_state(instance_id, &ua_transact);
    assert_eq!(
        state2,
        FirmwareDeviceState::Apply as u8,
        "FD should be applying firmware after verify completes"
    );

    instance_id += 1;
    let state3 = get_state(instance_id, &ua_transact);
    assert_eq!(
        state3,
        FirmwareDeviceState::ReadyXfer as u8,
        "FD should return to ReadyXfer after apply completes"
    );

    // ---- Final assertions: the whole FD-driven flow ran end to end ----
    assert_eq!(
        fd_ops.download_bytes_received.get(),
        IMAGE_SIZE as usize,
        "the full firmware image should have been downloaded"
    );
    assert!(fd_ops.verified.get(), "firmware should have been verified");
    assert!(fd_ops.applied.get(), "firmware should have been applied");

    println!(
        "Firmware update host test completed: downloaded {} bytes, verified={}, applied={}",
        fd_ops.download_bytes_received.get(),
        fd_ops.verified.get(),
        fd_ops.applied.get()
    );
}
