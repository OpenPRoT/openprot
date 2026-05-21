// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! PLDM responder that processes incoming PLDM-over-MCTP messages.
//!
//! ## Buffer layout
//!
//! `CmdInterface` from `pldm-interface` operates on a single flat buffer
//! whose first byte is the MCTP message-type byte (0x01 for PLDM) followed
//! immediately by the PLDM header and payload:
//!
//! ```text
//! buf[0]          : MCTP message-type (0x01)
//! buf[1..]        : PLDM message (header + data)
//! ```
//!
//! The MCTP API's [`MctpListener::recv`] writes only the PLDM bytes (no
//! MCTP framing byte) into the supplied buffer.  [`PldmResponder::run_responder`]
//! therefore receives into `buf[1..]` and sets `buf[0]` before handing the
//! whole slice to `CmdInterface`.  The PLDM response (also without the MCTP
//! framing byte) is then extracted from `buf[1..resp_len]` and sent back via
//! the response channel.

use openprot_mctp_api::MctpClient;
use pldm_common::util::mctp_transport::MCTP_PLDM_MSG_TYPE;
use crate::firmware_device::{FD_IPC_MAX_MSG, UaFdCmdChannel};

use crate::error::PldmServiceError;
use crate::transport::MctpPldmTransport;

/// The MCTP message-type value used for PLDM (0x01).
pub const PLDM_MSG_TYPE: u8 = MCTP_PLDM_MSG_TYPE;

/// PLDM responder service.
#[derive(Debug, Default)]
pub struct PldmResponder;

impl PldmResponder {
    /// Create a new PLDM responder.
    pub fn new() -> Self {
        PldmResponder
    }

    /// Receive and handle one incoming PLDM message over an MCTP transport.
    ///
    /// Calls [`MctpPldmTransport::recv_and_respond`] once, passing the framed
    /// buffer to `handler`.  `handler` must return the total response length
    /// (including `buf[0]`, the MCTP type byte).
    ///
    /// A `timeout_millis` of `0` blocks indefinitely.
    pub fn run_responder<C: MctpClient>(
        &self,
        transport: &MctpPldmTransport<C>,
        fd_cmd: &impl UaFdCmdChannel,
        buf: &mut [u8],
        timeout_millis: u32
    ) -> Result<(), PldmServiceError> {
        loop {
            transport.recv_and_respond(buf, timeout_millis, |framed_buf, req_total_len| {
                // `transact` needs distinct request/response buffers, so copy
                // the framed request into scratch storage before the response
                // is written back into `framed_buf`.
                let mut req = [0u8; FD_IPC_MAX_MSG];
                let req_slice = framed_buf
                    .get(..req_total_len)
                    .ok_or(PldmServiceError::Overflow)?;
                let req_dst = req
                    .get_mut(..req_slice.len())
                    .ok_or(PldmServiceError::Overflow)?;
                req_dst.copy_from_slice(req_slice);

                fd_cmd.transact(req_dst, framed_buf)
            })?;
        }
    }
}
