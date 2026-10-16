// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Harness for the MCTP multi-protocol QEMU test.
//!
//! Plays the remote peer for both echo tasks. It issues requests through
//! its own channel (`mctp_harness`) on `Stack` request channels; the
//! server's loopback transport turns each one into an inbound request that
//! the server must route to the SPDM or PLDM listener by message type.
//! Each echo task answers on the request's tag, so a reply arriving on the
//! right request channel with the right payload proves demultiplexing and
//! the response path (unowned tag back to the request) through one server.
//! Which task answered is asserted inside the tasks themselves (see
//! echo_task.rs).
//!
//! ## Test cases
//!
//! | TC   | Exercises                                                              |
//! |------|-------------------------------------------------------------------------|
//! | TC-1 | SPDM then PLDM, one at a time: demux by type + reply correlation       |
//! | TC-2 | both requests in flight at once, replies collected in reverse order:   |
//! |      | the two parked recvs are independent and neither blocks the other      |
//! | TC-3 | a type nobody listens for is dropped: harness times out, no task fires |
//! | TC-4 | TC-1 again after TC-2/3: both tasks re-parked and keep serving         |
//!
//! Startup needs no fixed delay: each echo task answers one transaction on
//! its `ctl` channel only after its listener is registered, and the harness
//! blocks on those two transactions before its first request.

#![no_main]
#![no_std]

use echo_task::{MSG_TYPE_PLDM, MSG_TYPE_SPDM};
use mctp_lib::config;
use openprot_mctp_api::{MctpReqChannel, Stack};
use openprot_mctp_client_ipc::IpcMctpClient;
use pw_status::Error;
use userspace::time::Instant;
use userspace::{entry, syscall};

use app_harness::handle;

const OWN_EID: u8 = 8;
/// A type with no listener in this image (vendor-defined PCI).
const MSG_TYPE_UNSERVED: u8 = 0x7E;
/// Generous: a loopback round trip is a few IPC hops.
const REPLY_TIMEOUT_MS: u32 = 2000;
/// Short: TC-3 expects this to expire.
const DROP_TIMEOUT_MS: u32 = 300;

#[entry]
fn entry() {
    match run() {
        Ok(()) => {
            pw_log::info!("All test cases PASSED");
            let _ = syscall::debug_shutdown(Ok(()));
        }
        Err(()) => {
            let _ = syscall::debug_shutdown(Err(Error::Internal));
        }
    }
    loop {}
}

fn run() -> Result<(), ()> {
    // Barrier: both echo tasks must have their listeners up before the
    // first request, or the server drops it as unroutable.
    for ctl in [handle::CTL_SPDM, handle::CTL_PLDM] {
        let mut scratch = [0u8; 1];
        if syscall::channel_transact(ctl, &[0u8; 0], &mut scratch, Instant::MAX).is_err() {
            pw_log::error!("harness: ctl barrier failed");
            return Err(());
        }
    }
    pw_log::info!("harness: both listeners registered");

    let stack = Stack::new(IpcMctpClient::new(handle::MCTP));
    let mut spdm = stack.req(OWN_EID, REPLY_TIMEOUT_MS).map_err(|e| {
        pw_log::error!("harness: req(spdm) code={}", e.code as u32);
    })?;
    let mut pldm = stack.req(OWN_EID, REPLY_TIMEOUT_MS).map_err(|e| {
        pw_log::error!("harness: req(pldm) code={}", e.code as u32);
    })?;

    // ── TC-1: one protocol at a time ────────────────────────────────────────
    pw_log::info!("TC-1: SPDM then PLDM, sequential");
    send(&mut spdm, MSG_TYPE_SPDM, b"S1")?;
    expect_echo(&mut spdm, MSG_TYPE_SPDM, b"S1")?;
    send(&mut pldm, MSG_TYPE_PLDM, b"P1")?;
    expect_echo(&mut pldm, MSG_TYPE_PLDM, b"P1")?;

    // ── TC-2: both in flight, replies read in the opposite order ────────────
    pw_log::info!("TC-2: PLDM and SPDM in flight together");
    send(&mut pldm, MSG_TYPE_PLDM, b"P2")?;
    send(&mut spdm, MSG_TYPE_SPDM, b"S2")?;
    expect_echo(&mut spdm, MSG_TYPE_SPDM, b"S2")?;
    expect_echo(&mut pldm, MSG_TYPE_PLDM, b"P2")?;

    // ── TC-3: a type nobody serves ──────────────────────────────────────────
    pw_log::info!("TC-3: unserved message type is dropped");
    {
        let mut unserved = stack.req(OWN_EID, DROP_TIMEOUT_MS).map_err(|e| {
            pw_log::error!("harness: req(unserved) code={}", e.code as u32);
        })?;
        send(&mut unserved, MSG_TYPE_UNSERVED, b"X3")?;
        let mut buf = [0u8; config::MAX_PAYLOAD];
        match unserved.recv(&mut buf) {
            Err(e) if e.is_timeout() => {}
            Err(e) => {
                pw_log::error!("TC-3 FAIL: expected TimedOut, got {}", e.code as u32);
                return Err(());
            }
            Ok(_) => {
                pw_log::error!("TC-3 FAIL: unserved type was delivered");
                return Err(());
            }
        }
    }

    // ── TC-4: both tasks still serving after the above ──────────────────────
    pw_log::info!("TC-4: second round");
    send(&mut spdm, MSG_TYPE_SPDM, b"S4")?;
    expect_echo(&mut spdm, MSG_TYPE_SPDM, b"S4")?;
    send(&mut pldm, MSG_TYPE_PLDM, b"P4")?;
    expect_echo(&mut pldm, MSG_TYPE_PLDM, b"P4")?;

    Ok(())
}

fn send<R: MctpReqChannel>(req: &mut R, msg_type: u8, payload: &[u8]) -> Result<(), ()> {
    req.send(msg_type, payload).map_err(|e| {
        pw_log::error!("FAIL: send type {} code={}", msg_type as u32, e.code as u32);
    })
}

/// Block for the echo of `payload` on `req` and check its header and body.
fn expect_echo<R: MctpReqChannel>(req: &mut R, msg_type: u8, payload: &[u8]) -> Result<(), ()> {
    let mut buf = [0u8; config::MAX_PAYLOAD];
    let (meta, echoed) = req.recv(&mut buf).map_err(|e| {
        pw_log::error!("FAIL: recv type {} code={}", msg_type as u32, e.code as u32);
    })?;
    if meta.msg_type != msg_type || meta.remote_eid != OWN_EID {
        pw_log::error!(
            "FAIL: reply header type {} eid {}",
            meta.msg_type as u32,
            meta.remote_eid as u32
        );
        return Err(());
    }
    if echoed != payload {
        pw_log::error!("FAIL: payload mismatch for type {}", msg_type as u32);
        return Err(());
    }
    Ok(())
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
