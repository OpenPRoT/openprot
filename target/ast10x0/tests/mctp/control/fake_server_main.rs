// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Bus owner stand-in that drives the control responder under QEMU.
//!
//! QEMU offers no I2C peer, so this app takes the place of both the MCTP
//! server and the bus owner behind it. It answers the responder's IPC calls,
//! hands it one control request per receive, and checks the IPC header and
//! every byte of each reply. It calls `debug_shutdown(Ok(()))` once every
//! case passes and `debug_shutdown(Err(_))` on the first mismatch, which the
//! kernel target turns into the UART sentinel `qemu_runner.py` reads.
//!
//! Each expected reply is written out as literal bytes rather than built by
//! an encoder, so an encoding bug fails a case instead of appearing on both
//! sides of the comparison. See CASES for what each one covers. Clause,
//! table, and figure numbers refer to DSP0236 1.3.1.

#![no_main]
#![no_std]

use app_fake_server::handle;
use mctp_lib::config;
use openprot_mctp_api::wire::{self, MctpOp, MctpRequestHeader, MctpResponseHeader};
use pw_status::Error;
use userspace::entry;
use userspace::syscall::{self, Signals};
use userspace::time::Instant;

/// The EID this stand-in sends from, which replies come back to.
const BUS_OWNER_EID: u8 = 0x08;

/// The tag each request carries, which the reply repeats.
///
/// A reply sent without its tag reaches this app as tag 0, so this tag is a
/// non-zero value in the 3-bit range of an MCTP message tag.
const TAG: u8 = 0x05;

const LISTENER_HANDLE: u32 = 0x21;

/// The EID the responder holds before a bus owner assigns one.
///
/// DSP0236 Table 15 gives byte 2 of the Get Endpoint ID response as
/// "0x00 = EID not yet assigned", which is what an unassigned dynamic endpoint
/// reports.
const UNASSIGNED_EID: u8 = 0x00;

const MAX_REQUEST_SIZE: usize = MctpRequestHeader::SIZE + config::MAX_PAYLOAD;
const MAX_RESPONSE_SIZE: usize = MctpResponseHeader::SIZE + config::MAX_PAYLOAD;

/// One request and the reply it must draw.
struct Case {
    /// The control message bytes a bus owner puts on the bus.
    request: &'static [u8],
    /// The reply bytes, or `None` for a message that draws no reply.
    expected: Option<&'static [u8]>,
}

/// Case 3's reply: header, Success, an entry count of 4, then the versions.
///
/// DSP0236 clause 12.6.3 fixes the control protocol's own entries, which for
/// 1.3.1 match these, so case 4's reply differs in the instance ID alone.
const BASE_VERSION_REPLY: [u8; 20] = [
    0x03, 0x04, 0x00, 0x04, //
    0xf1, 0xf0, 0xff, 0x00, //
    0xf1, 0xf1, 0xff, 0x00, //
    0xf1, 0xf2, 0xff, 0x00, //
    0xf1, 0xf3, 0xf1, 0x00,
];

/// Case 4's reply. See BASE_VERSION_REPLY.
const CONTROL_VERSION_REPLY: [u8; 20] = [
    0x04, 0x04, 0x00, 0x04, //
    0xf1, 0xf0, 0xff, 0x00, //
    0xf1, 0xf1, 0xff, 0x00, //
    0xf1, 0xf2, 0xff, 0x00, //
    0xf1, 0xf3, 0xf1, 0x00,
];

/// The script this stand-in runs, one case per receive.
///
/// A request byte 0 carries the request bit, the datagram bit, a reserved
/// bit, then a 5-bit instance ID, which the reply repeats with the request
/// and datagram bits cleared. Each case drawing a reply uses its own case
/// number as that instance ID, so a reply carrying the wrong one fails here
/// rather than passing unnoticed.
static CASES: [Case; 11] = [
    // Get Endpoint ID before any assignment: Success, the null EID, a simple
    // endpoint with EID type 00b for a dynamic EID, then medium-specific
    // information 0x00.
    Case {
        request: &[0x80, 0x02],
        expected: Some(&[0x00, 0x02, 0x00, UNASSIGNED_EID, 0x00, 0x00]),
    },
    // Set Endpoint ID, operation 00b, assigning 0x1c: accepted, no EID pool,
    // reporting the EID taken, then a pool size of 0.
    Case {
        request: &[0x81, 0x01, 0x00, 0x1c],
        expected: Some(&[0x01, 0x01, 0x00, 0x00, 0x1c, 0x00]),
    },
    // Get Endpoint ID again. This case is why the image exists: the reply
    // reports 0x1c only if case 1's Set Endpoint ID reached the stack over
    // IPC, which this app plays and no host test observes.
    Case {
        request: &[0x82, 0x02],
        expected: Some(&[0x02, 0x02, 0x00, 0x1c, 0x00, 0x00]),
    },
    // Message type 0xff asks for the MCTP base specification version.
    Case {
        request: &[0x83, 0x04, 0xff],
        expected: Some(&BASE_VERSION_REPLY),
    },
    // Message type 0x00 asks for the control protocol's own version.
    Case {
        request: &[0x84, 0x04, 0x00],
        expected: Some(&CONTROL_VERSION_REPLY),
    },
    // Get Message Type Support: a count of 2, then PLDM and SPDM. Table 19
    // counts the types in addition to control itself, so 0x00 is absent.
    Case {
        request: &[0x85, 0x05],
        expected: Some(&[0x05, 0x05, 0x00, 0x02, 0x01, 0x05]),
    },
    // Get Endpoint UUID is not implemented, so it answers
    // ERROR_UNSUPPORTED_CMD under its own command code rather than silence.
    Case {
        request: &[0x86, 0x03],
        expected: Some(&[0x06, 0x03, 0x05]),
    },
    // Set Endpoint ID assigning the null EID, which Table 14 marks illegal:
    // ERROR_INVALID_DATA.
    Case {
        request: &[0x87, 0x01, 0x00, 0x00],
        expected: Some(&[0x07, 0x01, 0x02]),
    },
    // Reset EID, operation 10b, with byte 2 left out: Table 14 has the endpoint
    // ignore that byte, and the responder accepts a body without it. A dynamic
    // endpoint holds no configured value to restore, so it answers
    // ERROR_INVALID_DATA.
    Case {
        request: &[0x88, 0x01, 0x02],
        expected: Some(&[0x08, 0x01, 0x02]),
    },
    // DSP0236 Table 11: a response is not a request. Answering it would put a
    // message on the bus that no requester is waiting for.
    Case {
        request: &[0x00, 0x02],
        expected: None,
    },
    // DSP0236 Table 11: a datagram is an unacknowledged request, whose
    // responses are "neither expected nor tracked by Instance ID at the
    // requester".
    Case {
        request: &[0xc0, 0x02],
        expected: None,
    },
];

/// What this stand-in is waiting for next.
struct Script {
    step: usize,
    /// Set while a case has been handed over and its reply not yet checked.
    pending: bool,
    /// The EID this app reports for the responder's stack, which SetEid moves.
    eid: u8,
}

#[entry]
fn entry() {
    let mut request_buf = [0u8; MAX_REQUEST_SIZE];
    let mut response_buf = [0u8; MAX_RESPONSE_SIZE];
    let mut script = Script {
        step: 0,
        pending: false,
        eid: UNASSIGNED_EID,
    };

    if syscall::wait_group_add(handle::WG, handle::MCTP, Signals::READABLE, 0usize).is_err() {
        pw_log::error!("fake server cannot watch its channel");
        fail();
    }

    pw_log::info!("control responder test starting");

    loop {
        let _ = syscall::object_wait(handle::WG, Signals::READABLE, Instant::MAX);

        let len = match syscall::channel_read(handle::MCTP, 0, &mut request_buf) {
            Ok(n) => n,
            Err(_) => continue,
        };

        let resp_len = serve(&mut script, &request_buf[..len], &mut response_buf);
        let _ = syscall::channel_respond(handle::MCTP, &response_buf[..resp_len]);
    }
}

/// Answer one IPC request from the responder, advancing the script.
fn serve(script: &mut Script, request: &[u8], response: &mut [u8]) -> usize {
    let Ok(header) = wire::decode_request_header(request) else {
        pw_log::error!("fake server got an undecodable request");
        fail();
    };

    let Some(op) = MctpOp::from_u8(header.op) else {
        pw_log::error!("responder made an unknown call: op {}", header.op as u32);
        fail();
    };

    match op {
        // The responder listens for the control message type alone.
        MctpOp::Listener => {
            if header.msg_type != 0x00 {
                pw_log::error!("responder listened on type {}", header.msg_type as u32);
                fail();
            }
            wire::encode_handle_response(response, LISTENER_HANDLE).unwrap_or(0)
        }

        // An accepted Set Endpoint ID updates the EID this app holds, which
        // case 2's Get Endpoint ID then reads back.
        MctpOp::SetEid => {
            script.eid = header.eid;
            wire::encode_success_response(response).unwrap_or(0)
        }

        MctpOp::GetEid => wire::encode_get_eid_response(response, script.eid).unwrap_or(0),

        MctpOp::Recv => serve_recv(script, response),

        MctpOp::Send => serve_send(script, &header, request, response),

        MctpOp::Req | MctpOp::Unbind => {
            pw_log::error!("responder made an unexpected call: op {}", header.op as u32);
            fail();
        }
    }
}

/// Hand the responder its next control request.
fn serve_recv(script: &mut Script, response: &mut [u8]) -> usize {
    // A receive arriving with a case still pending means that case drew no
    // reply, which is the right outcome for a response and for a datagram.
    if script.pending {
        if CASES[script.step].expected.is_some() {
            pw_log::error!("case {} drew no reply", script.step as u32);
            fail();
        }
        script.step += 1;
        script.pending = false;
    }

    if script.step >= CASES.len() {
        pw_log::info!("every control case passed");
        pass();
    }

    script.pending = true;
    wire::encode_recv_response(
        response,
        0x00,
        /* msg_ic= */ false,
        BUS_OWNER_EID,
        TAG,
        CASES[script.step].request,
    )
    .unwrap_or(0)
}

/// Check the reply the responder sends against the case that drew it.
fn serve_send(
    script: &mut Script,
    header: &wire::MctpRequestHeader,
    request: &[u8],
    response: &mut [u8],
) -> usize {
    if !script.pending {
        pw_log::error!("responder sent a reply no request asked for");
        fail();
    }

    let Some(expected) = CASES[script.step].expected else {
        pw_log::error!("case {} drew a reply it must not", script.step as u32);
        fail();
    };

    // A reply goes back as a control message to the requester's EID under the
    // request's tag. It carries no handle, which marks it as a reply, and
    // DSP0236 Table 10 has a control message carry IC = 0b.
    if header.msg_type != 0x00 {
        pw_log::error!(
            "case {} replied with type {}",
            script.step as u32,
            header.msg_type as u32
        );
        fail();
    }
    if header.flags & wire::flags::HAS_EID == 0 || header.eid != BUS_OWNER_EID {
        pw_log::error!(
            "case {} replied to EID {}, flags {}",
            script.step as u32,
            header.eid as u32,
            header.flags as u32
        );
        fail();
    }
    if header.flags & wire::flags::HAS_TAG == 0 || header.tag != TAG {
        pw_log::error!(
            "case {} replied with tag {}, flags {}",
            script.step as u32,
            header.tag as u32,
            header.flags as u32
        );
        fail();
    }
    if header.flags & wire::flags::HAS_HANDLE != 0 {
        pw_log::error!(
            "case {} replied with a handle, flags {}",
            script.step as u32,
            header.flags as u32
        );
        fail();
    }
    if header.flags & wire::flags::IC != 0 {
        pw_log::error!(
            "case {} replied with IC set, flags {}",
            script.step as u32,
            header.flags as u32
        );
        fail();
    }

    let reply = wire::get_request_payload(request);
    if reply != expected {
        // Name the first differing byte, which the lengths alone hide.
        pw_log::error!(
            "case {} replied {} bytes, expected {}",
            script.step as u32,
            reply.len() as u32,
            expected.len() as u32
        );
        for (i, want) in expected.iter().enumerate() {
            match reply.get(i) {
                Some(got) if got == want => continue,
                Some(got) => pw_log::error!(
                    "  byte {} is {}, expected {}",
                    i as u32,
                    *got as u32,
                    *want as u32
                ),
                None => pw_log::error!("  byte {} is missing", i as u32),
            }
            break;
        }
        fail();
    }

    script.step += 1;
    script.pending = false;
    wire::encode_send_response(response, TAG).unwrap_or(0)
}

/// Report every case passed, which the kernel turns into `TEST_RESULT:PASS`.
fn pass() -> ! {
    let _ = syscall::debug_shutdown(Ok(()));
    loop {}
}

/// Report a failed case, which the kernel turns into `TEST_RESULT:FAIL`.
fn fail() -> ! {
    let _ = syscall::debug_shutdown(Err(Error::Internal));
    loop {}
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    let _ = syscall::debug_shutdown(Err(Error::Internal));
    loop {}
}
