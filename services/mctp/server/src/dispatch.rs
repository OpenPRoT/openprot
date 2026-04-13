// Licensed under the Apache-2.0 license

//! MCTP IPC request dispatch.
//!
//! Decodes wire-protocol requests and dispatches them to the [`Server`].
//! This is the server-side counterpart of `openprot-mctp-client`.

use openprot_mctp_api::wire::{
    self, flags, MctpOp, MctpRequestHeader,
};
use openprot_mctp_api::ResponseCode;

use crate::{Sender, Server};

/// Dispatch an IPC request to the MCTP server.
///
/// Decodes the request header, calls the appropriate `Server` method,
/// and encodes the response into `response`.
///
/// Returns `Some(len)` when the caller should immediately call
/// `channel_respond` with `response[..len]`.  Returns `None` for a
/// deferred `Recv`: the client is parked in `server.outstanding` and will
/// be unblocked by a subsequent call to `server.update()` after an inbound
/// MCTP packet arrives.
///
/// This is the MCTP equivalent of `dispatch_i2c_op` in the I2C server.
pub fn dispatch_mctp_op<S: Sender, const N: usize>(
    request: &[u8],
    response: &mut [u8],
    server: &mut Server<S, N>,
    recv_buf: &mut [u8],
) -> Option<usize> {
    let header = match MctpRequestHeader::from_bytes(request) {
        Some(h) => h,
        None => return Some(encode_error(response, ResponseCode::BadArgument)),
    };

    let Some(op) = header.operation() else {
        return Some(encode_error(response, ResponseCode::BadArgument));
    };

    match op {
        MctpOp::SetEid => Some(match server.set_eid(header.eid) {
            Ok(()) => encode_success(response),
            Err(e) => encode_error(response, e.code),
        }),

        MctpOp::GetEid => Some({
            let eid = server.get_eid();
            wire::encode_get_eid_response(response, eid)
                .unwrap_or_else(|_| encode_error(response, ResponseCode::InternalError))
        }),

        MctpOp::Listener => Some(match server.listener(header.msg_type) {
            Ok(handle) => wire::encode_handle_response(response, handle.0)
                .unwrap_or_else(|_| encode_error(response, ResponseCode::InternalError)),
            Err(e) => encode_error(response, e.code),
        }),

        MctpOp::Req => Some(match server.req(header.eid) {
            Ok(handle) => wire::encode_handle_response(response, handle.0)
                .unwrap_or_else(|_| encode_error(response, ResponseCode::InternalError)),
            Err(e) => encode_error(response, e.code),
        }),

        MctpOp::Recv => {
            let handle = openprot_mctp_api::Handle(header.handle);

            if let Some(meta) = server.try_recv(handle, recv_buf) {
                // Message already queued — reply immediately.
                let payload = &recv_buf[..meta.payload_size];
                Some(
                    wire::encode_recv_response(
                        response,
                        meta.msg_type,
                        meta.msg_ic,
                        meta.remote_eid,
                        meta.msg_tag,
                        payload,
                    )
                    .unwrap_or_else(|_| encode_error(response, ResponseCode::InternalError)),
                )
            } else {
                // No message yet — park the client.  Decode timeout_millis
                // from the 4-byte payload appended by encode_recv().
                let payload = wire::get_request_payload(request);
                let timeout_millis = if payload.len() >= 4 {
                    u32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]])
                } else {
                    0
                };
                server.register_recv(handle, timeout_millis, 0).ok();
                // Return None — main loop must NOT call channel_respond yet.
                None
            }
        }

        MctpOp::Send => Some({
            let handle = if header.flags & flags::HAS_HANDLE != 0 {
                Some(openprot_mctp_api::Handle(header.handle))
            } else {
                None
            };
            let eid = if header.flags & flags::HAS_EID != 0 {
                Some(header.eid)
            } else {
                None
            };
            let tag = if header.flags & flags::HAS_TAG != 0 {
                Some(header.tag)
            } else {
                None
            };
            let ic = header.flags & flags::IC != 0;
            let payload = wire::get_request_payload(request);

            match server.send(handle, header.msg_type, eid, tag, ic, payload) {
                Ok(tag_val) => wire::encode_send_response(response, tag_val)
                    .unwrap_or_else(|_| encode_error(response, ResponseCode::InternalError)),
                Err(e) => encode_error(response, e.code),
            }
        }),

        MctpOp::Unbind => Some({
            let handle = openprot_mctp_api::Handle(header.handle);
            match server.unbind(handle) {
                Ok(()) => encode_success(response),
                Err(e) => encode_error(response, e.code),
            }
        }),
    }
}

fn encode_error(response: &mut [u8], code: ResponseCode) -> usize {
    wire::encode_error_response(response, code).unwrap_or(0)
}

fn encode_success(response: &mut [u8]) -> usize {
    wire::encode_success_response(response).unwrap_or(0)
}
