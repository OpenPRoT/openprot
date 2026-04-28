// Licensed under the Apache-2.0 license

#![no_std]

pub mod runtime;

use usart_api::backend::{IrqMask, UsartBackend};
use usart_api::{UsartError, UsartOp, UsartRequestHeader, UsartResponseHeader};

pub const MAX_REQUEST_SIZE: usize = 512;
pub const MAX_RESPONSE_SIZE: usize = 512;

pub fn dispatch_request<B: UsartBackend>(
    backend: &mut B,
    request: &[u8],
    response: &mut [u8],
) -> usize {
    if request.len() < UsartRequestHeader::SIZE {
        return encode_error(response, UsartError::InvalidOperation);
    }

    let hdr_bytes = &request[..UsartRequestHeader::SIZE];
    let Some(hdr) = zerocopy::Ref::<_, UsartRequestHeader>::from_bytes(hdr_bytes).ok() else {
        return encode_error(response, UsartError::InvalidOperation);
    };

    let op = match hdr.operation() {
        Ok(op) => op,
        Err(e) => return encode_error(response, e),
    };

    let payload_len = hdr.payload_length();
    if request.len() < UsartRequestHeader::SIZE + payload_len {
        return encode_error(response, UsartError::InvalidOperation);
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
                Ok(()) => encode_success(response, &[]),
                Err(e) => encode_error(response, e.into()),
            }
        }
        UsartOp::Write => match backend.write(payload) {
            Ok(_) => encode_success(response, &[]),
            Err(e) => encode_error(response, e.into()),
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
                    UsartResponseHeader::SIZE + n
                }
                Err(e) => encode_error(response, e.into()),
            }
        }
        UsartOp::GetLineStatus => match backend.line_status() {
            Ok(lsr) => encode_success(response, &[lsr.0]),
            Err(e) => encode_error(response, e.into()),
        },
        UsartOp::EnableInterrupts => {
            let mask = IrqMask::from_bits_truncate(hdr.arg0_value());
            match backend.enable_interrupts(mask) {
                Ok(()) => encode_success(response, &[]),
                Err(e) => encode_error(response, e.into()),
            }
        }
        UsartOp::DisableInterrupts => {
            let mask = IrqMask::from_bits_truncate(hdr.arg0_value());
            match backend.disable_interrupts(mask) {
                Ok(()) => encode_success(response, &[]),
                Err(e) => encode_error(response, e.into()),
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
