// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Shared body of the two echo tasks that stand in for application
//! protocols, plus the constants the harness uses to address them.
//!
//! Each task is an ordinary MCTP application: it builds a `Stack` over its
//! own IPC channel to the shared server, opens one listener for its message
//! type, and echoes every request back on the request's tag, which is what
//! `openprot_mctp_echo` does. The only addition is a check that the
//! message the server handed over really is of the listener's type. An
//! echo reply copies the request's type, so without that check a request
//! routed to the wrong task would be echoed back indistinguishably; with
//! it, a misroute fails the test from inside the task that saw it.
//!
//! Startup barrier: after its listener is registered, a task answers one
//! transaction on its `ctl` channel. The harness blocks on that transaction
//! before sending anything, so a request can never arrive before the
//! listener exists (the server drops unroutable requests silently).

#![no_std]

use mctp_lib::config;
use openprot_mctp_api::{MctpListener, MctpRespChannel, Stack};
use openprot_mctp_client_ipc::IpcMctpClient;
use pw_status::Error;
use userspace::syscall::{self, Signals};
use userspace::time::Instant;

/// DMTF DSP0239 message type for PLDM.
pub const MSG_TYPE_PLDM: u8 = 0x01;
/// DMTF DSP0239 message type for SPDM.
pub const MSG_TYPE_SPDM: u8 = 0x05;

/// Echo `msg_type` requests arriving on `channel` forever. Never returns;
/// a routing error shuts the whole test down with a failure.
pub fn serve(channel: u32, ctl: u32, msg_type: u8) -> ! {
    let stack = Stack::new(IpcMctpClient::new(channel));
    // timeout 0: each recv blocks until the server has a message for us.
    let mut listener = match stack.listener(msg_type, 0) {
        Ok(l) => l,
        Err(e) => {
            pw_log::error!(
                "echo {}: listener failed code={}",
                msg_type as u32,
                e.code as u32
            );
            fail();
        }
    };
    pw_log::info!("echo {}: listening", msg_type as u32);
    ready(ctl, msg_type);

    let mut buf = [0u8; config::MAX_PAYLOAD];
    loop {
        match listener.recv(&mut buf) {
            Ok((meta, msg, mut resp)) => {
                if meta.msg_type != msg_type {
                    pw_log::error!(
                        "echo {}: misrouted message of type {}",
                        msg_type as u32,
                        meta.msg_type as u32
                    );
                    fail();
                }
                if let Err(e) = resp.send(msg) {
                    pw_log::error!(
                        "echo {}: send failed code={}",
                        msg_type as u32,
                        e.code as u32
                    );
                    fail();
                }
            }
            Err(e) => {
                pw_log::error!(
                    "echo {}: recv failed code={}",
                    msg_type as u32,
                    e.code as u32
                );
                fail();
            }
        }
    }
}

/// Report failure to the kernel target and park.
fn fail() -> ! {
    let _ = syscall::debug_shutdown(Err(Error::Internal));
    #[expect(clippy::empty_loop)]
    loop {}
}

/// Answer the harness's one `ctl` transaction: "my listener is registered".
fn ready(ctl: u32, msg_type: u8) {
    let mut scratch = [0u8; 1];
    let ok = syscall::object_wait(ctl, Signals::READABLE, Instant::MAX).is_ok()
        && syscall::channel_read(ctl, 0, &mut scratch).is_ok()
        && syscall::channel_respond(ctl, &[0u8; 0]).is_ok();
    if !ok {
        pw_log::error!("echo {}: ctl barrier failed", msg_type as u32);
        fail();
    }
}
