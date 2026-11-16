// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! MCTP echo app: a client of the MCTP server that echoes every request it
//! receives, sends requests of its own when asked, and signals PASS after the one
//! that ends the run.
//!
//! Uses the reusable `//services/mctp/echo` policy over `IpcMctpClient`, exactly
//! like the ast10x0 echo app. The host decides how many requests to send (two for
//! the quick test, many for the soak) and ends the run with a request whose
//! payload is `DONE`. The firmware echoes that one too, and exits 0 once its
//! reply has been handed to the transport, so the run has a clean exit for the
//! host to wait on.
//!
//! A request whose payload is `INITIATE` is echoed, and then makes the app a
//! requester: it sends `TARGET_REQUEST_SIZES` requests to the host (`HOST_EID`),
//! one at a time, and checks that each response is its own payload echoed back.
//! This is the direction the echo itself never uses: the target owns the tag, and
//! the host answers.

#![no_main]
#![no_std]

use mctp_echo_codegen::handle;
use openprot_mctp_api::{MctpListener, MctpReqChannel, MctpRespChannel, Stack};
use openprot_mctp_client_ipc::IpcMctpClient;
use openprot_mctp_echo::{prepare_listener, ECHO_MSG_TYPE};
use userspace::process_entry;
use userspace::syscall;

/// The payload of the request that ends the run (see `host_test.rs`).
const DONE: &[u8] = b"mctp-i3c-done";

/// The payload of the request that makes the app send requests of its own.
const INITIATE: &[u8] = b"mctp-i3c-initiate";

/// The host's EID (`HOST_EID` in `host_test.rs`), where the app sends its requests.
const HOST_EID: u8 = 9;

/// Payload sizes of the requests the app sends, in order (`host_test.rs` expects
/// the same). They are the sizes of caliptra-mcu-sw's own loopback test: the
/// baseline, one under it, and two that need several packets.
const TARGET_REQUEST_SIZES: [usize; 4] = [64, 63, 256, 1000];

/// How long to wait for the host's response to one of those requests.
const TARGET_REQUEST_TIMEOUT_MS: u32 = 10_000;

/// Room for the largest message the host tests send, which is also the largest
/// the stack accepts (`MCTP_ESTACK_MAX_MESSAGE`, 1032 bytes without the type).
const ECHO_BUF: usize = 1032;

/// The payload of the app's request of `len` bytes: a pattern the host builds the
/// same way (`target_payload` in `host_test.rs`).
fn fill_target_payload(buf: &mut [u8]) {
    for (i, byte) in buf.iter_mut().enumerate() {
        *byte = (i % 253) as u8;
    }
}

/// Send the requests of `TARGET_REQUEST_SIZES` to the host, one at a time, and
/// check each response. Returns the number that did not match or failed.
fn send_target_requests(stack: &Stack<IpcMctpClient>) -> u32 {
    let mut failures = 0u32;
    let mut sent = [0u8; ECHO_BUF];
    let mut got = [0u8; ECHO_BUF];
    for len in TARGET_REQUEST_SIZES {
        let Some(payload) = sent.get_mut(..len) else {
            failures += 1;
            continue;
        };
        fill_target_payload(payload);
        let payload = &*payload;
        let ok = match stack.req(HOST_EID, TARGET_REQUEST_TIMEOUT_MS) {
            Ok(mut req) => match req.send(ECHO_MSG_TYPE, payload) {
                Ok(()) => match req.recv(&mut got) {
                    Ok((_meta, msg)) => msg == payload,
                    Err(e) => {
                        pw_log::error!("target request recv failed: code={}", e.code as u32);
                        false
                    }
                },
                Err(e) => {
                    pw_log::error!("target request send failed: code={}", e.code as u32);
                    false
                }
            },
            Err(e) => {
                pw_log::error!("target request open failed: code={}", e.code as u32);
                false
            }
        };
        if ok {
            pw_log::info!("mctp echo: target request of {} bytes ok", len as u32);
        } else {
            pw_log::error!("mctp echo: target request of {} bytes failed", len as u32);
            failures += 1;
        }
    }
    failures
}

#[process_entry("mctp_echo")]
fn entry() {
    let stack = Stack::new(IpcMctpClient::new(handle::MCTP));
    let mut listener = match prepare_listener(&stack) {
        Ok(listener) => listener,
        Err(e) => {
            pw_log::error!("echo setup failed: code={}", e.code as u32);
            let _ = syscall::debug_shutdown(Err(pw_status::Error::Internal));
            loop {}
        }
    };

    // The listener is armed; the host may now send. `recv` blocks until a
    // request arrives, and the payload goes straight back.
    pw_log::info!("mctp echo: waiting for request");

    let mut buf = [0u8; ECHO_BUF];
    let mut served: u32 = 0;
    loop {
        let (done, initiate) = match listener.recv(&mut buf) {
            Ok((_meta, msg, mut resp)) => {
                let flags = (msg == DONE, msg == INITIATE);
                if let Err(e) = resp.send(msg) {
                    pw_log::error!("echo failed: code={}", e.code as u32);
                    let _ = syscall::debug_shutdown(Err(pw_status::Error::Internal));
                    loop {}
                }
                flags
            }
            Err(e) => {
                pw_log::error!("echo failed: code={}", e.code as u32);
                let _ = syscall::debug_shutdown(Err(pw_status::Error::Internal));
                loop {}
            }
        };
        served += 1;
        pw_log::info!("mctp echo: serviced request {}", served);
        if initiate && send_target_requests(&stack) != 0 {
            let _ = syscall::debug_shutdown(Err(pw_status::Error::Internal));
            loop {}
        }
        if done {
            break;
        }
    }
    let _ = syscall::debug_shutdown(Ok(()));
    #[expect(clippy::empty_loop)]
    loop {}
}
