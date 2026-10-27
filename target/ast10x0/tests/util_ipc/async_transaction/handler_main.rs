// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Handler side of the util/ipc AsyncTransaction QEMU test.
//!
//! Exercises `util_ipc::IpcHandler`: waits for a request, reads it, and
//! responds with the request byte incremented by one. A request of
//! `GATED_REQUEST` is held until the initiator raises Signals::USER.

#![no_main]
#![no_std]

use app_handler::handle;
use pw_status::Error;
use userspace::entry;
use userspace::syscall::{self, Signals};
use userspace::time::Instant;
use util_ipc::{IpcHandle, IpcHandler};

/// Request byte that parks the handler instead of responding: it raises
/// Signals::USER on the initiator to say it is parked, then waits for the
/// initiator to raise USER back before responding. The parked signal is
/// what makes the initiator's "pending" observation deterministic.
const GATED_REQUEST: u8 = 0x40;

#[entry]
fn entry() {
    let ipc = IpcHandle::new(handle::IPC);

    loop {
        if syscall::object_wait(handle::IPC, Signals::READABLE, Instant::MAX).is_err() {
            continue;
        }

        let mut buf = [0u8; 1];
        match ipc.read(0, &mut buf) {
            Ok(1) => {
                if buf[0] == GATED_REQUEST {
                    // Tell the initiator we are parked, wait for its
                    // release, then lower the parked signal again.
                    if ipc.set_peer_user_signal(true).is_err()
                        || syscall::object_wait(handle::IPC, Signals::USER, Instant::MAX).is_err()
                    {
                        continue;
                    }
                    let _ = ipc.set_peer_user_signal(false);
                }
                buf[0] = buf[0].wrapping_add(1);
                let _ = ipc.respond(&buf);
            }
            // Transaction was cancelled by the initiator while we were
            // waking up; go back to waiting for the next one.
            Ok(_) | Err(Error::Unavailable) => continue,
            Err(_) => continue,
        }
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
