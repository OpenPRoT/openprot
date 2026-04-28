// Licensed under the Apache-2.0 license

#![no_std]

pub mod runtime;

use usart_api::backend::{BackendError, IrqMask, UsartBackend};
use usart_api::{UsartError, UsartOp, UsartRequestHeader, UsartResponseHeader};

pub const MAX_REQUEST_SIZE: usize = 512;
pub const MAX_RESPONSE_SIZE: usize = 512;

/// Outcome of a single dispatch call.
pub enum DispatchOutcome {
    /// Response is ready; caller should send `response[..len]` immediately.
    Respond(usize),
    /// No data was available yet.  The runtime stored the pending request and
    /// will respond once the RX interrupt fires.  Caller must NOT call
    /// `channel_respond` for this request.
    Queued,
}

pub fn dispatch_request<B: UsartBackend>(
    backend: &mut B,
    pending: &mut runtime::PendingRead,
    client_channel: u32,
    request: &[u8],
    response: &mut [u8],
) -> DispatchOutcome {
    if request.len() < UsartRequestHeader::SIZE {
        return DispatchOutcome::Respond(encode_error(response, UsartError::InvalidOperation));
    }

    let hdr_bytes = &request[..UsartRequestHeader::SIZE];
    let Some(hdr) = zerocopy::Ref::<_, UsartRequestHeader>::from_bytes(hdr_bytes).ok() else {
        return DispatchOutcome::Respond(encode_error(response, UsartError::InvalidOperation));
    };

    let op = match hdr.operation() {
        Ok(op) => op,
        Err(e) => return DispatchOutcome::Respond(encode_error(response, e)),
    };

    let payload_len = hdr.payload_length();
    if request.len() < UsartRequestHeader::SIZE + payload_len {
        return DispatchOutcome::Respond(encode_error(response, UsartError::InvalidOperation));
    }
    let payload = &request[UsartRequestHeader::SIZE..UsartRequestHeader::SIZE + payload_len];

    match op {
        UsartOp::Configure => {
            let baud = ((hdr.arg1_value() as u32) << 16) | (hdr.arg0_value() as u32);
            let cfg = usart_api::backend::UsartConfig {
                baud_rate: baud,
                parity: usart_api::backend::Parity::None,
                stop_bits: 1,
            };
            match backend.configure(cfg) {
                Ok(()) => DispatchOutcome::Respond(encode_success(response, &[])),
                Err(e) => DispatchOutcome::Respond(encode_error(response, e.into())),
            }
        }
        UsartOp::Write => match backend.write(payload) {
            Ok(_) => DispatchOutcome::Respond(encode_success(response, &[])),
            Err(e) => DispatchOutcome::Respond(encode_error(response, e.into())),
        },
        UsartOp::Read => {
            let req_len = hdr.arg0_value() as usize;
            let payload_offset = UsartResponseHeader::SIZE;
            let payload_capacity = response.len().saturating_sub(payload_offset);
            let read_buf_len = core::cmp::min(req_len, payload_capacity);

            match backend.read(&mut response[payload_offset..payload_offset + read_buf_len]) {
                Ok(n) => {
                    let hdr = UsartResponseHeader::success(n as u16);
                    response[..UsartResponseHeader::SIZE]
                        .copy_from_slice(zerocopy::IntoBytes::as_bytes(&hdr));
                    DispatchOutcome::Respond(UsartResponseHeader::SIZE + n)
                }
                Err(e) => DispatchOutcome::Respond(encode_error(response, e.into())),
            }
        }
        UsartOp::TryRead => {
            let req_len = hdr.arg0_value() as usize;
            let payload_offset = UsartResponseHeader::SIZE;
            let payload_capacity = response.len().saturating_sub(payload_offset);
            let read_buf_len = core::cmp::min(req_len, payload_capacity);

            // Attempt immediate non-blocking drain of the RX FIFO.
            match backend.try_read(&mut response[payload_offset..payload_offset + read_buf_len]) {
                Ok(n) => {
                    // Data available right now — respond immediately.
                    let hdr = UsartResponseHeader::success(n as u16);
                    response[..UsartResponseHeader::SIZE]
                        .copy_from_slice(zerocopy::IntoBytes::as_bytes(&hdr));
                    DispatchOutcome::Respond(UsartResponseHeader::SIZE + n)
                }
                Err(BackendError::WouldBlock) => {
                    // No data yet.  Attempt to park the request; fail with
                    // Timeout if another TryRead is already pending — UART RX
                    // is a single stream and cannot serve two concurrent
                    // consumers.
                    if !pending.park(client_channel, req_len) {
                        return DispatchOutcome::Respond(encode_error(
                            response,
                            UsartError::Timeout,
                        ));
                    }
                    let _ = backend.enable_interrupts(IrqMask::RX_DATA_AVAILABLE);
                    DispatchOutcome::Queued
                }
                Err(e) => DispatchOutcome::Respond(encode_error(response, e.into())),
            }
        }
        UsartOp::GetLineStatus => match backend.line_status() {
            Ok(lsr) => DispatchOutcome::Respond(encode_success(response, &[lsr.0])),
            Err(e) => DispatchOutcome::Respond(encode_error(response, e.into())),
        },
        UsartOp::EnableInterrupts => {
            let mask = IrqMask::from_bits_truncate(hdr.arg0_value());
            match backend.enable_interrupts(mask) {
                Ok(()) => DispatchOutcome::Respond(encode_success(response, &[])),
                Err(e) => DispatchOutcome::Respond(encode_error(response, e.into())),
            }
        }
        UsartOp::DisableInterrupts => {
            let mask = IrqMask::from_bits_truncate(hdr.arg0_value());
            match backend.disable_interrupts(mask) {
                Ok(()) => DispatchOutcome::Respond(encode_success(response, &[])),
                Err(e) => DispatchOutcome::Respond(encode_error(response, e.into())),
            }
        }
    }
}

fn encode_error(response: &mut [u8], error: UsartError) -> usize {
    let hdr = UsartResponseHeader::error(error);
    response[..UsartResponseHeader::SIZE].copy_from_slice(zerocopy::IntoBytes::as_bytes(&hdr));
    UsartResponseHeader::SIZE
}

fn encode_success(response: &mut [u8], payload: &[u8]) -> usize {
    let hdr = UsartResponseHeader::success(payload.len() as u16);
    response[..UsartResponseHeader::SIZE].copy_from_slice(zerocopy::IntoBytes::as_bytes(&hdr));
    response[UsartResponseHeader::SIZE..UsartResponseHeader::SIZE + payload.len()]
        .copy_from_slice(payload);
    UsartResponseHeader::SIZE + payload.len()
}
