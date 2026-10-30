// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! MCTP IPC request dispatch.
//!
//! Decodes wire-protocol requests and dispatches them to the [`Server`].
//! This is the server-side counterpart of `openprot-mctp-client`.

use openprot_mctp_api::wire::{self, flags, MctpOp, MctpRequestHeader};
use openprot_mctp_api::{Handle, RecvMetadata, ResponseCode};

use crate::{RecvResult, Sender, Server};

/// Outcome of [`dispatch_mctp_op`].
pub enum DispatchOutcome {
    /// Response is ready; `response[..n]` bytes are filled.
    Reply(usize),
    /// No message was available for `Recv`; the call has been registered
    /// as pending. The platform must store its reply token keyed on
    /// `handle` and call [`drive_pending`] after every later event and no
    /// later than [`Server::next_recv_deadline`]. If the platform cannot
    /// hold the reply token after all, it must call
    /// [`Server::cancel_recv`] and answer the client itself.
    Pending {
        /// The handle whose recv was deferred.
        handle: Handle,
    },
}

/// Dispatch an IPC request to the MCTP server.
///
/// Decodes the request header, calls the appropriate `Server` method,
/// and encodes the response into `response`.
///
/// `now_millis` is the current monotonic time. The server's clock is
/// advanced to it before the request is handled, so what the request
/// creates (a recv deadline, the flow a `Send` opens for its response) is
/// dated now and not at the previous event.
///
/// Returns `DispatchOutcome::Reply(n)` when a response is immediately
/// available, or `DispatchOutcome::Pending { handle }` when a `Recv`
/// has been registered and will be fulfilled later by [`drive_pending`].
/// `Pending` is returned only if the registration succeeded: a `Recv` that
/// cannot be registered (table full, or one already outstanding on the
/// handle) is answered with that error instead of being left to hang.
pub fn dispatch_mctp_op<S: Sender, const N: usize>(
    request: &[u8],
    response: &mut [u8],
    server: &mut Server<S, N>,
    recv_buf: &mut [u8],
    now_millis: u64,
) -> DispatchOutcome {
    server.advance(now_millis);

    let header = match MctpRequestHeader::from_bytes(request) {
        Some(h) => h,
        None => return DispatchOutcome::Reply(encode_error(response, ResponseCode::BadArgument)),
    };

    let Some(op) = header.operation() else {
        return DispatchOutcome::Reply(encode_error(response, ResponseCode::BadArgument));
    };

    let n = match op {
        MctpOp::SetEid => match server.set_eid(header.eid) {
            Ok(()) => encode_success(response),
            Err(e) => encode_error(response, e.code),
        },

        MctpOp::GetEid => {
            let eid = server.get_eid();
            wire::encode_get_eid_response(response, eid)
                .unwrap_or_else(|_| encode_error(response, ResponseCode::InternalError))
        }

        MctpOp::Listener => match server.listener(header.msg_type) {
            Ok(handle) => wire::encode_handle_response(response, handle.0)
                .unwrap_or_else(|_| encode_error(response, ResponseCode::InternalError)),
            Err(e) => encode_error(response, e.code),
        },

        MctpOp::Req => match server.req(header.eid) {
            Ok(handle) => wire::encode_handle_response(response, handle.0)
                .unwrap_or_else(|_| encode_error(response, ResponseCode::InternalError)),
            Err(e) => encode_error(response, e.code),
        },

        MctpOp::Recv => {
            let handle = Handle(header.handle);

            match server.try_recv(handle, recv_buf) {
                Some(meta) => encode_recv(response, recv_buf, &meta),
                None => {
                    let timeout = wire::get_recv_timeout(request);
                    match server.register_recv(handle, timeout, now_millis) {
                        Ok(()) => return DispatchOutcome::Pending { handle },
                        Err(e) => encode_error(response, e.code),
                    }
                }
            }
        }

        MctpOp::Send => {
            let handle = if header.flags & flags::HAS_HANDLE != 0 {
                Some(Handle(header.handle))
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
        }

        MctpOp::Unbind => {
            let handle = Handle(header.handle);
            match server.unbind(handle) {
                Ok(()) => encode_success(response),
                Err(e) => encode_error(response, e.code),
            }
        }
    };

    DispatchOutcome::Reply(n)
}

/// Drive pending receive calls to completion.
///
/// Call this after every event the platform handles (an inbound packet fed
/// to the server, a client request dispatched) and no later than
/// [`Server::next_recv_deadline`]. Besides resolving receives, each call
/// advances the router's clock, which is why it should follow every event
/// and not only those that could resolve one.
/// For each handle that is now ready (message arrived or timed out),
/// `on_ready(handle, response_bytes)` is called with the encoded response.
/// The platform must look up its stored reply token for `handle` and send
/// `response_bytes` through it *before returning*, since `response` is
/// reused for the next ready handle (if any).
pub fn drive_pending<S: Sender, const N: usize>(
    server: &mut Server<S, N>,
    now_millis: u64,
    recv_buf: &mut [u8],
    response: &mut [u8],
    mut on_ready: impl FnMut(Handle, &[u8]),
) {
    // The router's suggested housekeeping interval is deliberately unused;
    // see `Server::next_recv_deadline`.
    let (_, ready) = server.update(now_millis, recv_buf);
    for (handle, result) in ready {
        let len = match result {
            RecvResult::Message(meta) => encode_recv(response, recv_buf, &meta),
            RecvResult::TimedOut => encode_error(response, ResponseCode::TimedOut),
        };
        on_ready(handle, response.get(..len).unwrap_or(&[]));
    }
}

/// Encode the response to a `Recv` whose payload the server copied into
/// `recv_buf`. A payload that did not fit there was not copied (see
/// `Server::try_recv`); the client gets `NoSpace` rather than a truncated
/// or stale message.
fn encode_recv(response: &mut [u8], recv_buf: &[u8], meta: &RecvMetadata) -> usize {
    let Some(payload) = recv_buf.get(..meta.payload_size) else {
        return encode_error(response, ResponseCode::NoSpace);
    };
    wire::encode_recv_response(
        response,
        meta.msg_type,
        meta.msg_ic,
        meta.remote_eid,
        meta.msg_tag,
        payload,
    )
    .unwrap_or_else(|_| encode_error(response, ResponseCode::InternalError))
}

fn encode_error(response: &mut [u8], code: ResponseCode) -> usize {
    wire::encode_error_response(response, code).unwrap_or(0)
}

fn encode_success(response: &mut [u8]) -> usize {
    wire::encode_success_response(response).unwrap_or(0)
}
