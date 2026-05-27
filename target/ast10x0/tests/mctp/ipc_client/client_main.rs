// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! MCTP IPC-client exerciser for the QEMU test.
//!
//! Runs ten test cases against `fake_server` via `IpcMctpClient` and calls
//! `debug_shutdown(Ok(()))` on full pass or `debug_shutdown(Err(_))` on the
//! first failure.  The kernel target writes `TEST_RESULT:PASS/FAIL` to UART.
//!
//! ## Test cases
//!
//! | TC   | Method          | Expected result                          |
//! |------|-----------------|------------------------------------------|
//! | TC-01 | set_eid(8)     | Ok(())                                   |
//! | TC-02 | get_eid()      | returns 8                                |
//! | TC-03 | listener(5)    | Ok(Handle(42))                           |
//! | TC-04 | req(8)         | Ok(Handle(43))                           |
//! | TC-05 | recv(h42, …)   | Ok(meta); msg_type=5, eid=8,             |
//! |       |                | payload=[DE AD BE EF]                    |
//! | TC-06 | send(…, b"hi") | Ok(1)  (tag)                             |
//! | TC-07 | drop_handle(h43)| no panic                                |
//! | TC-08 | listener(0xFE) | Err(BadArgument)                         |
//! | TC-09 | recv(h0, …)    | Err(TimedOut)                            |
//! | TC-10 | recv(h0xFEFE,…)| Err(InternalError)  (truncated response) |

#![no_main]
#![no_std]

use openprot_mctp_api::{Handle, MctpClient, ResponseCode};
use openprot_mctp_client_ipc::IpcMctpClient;
use pw_status::Error;
use userspace::{entry, syscall};

use app_client_test::handle;

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
    let client = IpcMctpClient::new(handle::MCTP);

    // ── TC-01: set_eid ────────────────────────────────────────────────────────
    pw_log::info!("TC-01: set_eid(8)");
    client.set_eid(8).map_err(|e| {
        pw_log::error!("TC-01 FAIL: code={}", e.code as u32);
    })?;

    // ── TC-02: get_eid ────────────────────────────────────────────────────────
    pw_log::info!("TC-02: get_eid()");
    let eid = client.get_eid();
    if eid != 8 {
        pw_log::error!("TC-02 FAIL: expected 8, got {}", eid as u32);
        return Err(());
    }

    // ── TC-03: listener ───────────────────────────────────────────────────────
    pw_log::info!("TC-03: listener(5)");
    let listener_handle = client.listener(5).map_err(|e| {
        pw_log::error!("TC-03 FAIL: code={}", e.code as u32);
    })?;
    if listener_handle.0 != 42 {
        pw_log::error!("TC-03 FAIL: expected handle 42, got {}", listener_handle.0 as u32);
        return Err(());
    }

    // ── TC-04: req ────────────────────────────────────────────────────────────
    pw_log::info!("TC-04: req(8)");
    let req_handle = client.req(8).map_err(|e| {
        pw_log::error!("TC-04 FAIL: code={}", e.code as u32);
    })?;
    if req_handle.0 != 43 {
        pw_log::error!("TC-04 FAIL: expected handle 43, got {}", req_handle.0 as u32);
        return Err(());
    }

    // ── TC-05: recv — success with payload ───────────────────────────────────
    pw_log::info!("TC-05: recv(listener_handle)");
    let mut recv_buf = [0u8; 16];
    let meta = client
        .recv(listener_handle, 0, &mut recv_buf)
        .map_err(|e| {
            pw_log::error!("TC-05 FAIL: code={}", e.code as u32);
        })?;
    if meta.msg_type != 5 {
        pw_log::error!("TC-05 FAIL: msg_type expected 5, got {}", meta.msg_type as u32);
        return Err(());
    }
    if meta.remote_eid != 8 {
        pw_log::error!("TC-05 FAIL: remote_eid expected 8, got {}", meta.remote_eid as u32);
        return Err(());
    }
    if meta.payload_size != 4 {
        pw_log::error!("TC-05 FAIL: payload_size expected 4, got {}", meta.payload_size as u32);
        return Err(());
    }
    if recv_buf[..4] != [0xDE, 0xAD, 0xBE, 0xEF] {
        pw_log::error!("TC-05 FAIL: payload mismatch");
        return Err(());
    }

    // ── TC-06: send ───────────────────────────────────────────────────────────
    pw_log::info!("TC-06: send()");
    let tag = client
        .send(None, 5, Some(8), None, false, b"hi")
        .map_err(|e| {
            pw_log::error!("TC-06 FAIL: code={}", e.code as u32);
        })?;
    if tag != 1 {
        pw_log::error!("TC-06 FAIL: expected tag 1, got {}", tag as u32);
        return Err(());
    }

    // ── TC-07: drop_handle ───────────────────────────────────────────────────
    pw_log::info!("TC-07: drop_handle(req_handle)");
    client.drop_handle(req_handle);

    // ── TC-08: error response — BadArgument ──────────────────────────────────
    pw_log::info!("TC-08: listener(0xFE) should return BadArgument");
    match client.listener(0xFE) {
        Err(e) if e.code == ResponseCode::BadArgument => {}
        Err(e) => {
            pw_log::error!("TC-08 FAIL: expected BadArgument, got {}", e.code as u32);
            return Err(());
        }
        Ok(_) => {
            pw_log::error!("TC-08 FAIL: expected Err, got Ok");
            return Err(());
        }
    }

    // ── TC-09: TimedOut response ─────────────────────────────────────────────
    pw_log::info!("TC-09: recv(Handle(0)) should return TimedOut");
    match client.recv(Handle(0), 1000, &mut recv_buf) {
        Err(e) if e.code == ResponseCode::TimedOut => {}
        Err(e) => {
            pw_log::error!("TC-09 FAIL: expected TimedOut, got {}", e.code as u32);
            return Err(());
        }
        Ok(_) => {
            pw_log::error!("TC-09 FAIL: expected Err, got Ok");
            return Err(());
        }
    }

    // ── TC-10: short (truncated) response — InternalError ────────────────────
    pw_log::info!("TC-10: recv(Handle(0xFEFE)) should return InternalError");
    match client.recv(Handle(0xFEFE), 1000, &mut recv_buf) {
        Err(e) if e.code == ResponseCode::InternalError => {}
        Err(e) => {
            pw_log::error!("TC-10 FAIL: expected InternalError, got {}", e.code as u32);
            return Err(());
        }
        Ok(_) => {
            pw_log::error!("TC-10 FAIL: expected Err, got Ok");
            return Err(());
        }
    }

    Ok(())
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
