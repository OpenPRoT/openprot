// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#![no_main]
#![no_std]

use openprot_mctp_api::Stack;
use openprot_mctp_client_ipc::IpcMctpClient;
use openprot_mctp_echo::{prepare_listener_with_eid_and_timeout, run_with_peer_round_trip_limit};
use userspace::{entry, syscall};

// Peer echo endpoint at EID 9, sends to primary at EID 8.
const ECHO_EID: u8 = 9;
const PEER_EID: u8 = 8;
const LISTEN_TIMEOUT_MS: u32 = 100;
const MAX_ROUND_TRIPS: u32 = 64;

#[entry]
fn entry() {
    let stack = Stack::new(IpcMctpClient::new(app_mctp_echo_client_peer::handle::MCTP));
    let mut listener =
        match prepare_listener_with_eid_and_timeout(&stack, ECHO_EID, LISTEN_TIMEOUT_MS) {
            Ok(listener) => listener,
            Err(e) => {
                pw_log::error!("echo setup failed: code={}", e.code as u32);
                syscall::process_exit(1);
            }
        };

    run_with_peer_round_trip_limit(&stack, PEER_EID, &mut listener, MAX_ROUND_TRIPS);
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
