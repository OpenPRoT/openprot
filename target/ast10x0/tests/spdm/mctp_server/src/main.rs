// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! MCTP Server stub for SPDM Responder testing.
//!
//! STUB IMPLEMENTATION — This is a minimal MCTP server that speaks the
//! `openprot_mctp_api::wire` protocol over a Pigweed IPC channel.
//! It provides deterministic responses for testing SPDM over MCTP.
//!
//! TODO: Replace with real MCTP server when hardware transport is integrated.
//!
//! ## Supported Operations
//!
//! | Op        | Response                                           |
//! |-----------|----------------------------------------------------|
//! | SetEid    | Success                                            |
//! | GetEid    | Success, eid = 8                                   |
//! | Listener  | Success, handle = 1                                |
//! | Req       | Success, handle = 2                                |
//! | Recv      | Success, returns pending SPDM request (if any)     |
//! | Send      | Success, buffers response for next recv            |
//! | Unbind    | Success                                            |

#![no_main]
#![no_std]

use app_mctp_server::handle;
use openprot_mctp_api::wire::{
    self, MctpOp, MctpRequestHeader, MAX_REQUEST_SIZE, MAX_RESPONSE_SIZE,
};
use openprot_mctp_api::ResponseCode;
use userspace::syscall::Signals;
use userspace::time::Instant;
use userspace::{entry, syscall};

const LISTENER_HANDLE: u32 = 1;
const REQ_HANDLE: u32 = 2;
const LOCAL_EID: u8 = 8;

#[entry]
fn entry() {
    pw_log::info!("MCTP server stub starting");

    let mut req_buf = [0u8; MAX_REQUEST_SIZE];
    let mut resp_buf = [0u8; MAX_RESPONSE_SIZE];

    if syscall::wait_group_add(handle::WG, handle::MCTP, Signals::READABLE, 0usize).is_err() {
        pw_log::error!("Failed to add MCTP channel to wait group");
        loop {}
    }

    loop {
        let _ = syscall::object_wait(handle::WG, Signals::READABLE, Instant::MAX);

        let len = match syscall::channel_read(handle::MCTP, 0, &mut req_buf) {
            Ok(n) => n,
            Err(_) => continue,
        };

        let header = match MctpRequestHeader::from_bytes(&req_buf[..len]) {
            Some(h) => h,
            None => {
                let rlen = wire::encode_error_response(&mut resp_buf, ResponseCode::BadArgument)
                    .unwrap_or(0);
                let _ = syscall::channel_respond(handle::MCTP, &resp_buf[..rlen]);
                continue;
            }
        };

        let resp_len = dispatch(&header, &req_buf[..len], &mut resp_buf);
        let _ = syscall::channel_respond(handle::MCTP, &resp_buf[..resp_len]);
    }
}

fn dispatch(header: &MctpRequestHeader, _req: &[u8], resp: &mut [u8]) -> usize {
    let op = match header.operation() {
        Some(op) => op,
        None => {
            return wire::encode_error_response(resp, ResponseCode::BadArgument).unwrap_or(0);
        }
    };

    match op {
        MctpOp::SetEid => {
            pw_log::debug!("SetEid: {}", header.eid as u32);
            wire::encode_success_response(resp).unwrap_or(0)
        }

        MctpOp::GetEid => {
            pw_log::debug!("GetEid -> {}", LOCAL_EID as u32);
            wire::encode_get_eid_response(resp, LOCAL_EID).unwrap_or(0)
        }

        MctpOp::Listener => {
            pw_log::debug!("Listener: msg_type={}", header.msg_type as u32);
            wire::encode_handle_response(resp, LISTENER_HANDLE).unwrap_or(0)
        }

        MctpOp::Req => {
            pw_log::debug!("Req: eid={}", header.eid as u32);
            wire::encode_handle_response(resp, REQ_HANDLE).unwrap_or(0)
        }

        MctpOp::Recv => {
            pw_log::debug!("Recv: handle={}", header.handle as u32);
            // Return a minimal SPDM GET_VERSION request for testing
            // SPDM message type = 0x05, SPDMVersion=0x10, RequestResponseCode=0x84 (GET_VERSION)
            let spdm_get_version: [u8; 4] = [0x10, 0x84, 0x00, 0x00];
            wire::encode_recv_response(resp, 0x05, false, LOCAL_EID, 0, &spdm_get_version)
                .unwrap_or(0)
        }

        MctpOp::Send => {
            pw_log::debug!("Send: msg_type={}", header.msg_type as u32);
            wire::encode_send_response(resp, 0).unwrap_or(0)
        }

        MctpOp::Unbind => {
            pw_log::debug!("Unbind: handle={}", header.handle as u32);
            wire::encode_success_response(resp).unwrap_or(0)
        }
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    pw_log::error!("MCTP server panic");
    loop {}
}
