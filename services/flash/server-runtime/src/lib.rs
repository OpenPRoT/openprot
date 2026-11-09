// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Flash server IPC loop.
//!
//! Wraps the platform-independent `openprot_flash_server::Server` in a wait on
//! one channel. The server decides what a request means and builds the
//! response frame. This crate only moves the bytes.

#![no_std]

use hal_flash::Flash;
use openprot_flash_server::Server;
use userspace::syscall::{self, Signals};
use userspace::time::Instant;
use util_error::ErrorCode;
use util_ipc::{IpcHandle, IpcHandler};

/// Serve requests on the channel `handle` forever.
///
/// `buf` holds one request and its reply, so it must fit the largest of
/// either plus the opcode or status header.
pub fn run<TFlash: Flash<Error = ErrorCode>>(
    handle: u32,
    server: &mut Server<TFlash>,
    buf: &mut [u8],
) -> ! {
    let ipc = IpcHandle::new(handle);
    loop {
        if syscall::object_wait(handle, Signals::READABLE, Instant::MAX).is_err() {
            continue;
        }
        if let Err(e) = handle_one(server, &ipc, buf) {
            pw_log::error!("flash server: request failed: {:08x}", e.0.get() as u32);
        }
    }
}

fn handle_one<TFlash: Flash<Error = ErrorCode>>(
    server: &mut Server<TFlash>,
    ipc: &IpcHandle,
    data: &mut [u8],
) -> Result<(), ErrorCode> {
    let len = ipc.read(0, data).map_err(ErrorCode::kernel_error)?;
    let frame_len = server.handle_frame(data, len);
    ipc.respond(data.get(..frame_len).unwrap_or(&[]))
        .map_err(ErrorCode::kernel_error)?;
    Ok(())
}
