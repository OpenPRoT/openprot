// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! I3C server process: runs the real `//services/i3c/server-runtime` reactor,
//! identical to `tests/i3c_service`. Here it is exercised for its *outbound*
//! back-pressure: it holds a second `Send` until the controller has read the
//! first (signalled by `TargetEvent::ResponseRead`).

#![no_main]
#![no_std]

use caliptra_i3c_target::CaliptraI3cTarget;
use i3c_server::Server;
use server_codegen::{handle, signals};
use userspace::process_entry;

#[process_entry("server")]
fn entry() {
    // SAFETY: this process exclusively owns the I3C peripheral, mapped as a
    // device region by system.json5; Caliptra ROM already initialized the core.
    let target = unsafe { CaliptraI3cTarget::new() };
    let mut srv = Server::new(handle::CHANNEL, handle::I3C_IRQ, target);
    i3c_server_runtime::run(handle::WG, signals::I3C, &mut srv);
}
