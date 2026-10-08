// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Echo responder for the MCTP-over-I2C throughput test (card A, EID 8).
//!
//! Passive: listens for echo messages and reflects each payload back to the
//! sender. It never originates traffic, so the requester's measurements are
//! not perturbed by responder-initiated sends.

#![no_main]
#![no_std]

use app_mctp_echo_client::handle;
use openprot_mctp_api::wire::MAX_PAYLOAD_SIZE;
use openprot_mctp_api::Stack;
use openprot_mctp_client_ipc::IpcMctpClient;
use openprot_mctp_echo::{echo_once, prepare_listener_with_eid_and_timeout};
use userspace::{entry, syscall};

const ECHO_EID: u8 = 8;
// 0 = block until a request arrives.
const LISTEN_TIMEOUT_MS: u32 = 0;

#[entry]
fn entry() {
    let stack = Stack::new(IpcMctpClient::new(handle::MCTP));
    let mut listener =
        match prepare_listener_with_eid_and_timeout(&stack, ECHO_EID, LISTEN_TIMEOUT_MS) {
            Ok(listener) => listener,
            Err(e) => {
                pw_log::error!("echo responder setup failed: code={}", e.code as u32);
                syscall::process_exit(1);
            }
        };

    pw_log::info!("echo responder ready, EID {}", ECHO_EID as u32);
    let mut buf = [0u8; MAX_PAYLOAD_SIZE];
    loop {
        if let Err(e) = echo_once(&mut listener, &mut buf) {
            if !e.is_timeout() {
                pw_log::error!("echo responder failed: code={}", e.code as u32);
            }
        }
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
