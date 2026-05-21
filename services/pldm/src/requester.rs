// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! PLDM requester that sends queued PLDM messages over MCTP and processes
//! responses.
//!
//! [`PldmRequester`] acts as a PLDM *initiator*: it takes a queued command,
//! sends it to a remote endpoint over MCTP, and validates the response. It
//! complements the responder-side [`PldmResponder`], which handles inbound
//! requests.
//!
//! ## Buffer layout
//!
//! The buffer passed to [`PldmRequester::run_requester`] uses the same layout
//! as [`PldmResponder`]:
//!
//! ```text
//! buf[0]          : MCTP message-type (0x01) – written by send_request
//! buf[1..]        : PLDM message (header + data)
//! ```
//!
//! [`PldmResponder`]: crate::responder::PldmResponder

use openprot_mctp_api::MctpClient;

use crate::error::PldmServiceError;
use crate::firmware_device::UaFdRspChannel;
use crate::transport::MctpPldmTransport;

/// PLDM requester service (initiator mode).
///
/// See the [module documentation](self) for the buffer layout and message
/// flow.
#[derive(Debug, Default)]
pub struct PldmRequester;

impl PldmRequester {
    /// Create a new PLDM requester.
    pub fn new() -> Self {
        PldmRequester
    }

    /// Run a blocking loop that forwards raw PLDM requests from
    /// [`FirmwareDevice`] over MCTP and returns the responses.
    ///
    /// On each iteration:
    /// 1. Receives a framed PLDM request from `fd_req` (`buf[0]` = MCTP type,
    ///    `buf[1..]` = PLDM bytes).
    /// 2. Forwards it to `remote_eid` via `transport` and receives the
    ///    response into `buf[1..]`.
    /// 3. Responds to `fd_req` with `buf[0..1+pldm_resp_len]`.
    ///
    /// A `timeout_millis` of `0` blocks indefinitely on each MCTP exchange.
    ///
    /// [`FirmwareDevice`]: crate::firmware_device::FirmwareDevice
    pub fn run_requester<C: MctpClient>(
        &self,
        fd_req: &impl UaFdRspChannel,
        transport: &MctpPldmTransport<C>,
        remote_eid: u8,
        buf: &mut [u8],
        timeout_millis: u32,
    ) -> Result<(), PldmServiceError> {
        loop {
            // Receive raw PLDM request from FirmwareDevice.
            // buf[0] = MCTP framing byte (0x01), buf[1..msg_len] = PLDM bytes.
            let msg_len = fd_req.recv(buf)?;
            let pldm_len = msg_len
                .checked_sub(1)
                .ok_or(PldmServiceError::Overflow)?;

            // Forward over MCTP; response lands in buf[1..1+pldm_resp_len].
            let pldm_resp_len =
                transport.send_request(remote_eid, pldm_len, buf, timeout_millis)?;
            let resp_total_len = pldm_resp_len
                .checked_add(1)
                .ok_or(PldmServiceError::Overflow)?;

            // Return the framed response to FirmwareDevice.
            fd_req.respond(buf.get(..resp_total_len).ok_or(PldmServiceError::Overflow)?)?;
        }
    }
}
