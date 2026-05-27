// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Fake MCTP server for the IPC-client QEMU test.
//!
//! Speaks the `openprot_mctp_api::wire` protocol over a Pigweed IPC channel.
//! No I2C, no serial — just a response table keyed on op code and selected
//! request fields.
//!
//! ## Response table
//!
//! | Op            | Trigger                  | Response                                 |
//! |---------------|--------------------------|------------------------------------------|
//! | SetEid        | any                      | Success                                  |
//! | GetEid        | any                      | Success, eid = 8                         |
//! | Listener      | msg_type != 0xFE         | Success, handle = 42                     |
//! | Listener      | msg_type == 0xFE (TC-08) | Error: BadArgument                       |
//! | Req           | any                      | Success, handle = 43                     |
//! | Recv          | handle == 42             | Success, msg_type=5, eid=8, tag=0,       |
//! |               |                          | payload = [0xDE, 0xAD, 0xBE, 0xEF]      |
//! | Recv          | handle == 0    (TC-09)   | Error: TimedOut                          |
//! | Recv          | handle == 0xFEFE (TC-10) | 4-byte short response (no full header)   |
//! | Send          | any                      | Success, tag = 1                         |
//! | Unbind        | any                      | Success                                  |

#![no_main]
#![no_std]

use openprot_mctp_api::wire::{
    self, MctpOp, MctpRequestHeader, MAX_REQUEST_SIZE, MAX_RESPONSE_SIZE,
};
use openprot_mctp_api::ResponseCode;
use userspace::{entry, syscall};
use userspace::syscall::Signals;
use userspace::time::Instant;

use app_fake_server::handle;

// Handle value used by the client to trigger a short (truncated) response.
const TC10_HANDLE: u32 = 0xFEFE;
// Handle value used by the client to trigger a TimedOut error response.
const TC09_HANDLE: u32 = 0;

#[entry]
fn entry() {
    let mut req_buf = [0u8; MAX_REQUEST_SIZE];
    let mut resp_buf = [0u8; MAX_RESPONSE_SIZE];

    if syscall::wait_group_add(handle::WG, handle::MCTP, Signals::READABLE, 0usize).is_err() {
        loop {}
    }

    loop {
        let _ = syscall::object_wait(handle::WG, Signals::READABLE, Instant::MAX);

        let len = match syscall::channel_read(handle::MCTP, 0, &mut req_buf) {
            Ok(n) => n,
            Err(_) => continue,
        };

        // Parse the request header once; handle TC-10 (short response) before
        // the generic dispatch so we can bypass the full response encoder.
        let header = match MctpRequestHeader::from_bytes(&req_buf[..len]) {
            Some(h) => h,
            None => {
                let rlen =
                    wire::encode_error_response(&mut resp_buf, ResponseCode::BadArgument)
                        .unwrap_or(0);
                let _ = syscall::channel_respond(handle::MCTP, &resp_buf[..rlen]);
                continue;
            }
        };

        // TC-10: respond with 4 bytes only so the client sees a truncated header.
        if matches!(header.operation(), Some(MctpOp::Recv)) && header.handle == TC10_HANDLE {
            let _ = syscall::channel_respond(handle::MCTP, &[0u8; 4]);
            continue;
        }

        let resp_len = dispatch(&header, &mut resp_buf);
        let _ = syscall::channel_respond(handle::MCTP, &resp_buf[..resp_len]);
    }
}

/// Build a response for the given decoded header.
fn dispatch(header: &MctpRequestHeader, resp: &mut [u8]) -> usize {
    let op = match header.operation() {
        Some(op) => op,
        None => {
            return wire::encode_error_response(resp, ResponseCode::BadArgument).unwrap_or(0)
        }
    };

    match op {
        MctpOp::SetEid => wire::encode_success_response(resp).unwrap_or(0),

        MctpOp::GetEid => wire::encode_get_eid_response(resp, 8).unwrap_or(0),

        MctpOp::Listener => {
            // TC-08: msg_type 0xFE triggers a BadArgument error response.
            if header.msg_type == 0xFE {
                wire::encode_error_response(resp, ResponseCode::BadArgument).unwrap_or(0)
            } else {
                wire::encode_handle_response(resp, 42).unwrap_or(0)
            }
        }

        MctpOp::Req => wire::encode_handle_response(resp, 43).unwrap_or(0),

        MctpOp::Recv => {
            // TC-09: handle 0 triggers a TimedOut error response.
            if header.handle == TC09_HANDLE {
                wire::encode_error_response(resp, ResponseCode::TimedOut).unwrap_or(0)
            } else {
                let payload: [u8; 4] = [0xDE, 0xAD, 0xBE, 0xEF];
                wire::encode_recv_response(resp, 5, false, 8, 0, &payload).unwrap_or(0)
            }
        }

        MctpOp::Send => wire::encode_send_response(resp, 1).unwrap_or(0),

        MctpOp::Unbind => wire::encode_success_response(resp).unwrap_or(0),
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
