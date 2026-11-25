// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! I3C server process: runs the real `//services/i3c/server-runtime` reactor.
//!
//! This is the deployment shape that crate was written for -- its BUILD file
//! notes the reactor logic is "verified on-target (veer emulator), not host",
//! and this is that test.
//!
//! `run()` never returns. On an inbound bus frame it latches the payload,
//! `interrupt_ack`s the level-triggered source, and raises `USER` on the client
//! channel so the parked client process wakes.

#![no_main]
#![no_std]

use caliptra_i3c_target::CaliptraI3cTarget;
use i3c_server::Server;
use openprot_hal_blocking::i3c_hardware::I3cTargetConfig;
use server_codegen::{handle, signals};
use userspace::process_entry;

/// An IBI Mandatory Data Byte that is not the MCTP pending-read value.
const NON_MCTP_MDB: u8 = 0x1F;

#[process_entry("server")]
fn entry() {
    // SAFETY: this process exclusively owns the I3C peripheral, mapped as a
    // device region by system.json5; Caliptra ROM already initialized the core.
    let mut target = unsafe { CaliptraI3cTarget::new() };
    // Raise IBIs with a non-MCTP Mandatory Data Byte. After an MCTP pending-read
    // IBI (0xAE) the FPGA hardware model reads the reply itself at once, which
    // would release the server's outbound back-pressure before a second send
    // could be refused as busy. With any other MDB the reply stays queued until
    // the host reads it.
    target.set_config(I3cTargetConfig {
        static_address: None,
        ibi_mdb: NON_MCTP_MDB,
    });
    let mut srv = Server::new(handle::CHANNEL, handle::I3C_IRQ, target);
    if let Err(e) = i3c_server_runtime::run(handle::WG, signals::I3C, &mut srv) {
        pw_log::error!("i3c server stopped: {}", e as u32);
    }
}
