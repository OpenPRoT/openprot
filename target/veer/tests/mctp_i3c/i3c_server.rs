// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! I3C server process: runs the real `//services/i3c/server-runtime` reactor,
//! as in `tests/i3c_service`. Owns the I3C peripheral and, on an inbound bus
//! frame, latches the payload and raises `USER` on the MCTP server's channel so
//! it can collect the frame.
//!
//! The driver's default IBI Mandatory Data Byte is the MCTP pending-read value,
//! which is what an MCTP controller expects: it reads the reply after the IBI.
//! (`tests/i3c_service` overrides it to keep the hardware model from doing that.)

#![no_main]
#![no_std]

use caliptra_i3c_target::CaliptraI3cTarget;
use i3c_server::Server;
use i3c_server_codegen::{handle, signals};
use userspace::process_entry;

#[process_entry("i3c_server")]
fn entry() {
    // SAFETY: this process exclusively owns the I3C peripheral, mapped as a
    // device region by system.json5; Caliptra ROM already initialized the core.
    let target = unsafe { CaliptraI3cTarget::new() };
    let mut srv = Server::new(handle::CHANNEL, handle::I3C_IRQ, target);
    if let Err(e) = i3c_server_runtime::run(handle::WG, signals::I3C, &mut srv) {
        pw_log::error!("i3c server stopped: {}", e as u32);
    }
}
