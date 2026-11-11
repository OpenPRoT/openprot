// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Back-pressure client.
//!
//! On one trigger frame from the host, stages three distinct responses in a
//! row with `//services/i3c/client`. The i3c server holds each `Send` until the
//! controller has read the previous one, so `send` blocks and the three frames
//! reach the host strictly in order, one per private read. Without back-pressure
//! the later sends would overwrite the staged response and the host would not
//! see A, B, C in sequence.
//!
//! `send` returns once a frame is *staged*, not read, so exiting immediately
//! after `send(C)` would race the process teardown against the host's read of C
//! and drop it. Instead of guessing at a delay, the client rendezvous with the
//! host: after staging the three, it blocks on `recv` for a completion write the
//! host sends only once it has read all three, then exits 0. This mirrors the
//! recv-then-exit shape of `tests/i3c_service`.

#![no_main]
#![no_std]

use client_codegen::handle;
use i3c_api::MAX_PAYLOAD;
use i3c_client::I3cClient;
use i3c_client_ipc::IpcTransport;
use userspace::process_entry;
use userspace::syscall::{self, Signals};
use userspace::time::Instant;

/// The three distinct frames the host expects back, in order.
const FRAME_A: &[u8] = &[0xa1, 0xa2, 0xa3, 0xa4];
const FRAME_B: &[u8] = &[0xb1, 0xb2, 0xb3, 0xb4];
const FRAME_C: &[u8] = &[0xc1, 0xc2, 0xc3, 0xc4];
/// First byte of the host's completion write. The rendezvous matches on this so
/// a stray/earlier latched frame cannot end the wait prematurely.
const DONE_MARKER: u8 = 0xee;

macro_rules! fail {
    ($msg:literal) => {{
        pw_log::error!($msg);
        let _ = syscall::debug_shutdown(Err(pw_status::Error::Internal));
        loop {}
    }};
}

#[process_entry("client")]
fn entry() {
    let mut i3c = I3cClient::new(IpcTransport::new(handle::CHANNEL));
    let mut buf = [0u8; MAX_PAYLOAD];

    pw_log::info!("i3c backpressure: waiting for trigger");

    // Park until the host's trigger private write is latched, then consume it.
    loop {
        if syscall::object_wait(handle::CHANNEL, Signals::USER, Instant::MAX).is_err() {
            fail!("object_wait failed");
        }
        match i3c.recv(&mut buf) {
            Ok(Some(_)) => break,
            Ok(None) => continue,
            Err(_) => fail!("recv failed"),
        }
    }

    pw_log::info!("i3c backpressure: staging three frames");

    // Each send blocks until the controller has read the previous staged frame,
    // so by the time this loop returns the host has read A and B and C is
    // staged.
    for frame in [FRAME_A, FRAME_B, FRAME_C] {
        if i3c.send(frame).is_err() {
            fail!("send failed");
        }
    }

    pw_log::info!("i3c backpressure: three frames staged");

    // Rendezvous: block until the host's completion write, which it sends only
    // after reading all three frames. This guarantees C is delivered before we
    // tear down the emulator, with no timing guess. Match on DONE_MARKER so a
    // stray or earlier-latched frame does not end the wait early.
    loop {
        if syscall::object_wait(handle::CHANNEL, Signals::USER, Instant::MAX).is_err() {
            fail!("object_wait failed");
        }
        match i3c.recv(&mut buf) {
            Ok(Some(_)) if buf.first() == Some(&DONE_MARKER) => break,
            Ok(_) => continue,
            Err(_) => fail!("recv failed"),
        }
    }

    pw_log::info!("i3c backpressure: host acknowledged, exiting");
    let _ = syscall::debug_shutdown(Ok(()));
    loop {}
}
