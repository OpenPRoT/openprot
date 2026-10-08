// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! MCTP-over-I2C throughput requester (card B, EID 9).
//!
//! Sends `ITERS` echo requests per payload size to the responder at EID 8,
//! verifies each echoed payload, and logs round-trip rate and goodput. Ends
//! the test with PASS only if every round trip completed with matching data.
//!
//! Goodput counts payload bytes in both directions (request + echo).

#![no_main]
#![no_std]

use app_mctp_echo_client_peer::handle;
use openprot_mctp_api::wire::MAX_PAYLOAD_SIZE;
use openprot_mctp_api::{MctpReqChannel, Stack};
use openprot_mctp_client_ipc::IpcMctpClient;
use openprot_mctp_echo::ECHO_MSG_TYPE;
use pw_status::Error;
use userspace::time::{Clock, SystemClock};
use userspace::{entry, syscall};

const OWN_EID: u8 = 9;
const RESPONDER_EID: u8 = 8;
const REQ_TIMEOUT_MS: u32 = 1000;
const WARMUP_TIMEOUT_MS: u32 = 10_000;
const ITERS: u32 = 100;
const PAYLOAD_SIZES: [usize; 6] = [1, 32, 64, 128, 255, 1023];

fn now_micros() -> u64 {
    SystemClock::now().ticks() * 1_000_000 / SystemClock::TICKS_PER_SEC
}

/// One echo round trip; `true` if the response matched the request.
fn round_trip(stack: &Stack<IpcMctpClient>, tx: &[u8], rx: &mut [u8], timeout: u32) -> bool {
    let Ok(mut req) = stack.req(RESPONDER_EID, timeout) else {
        return false;
    };
    if req.send(ECHO_MSG_TYPE, tx).is_err() {
        return false;
    }
    matches!(req.recv(rx), Ok((_, echoed)) if echoed == tx)
}

fn fail() -> ! {
    let _ = syscall::debug_shutdown(Err(Error::Internal));
    loop {}
}

#[entry]
fn entry() {
    let stack = Stack::new(IpcMctpClient::new(handle::MCTP));
    if let Err(e) = stack.set_eid(OWN_EID) {
        pw_log::error!("set_eid failed: code={}", e.code as u32);
        fail();
    }

    let mut tx = [0u8; MAX_PAYLOAD_SIZE];
    let mut rx = [0u8; MAX_PAYLOAD_SIZE];
    for (i, b) in tx.iter_mut().enumerate() {
        *b = i as u8;
    }

    // The responder may boot after us; retry until the first echo succeeds.
    pw_log::info!("waiting for responder at EID {}", RESPONDER_EID as u32);
    while !round_trip(&stack, &tx[..1], &mut rx, WARMUP_TIMEOUT_MS) {}

    let mut total_errors = 0u32;
    for &size in PAYLOAD_SIZES.iter() {
        let mut errors = 0u32;
        let start = now_micros();
        for _ in 0..ITERS {
            if !round_trip(&stack, &tx[..size], &mut rx, REQ_TIMEOUT_MS) {
                errors += 1;
            }
        }
        let elapsed_us = now_micros().saturating_sub(start).max(1);
        let ok = (ITERS - errors) as u64;
        let rt_per_sec_x100 = ok * 100_000_000 / elapsed_us;
        let bytes_per_sec = ok * 2 * size as u64 * 1_000_000 / elapsed_us;
        pw_log::info!(
            "THROUGHPUT size={} ok={} err={} elapsed_us={} rt_per_sec_x100={} bytes_per_sec={}",
            size as u32,
            ok as u32,
            errors as u32,
            elapsed_us as u32,
            rt_per_sec_x100 as u32,
            bytes_per_sec as u32
        );
        total_errors += errors;
    }

    if total_errors == 0 {
        pw_log::info!("throughput test complete");
        let _ = syscall::debug_shutdown(Ok(()));
    } else {
        pw_log::error!("throughput test: {} failed round trips", total_errors as u32);
        fail();
    }
    loop {}
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
