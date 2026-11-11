// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! I3C client process.
//!
//! Drives `//services/i3c/client` over `//services/i3c/client-ipc` — the same
//! typed calls any consumer makes, so this test doubles as the on-target proof
//! that the client crate works against the real server.
//!
//! Walks the full `dispatch()` surface of `//services/i3c/server`, in order:
//!
//! 1. `recv` before any frame has arrived -> `None` (the empty latch).
//! 2. `dynamic_address` -> the address the Caliptra ROM assigned.
//! 3. park in `object_wait(CHANNEL, USER)`, then `recv` -> the host's payload.
//! 4. a second `recv` -> `None` again, proving `recv` consumed the latch.
//! 5. `send` -> stages a TX the host collects with a private read.
//!
//! Step 3 is also the wake this test exists to exercise: the server raises USER
//! from inside its I3C interrupt handling, so the process made runnable by that
//! interrupt is a *different* process from the one that was interrupted.

#![no_main]
#![no_std]

use client_codegen::handle;
use i3c_api::MAX_PAYLOAD;
use i3c_client::I3cClient;
use i3c_client_ipc::IpcTransport;
use userspace::process_entry;
use userspace::syscall::{self, Signals};
use userspace::time::Instant;

/// The host sends exactly this private-write payload.
const EXPECTED: &[u8] = &[0x01, 0x02, 0x03, 0x04];

/// The payload the firmware stages for the host to collect by private read.
const REPLY: &[u8] = &[0xa5, 0x5a, 0xc3, 0x3c];

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

    // 1. Nothing has arrived on the bus yet: the latch must read empty.
    match i3c.recv(&mut buf) {
        Ok(None) => {}
        Ok(Some(_)) => fail!("expected no frame from recv before any arrived"),
        Err(_) => fail!("recv failed"),
    }

    // 2. The ROM assigns the dynamic address during bus enumeration; the server
    //    reads it back out of the standby-controller registers.
    match i3c.dynamic_address() {
        Ok(Some(addr)) => pw_log::info!("i3c service: dynamic address {:#04x}", addr as u32),
        Ok(None) => fail!("dynamic address unassigned"),
        Err(_) => fail!("dynamic_address failed"),
    }

    pw_log::info!("i3c service: waiting for private write");

    // 3. Park until the server signals -- from its IRQ path -- that a frame has
    //    been latched. This is the cross-process wake under test.
    loop {
        if syscall::object_wait(handle::CHANNEL, Signals::USER, Instant::MAX).is_err() {
            fail!("object_wait failed");
        }

        match i3c.recv(&mut buf) {
            // USER raced ahead of the latch; park again rather than failing.
            Ok(None) => continue,
            Ok(Some(n)) => {
                if n < EXPECTED.len() || buf.get(..EXPECTED.len()) != Some(EXPECTED) {
                    pw_log::error!("i3c service: payload mismatch len={}", n as u32);
                    let _ = syscall::debug_shutdown(Err(pw_status::Error::DataLoss));
                    loop {}
                }
                break;
            }
            Err(_) => fail!("recv failed"),
        }
    }
    pw_log::info!("i3c service: received expected payload");

    // 4. `recv` consumes the latch, so an immediate second read must be empty.
    match i3c.recv(&mut buf) {
        Ok(None) => {}
        Ok(Some(_)) => fail!("expected no frame from second recv: latch was not consumed"),
        Err(_) => fail!("recv failed"),
    }

    // 5. Stage a transmit. The host collects it with a private read, which is
    //    what proves the Send path reached the hardware.
    if i3c.send(REPLY).is_err() {
        fail!("send failed");
    }
    pw_log::info!("i3c service: staged reply for private read");

    let _ = syscall::debug_shutdown(Ok(()));
    loop {}
}
