// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#![no_std]
#![no_main]

use earlgrey_sysmgr_server::SysmgrServer;
use pw_status::Error;
use sysmgr_codegen::handle;
use userspace::process_entry;
use userspace::syscall::{self, Signals};
use userspace::time::Instant;
use util_error::{AsStatus, ErrorCode};
use util_ipc::IpcHandle;
use util_zfmt::messages::{ProcessExit, ProcessStart};

fn sysmgr_server() -> Result<(), ErrorCode> {
    // SysmgrServer::new() will read boot log from retram and log boot info.
    let mut server = SysmgrServer::new()?;
    let service_channel = IpcHandle::new(handle::SYSMGR_SERVICE);
    let mut buf = [0u8; 1024];

    loop {
        // Wait for incoming IPC request.
        syscall::object_wait(handle::SYSMGR_SERVICE, Signals::READABLE, Instant::MAX)
            .map_err(ErrorCode::kernel_error)?;

        // Process request.
        server.handle_one(&service_channel, &mut buf)?;
    }
}

#[process_entry("sysmgr")]
fn entry() -> Result<(), Error> {
    util_zfmt::info!(ProcessStart { name: "sysmgr" });
    let ret = sysmgr_server();
    util_zfmt::error!(ProcessExit {
        name: "sysmgr",
        status: ret.as_status()
    });
    Err(Error::Unknown)
}
