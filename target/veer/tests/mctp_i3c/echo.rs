// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! MCTP echo app: a client of the MCTP server that services a fixed number of
//! echo exchanges, then signals PASS.
//!
//! Uses the reusable `//services/mctp/echo` policy over `IpcMctpClient`, exactly
//! like the ast10x0 echo app. A fixed count rather than an infinite loop so the
//! run has a clean exit for the host to wait on: the host sends `ECHO_REQUESTS`
//! MCTP requests over I3C (a short one, then one larger than the MTU), this
//! echoes each back, and the firmware exits 0 once the last reply has been
//! handed to the transport.

#![no_main]
#![no_std]

use mctp_echo_codegen::handle;
use openprot_mctp_api::Stack;
use openprot_mctp_client_ipc::IpcMctpClient;
use openprot_mctp_echo::{echo_once, prepare_listener};
use userspace::process_entry;
use userspace::syscall;

/// How many requests to echo before exiting; the host test sends this many.
const ECHO_REQUESTS: u8 = 2;

/// Room for the largest message the host test sends (more than one packet).
const ECHO_BUF: usize = 768;

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

    // The listener is armed; the host may now send. `echo_once` blocks in recv
    // until the request arrives, then sends the payload straight back.
    pw_log::info!("mctp echo: waiting for request");

    let mut buf = [0u8; ECHO_BUF];
    for n in 1..=ECHO_REQUESTS {
        if let Err(e) = echo_once(&mut listener, &mut buf) {
            pw_log::error!("echo failed: code={}", e.code as u32);
            let _ = syscall::debug_shutdown(Err(pw_status::Error::Internal));
            loop {}
        }
        pw_log::info!("mctp echo: serviced request {}", n as u32);
    }
    let _ = syscall::debug_shutdown(Ok(()));
    #[expect(clippy::empty_loop)]
    loop {}
}
