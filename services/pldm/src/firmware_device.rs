// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! PLDM Firmware Device (FD) service.
//!
//! [`FirmwareDevice`] owns the PLDM firmware-update state machine and the
//! platform-specific flash operations.  It talks to the Update Agent (UA)
//! directly over MCTP via two [`MctpPldmTransport`] instances:
//!
//! * **`responder_transport`** – listens for inbound PLDM FW-update commands
//!   from the UA and replies in place.
//! * **`requester_transport`** – forwards FD-initiated PLDM requests (e.g.
//!   `RequestFirmwareData`) to the UA at `remote_eid` and receives the
//!   response.
//!
//! This lets a single process own the whole FD state machine and its MCTP
//! I/O, without needing to bridge to separate responder/requester processes
//! over platform-specific IPC.
//!
//! ## Buffer layout
//!
//! Both transports carry the same flat buffer convention used throughout this
//! crate:
//!
//! ```text
//! buf[0]          : MCTP message-type (0x01)
//! buf[1..]        : PLDM message (header + data)
//! ```
//!
//! ## Main loop
//!
//! Each iteration of [`FirmwareDevice::run_terminus`] performs two interleaved
//! phases:
//!
//! 1. **Initiator (outbound)** – while the FD is in update mode, generate the
//!    next FD-initiated request via [`CmdInterface::generate_initiator_request`],
//!    send it to the UA through `requester_transport`, and feed the response
//!    back via [`CmdInterface::process_initiator_response`].
//! 2. **Responder (inbound)** – poll `responder_transport` for an inbound UA
//!    command and reply via [`CmdInterface::handle_responder_msg`]. Polling
//!    every iteration keeps the responder path live during a transfer so the
//!    Update Agent can send `CancelUpdate` at any time.

use openprot_mctp_api::MctpClient;
use pldm_interface::cmd_interface::CmdInterface;
use pldm_interface::control_context::ProtocolCapability;
use pldm_interface::firmware_device::fd_context::FirmwareDeviceContext;
use pldm_interface::firmware_device::fd_ops::FdOps;

use crate::error::{PldmMemError, PldmServiceError};
use crate::transport::MctpPldmTransport;

/// Maximum PLDM-over-MCTP message size (MCTP-type byte + PLDM payload).
pub const FD_MAX_MSG: usize = 1024;

/// Poll timeout (milliseconds) used for the inbound responder listener while
/// an initiator (FD-to-UA) request is active.
///
/// A short, non-zero timeout lets [`FirmwareDevice::run_terminus`] check for
/// an inbound Update Agent command (e.g. `CancelUpdate`) between successive
/// outbound requests without blocking the transfer; a lack of a message
/// within this window is expected and is not treated as an error.
const RESPONDER_POLL_TIMEOUT_MILLIS: u32 = 1;

/// Receiver for update-lifecycle notifications out of the PLDM FD state
/// machine.
///
/// [`FirmwareDevice::run_terminus`] owns the PLDM state machine but has no
/// knowledge of the platform's update orchestration; this trait is the seam
/// between the two. It is deliberately PLDM-flavored (no orchestrator types)
/// so that depending on this crate never pulls in the orchestrator stack —
/// the mapping to an orchestrator event lives in an adapter crate, following
/// the same rule as the orchestrator's HAL adapters.
pub trait UpdateEventSink {
    /// The Update Agent's `RequestUpdate` was accepted: the FD moved out of
    /// `Idle` (into `LearnComponents`) and the success response has already
    /// been sent. Called exactly once per accepted `RequestUpdate`; rejected
    /// ones (`ALREADY_IN_UPDATE_MODE`, bad transfer size) never reach here
    /// because they leave the FD state unchanged.
    fn update_requested(&mut self);
}

/// Drop update notifications, for callers with no orchestration to notify.
impl UpdateEventSink for () {
    fn update_requested(&mut self) {}
}

/// Outcome of [`FirmwareDevice::run_terminus`].
pub enum RunTerminusResult {
    /// The loop exited normally (currently unreachable: `run_terminus` only
    /// returns via an error today, but this variant exists so a future,
    /// well-defined completion condition does not require an API change).
    Completed,
    /// The loop was stopped by an unrecoverable error.
    StoppedByError(PldmServiceError),
}

/// PLDM Firmware Device service.
///
/// Owns the PLDM firmware-update state machine ([`CmdInterface`]) and drives
/// it via [`run_terminus`](FirmwareDevice::run_terminus), talking directly to
/// the Update Agent over MCTP through `responder_transport` (inbound UA
/// commands) and `requester_transport` (outbound FD-initiated requests).
pub struct FirmwareDevice<'a, O: FdOps, Cr: MctpClient, Cq: MctpClient> {
    cmd_interface: CmdInterface<'a, O>,
    responder_transport: MctpPldmTransport<Cr>,
    requester_transport: MctpPldmTransport<Cq>,
}

impl<'a, O: FdOps, Cr: MctpClient, Cq: MctpClient> FirmwareDevice<'a, O, Cr, Cq> {
    /// Create a new [`FirmwareDevice`] with the given protocol capabilities
    /// and MCTP transports.
    ///
    /// `protocol_capabilities` should advertise at least
    /// [`PldmSupportedType::FwUpdate`] so that the [`CmdInterface`] accepts
    /// and routes firmware-update commands correctly. `responder_transport`
    /// answers inbound UA commands; `requester_transport` forwards
    /// FD-initiated requests to the UA.
    ///
    /// [`PldmSupportedType::FwUpdate`]: pldm_common::protocol::base::PldmSupportedType::FwUpdate
    pub fn init(
        fdops: &'a O,
        protocol_capabilities: &'a [ProtocolCapability<'a>],
        responder_transport: MctpPldmTransport<Cr>,
        requester_transport: MctpPldmTransport<Cq>,
    ) -> Self {
        FirmwareDevice {
            cmd_interface: CmdInterface::new(
                protocol_capabilities,
                FirmwareDeviceContext::new(fdops),
            ),
            responder_transport,
            requester_transport,
        }
    }

    /// Run the firmware-device service loop.
    ///
    /// Each iteration performs two interleaved phases:
    ///
    /// 1. **Initiator** — while the FD is in update mode
    ///    ([`should_start_initiator_mode`]), generate at most one outbound
    ///    request (e.g. `RequestFirmwareData`) via
    ///    [`CmdInterface::generate_initiator_request`], send it to `remote_eid`
    ///    through `requester_transport`, and feed the response back into the
    ///    state machine via [`CmdInterface::process_initiator_response`].
    /// 2. **Responder** — poll `responder_transport` for an inbound Update
    ///    Agent command and reply via [`CmdInterface::handle_responder_msg`].
    ///    While an initiator request is active, this poll uses a short
    ///    timeout so the transfer keeps making progress; a lack of a message
    ///    within that window is expected and does not end the loop. When
    ///    idle, the poll blocks for the caller-supplied `timeout_millis`.
    ///
    /// The responder listener is registered once, before the loop starts, and
    /// reused for every poll (rather than being registered and dropped on
    /// each iteration). This matters because the underlying MCTP stack
    /// requires an active listener registration to accept an inbound request
    /// of a given message type: registering a fresh listener on every poll
    /// would leave a window during initiator (FD-to-UA) traffic in which no
    /// listener is bound, silently dropping any Update Agent command that
    /// arrives in that window.
    ///
    /// This method loops indefinitely and returns only on error.
    /// A `timeout_millis` of `0` blocks indefinitely while idle.
    ///
    /// `requester_timeout_millis` bounds how long each `send_request` call
    /// (Phase 1) is allowed to wait for the UA's response to an FD-initiated
    /// request. It is intentionally a separate value from `timeout_millis`:
    /// reusing `timeout_millis` here would let a `0` (block indefinitely)
    /// idle-timeout also apply to the wait for the UA's response, which could
    /// block this call — and therefore Phase 2's responder poll — forever if
    /// the UA never replies. A `requester_timeout_millis` of `0` still blocks
    /// indefinitely if that behavior is desired; callers that want the
    /// responder path to stay live even during a stalled FD-initiated
    /// request should pass a bounded value instead.
    ///
    /// `sink` receives [`UpdateEventSink::update_requested`] once per
    /// accepted `RequestUpdate` (the FD's only `Idle` → non-`Idle`
    /// transition), after the success response has been sent. Callers with
    /// nothing to notify pass `&mut ()`.
    ///
    /// [`should_start_initiator_mode`]: pldm_interface::firmware_device::fd_context::FirmwareDeviceContext
    pub fn run_terminus(
        &mut self,
        remote_eid: u8,
        buf: &mut [u8],
        timeout_millis: u32,
        requester_timeout_millis: u32,
        sink: &mut impl UpdateEventSink,
    ) -> RunTerminusResult {
        match self.run_terminus_inner(
            remote_eid,
            buf,
            timeout_millis,
            requester_timeout_millis,
            sink,
        ) {
            Ok(()) => RunTerminusResult::Completed,
            Err(e) => RunTerminusResult::StoppedByError(e),
        }
    }

    fn run_terminus_inner(
        &mut self,
        remote_eid: u8,
        buf: &mut [u8],
        timeout_millis: u32,
        requester_timeout_millis: u32,
        sink: &mut impl UpdateEventSink,
    ) -> Result<(), PldmServiceError> {
        let mut responder_listener = self
            .responder_transport
            .responder_listener(timeout_millis)?;
        // Scratch buffer for FD-initiated (outbound) requests, reused across
        // iterations rather than re-zeroed on every loop pass.
        let mut fw_buf = [0u8; FD_MAX_MSG];

        loop {
            // Phase 1: while in initiator mode, issue at most ONE outbound
            // request per iteration. We deliberately fall through to the
            // responder poll below (no `continue`) so an Update Agent command
            // such as CancelUpdate is serviced between every RequestFirmwareData.
            let initiator_active = self.cmd_interface.fd_ctx.should_start_initiator_mode();
            if initiator_active
                && let Some(pldm_len) = self
                    .cmd_interface
                    .generate_initiator_request(&mut fw_buf)
                    .map_err(PldmServiceError::MsgHandler)?
            {
                let resp_len = self.requester_transport.send_request(
                    remote_eid,
                    pldm_len,
                    &mut fw_buf,
                    requester_timeout_millis,
                )?;
                let resp_total_len = resp_len
                    .checked_add(1)
                    .ok_or(PldmServiceError::PldmMem(PldmMemError::OverflowMaxSize))?;
                let resp = fw_buf
                    .get_mut(..resp_total_len)
                    .ok_or(PldmServiceError::PldmMem(PldmMemError::BufferTooSmall))?;
                self.cmd_interface
                    .process_initiator_response(resp)
                    .map_err(PldmServiceError::MsgHandler)?;
            }

            // Phase 2: poll for an inbound command so the responder path
            // stays live during a transfer and the Update Agent can cancel at
            // any time. `handle_responder_msg` receives the *whole* buffer
            // because responses may be larger than the request they answer
            // (e.g. GetTid: 4-byte request, 5-byte response). Commands from
            // any EID other than `remote_eid` are dropped without a response.
            let poll_timeout = if initiator_active {
                RESPONDER_POLL_TIMEOUT_MILLIS
            } else {
                timeout_millis
            };
            responder_listener.set_timeout(poll_timeout);
            // Sampled around the responder poll: `RequestUpdate` is the only
            // command that takes the FD out of `Idle`, so the false→true edge
            // of `is_update_mode()` identifies exactly one accepted
            // `RequestUpdate` (the initiator phase above never leaves `Idle`).
            let was_update_mode = self.cmd_interface.fd_ctx.is_update_mode();
            match self.responder_transport.respond_once(
                &mut responder_listener,
                buf,
                |framed_buf, _req_total_len, source_eid| {
                    // Only act on commands from the UA this instance serves;
                    // silently drop anything else (e.g. a rogue endpoint).
                    if source_eid != remote_eid {
                        return Ok(0);
                    }
                    self.cmd_interface
                        .handle_responder_msg(framed_buf)
                        .map_err(PldmServiceError::MsgHandler)
                },
            ) {
                Ok(()) => {
                    if !was_update_mode && self.cmd_interface.fd_ctx.is_update_mode() {
                        sink.update_requested();
                    }
                }
                // A short poll timeout while an initiator request is active
                // just means no UA command arrived in that window; keep
                // looping so the transfer can continue.
                Err(PldmServiceError::Mctp(e)) if initiator_active && e.is_timeout() => {}
                Err(e) => return Err(e),
            }
        }
    }
}
