// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Answers the commands that `host_test.rs` sends to the two MCU mailboxes.
//!
//! The app releases the lock that the MCU holds on each mailbox after a reset
//! and then polls the mailboxes for commands:
//!
//! - [`INVERT`]: the response is the request with every bit inverted.
//! - [`FILL`]: the request is a length as a little-endian word, and the
//!   response has that many bytes. Byte `i` is the low byte of `i`.
//! - [`USER`]: the response is the AXI user of the requester as a
//!   little-endian word.
//! - [`SIZE`]: the response is the size in bytes of the SRAM that the app
//!   gave the driver, as a little-endian word.
//! - [`DONE`]: the command completes without response data.
//!
//! Any other command fails.
//!
//! The app is a userspace process that maps the mailboxes and no other part
//! of the MCI. On the VCK190 an access outside the mappings would stop the
//! app. The addresses and the sizes of the SRAMs come from the system config.

#![no_std]
#![no_main]

use pw_status::Error;
use responder_codegen::{constants, mapping};
use userspace::{entry, syscall};
use veer_mcu_mbox::{McuMailbox, Request, Status};

/// "INVT".
const INVERT: u32 = 0x494e_5654;
/// "FILL".
const FILL: u32 = 0x4649_4c4c;
/// "USER".
const USER: u32 = 0x5553_4552;
/// "SIZE".
const SIZE: u32 = 0x5349_5a45;
/// "DONE".
const DONE: u32 = 0x444f_4e45;

/// Number of bytes that the app reads and writes with one call of the
/// driver. Response data is written at multiples of 4 bytes.
const CHUNK: usize = 256;

/// Where `invert` splits the read of a chunk, so that the second read starts
/// inside a word.
const SPLIT: usize = 3;

type Result = core::result::Result<(), veer_mcu_mbox::Error>;

/// Inverts the request a chunk at a time: each chunk of the response goes
/// over the chunk of the request that it was made from.
fn invert(mut request: Request<'_>) -> Result {
    let len = request.len();
    let mut chunk = [0u8; CHUNK];
    let mut offset = 0;
    while offset < len {
        let count = (len - offset).min(CHUNK);
        let part = chunk
            .get_mut(..count)
            .ok_or(veer_mcu_mbox::Error::OutOfRange)?;
        let split = count.min(SPLIT);
        if let Some(head) = part.get_mut(..split) {
            request.read(offset, head)?;
        }
        if let Some(tail) = part.get_mut(split..) {
            request.read(offset + split, tail)?;
        }
        for byte in part.iter_mut() {
            *byte = !*byte;
        }
        request.write(offset, part)?;
        offset += count;
    }
    request.finish(Status::DataReady, len)
}

fn fill(mut request: Request<'_>) -> Result {
    let mut len = [0u8; 4];
    request.read(0, &mut len)?;
    let len = u32::from_le_bytes(len) as usize;
    let mut chunk = [0u8; CHUNK];
    let mut offset = 0;
    while offset < len {
        for (i, byte) in chunk.iter_mut().enumerate() {
            *byte = (offset + i) as u8;
        }
        let count = (len - offset).min(CHUNK);
        let part = chunk.get(..count).ok_or(veer_mcu_mbox::Error::OutOfRange)?;
        request.write(offset, part)?;
        offset += count;
    }
    request.finish(Status::DataReady, len)
}

/// Answers `request`. `sram_size` is the size of the SRAM of its mailbox.
fn handle(request: Request<'_>, sram_size: usize) -> Result {
    match request.command() {
        INVERT => invert(request),
        FILL => fill(request),
        USER => {
            let user = request.user().to_le_bytes();
            request.respond(Status::DataReady, &user)
        }
        SIZE => request.respond(Status::DataReady, &(sram_size as u32).to_le_bytes()),
        DONE => request.finish(Status::Complete, 0),
        _ => request.finish(Status::Failure, 0),
    }
}

#[entry]
fn entry() -> pw_status::Result<()> {
    // SAFETY: the system config maps the SRAM and the registers of both MCU
    // mailboxes for this process, with these addresses and sizes, and this
    // image has no other user of the mailboxes.
    let mut mailboxes = unsafe {
        [
            McuMailbox::new(
                mapping::MCU_MBOX0_SRAM_START_ADDRESS,
                constants::MCU_MBOX0_SRAM_SIZE as usize,
            ),
            McuMailbox::new(
                mapping::MCU_MBOX1_SRAM_START_ADDRESS,
                constants::MCU_MBOX1_SRAM_SIZE as usize,
            ),
        ]
    };
    for mbox in mailboxes.iter_mut() {
        mbox.clear();
    }
    pw_log::info!("The MCU mailboxes are ready");
    loop {
        for (index, mbox) in mailboxes.iter_mut().enumerate() {
            let sram_size = mbox.sram_size();
            let Some(request) = mbox.request() else {
                continue;
            };
            let command = request.command();
            // If `handle` returns an error, it has dropped the request,
            // which fails the command.
            if handle(request, sram_size).is_err() {
                pw_log::error!(
                    "Command {:#x} in mailbox {} failed",
                    command as u32,
                    index as u32
                );
            }
        }
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    pw_log::error!("FAIL: panic in the MCU mailbox responder");
    let _ = syscall::debug_shutdown(Err(Error::Internal));
    loop {}
}
