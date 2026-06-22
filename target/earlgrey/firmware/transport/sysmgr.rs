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

    syscall::wait_group_add(
        handle::SYSMGR_WAIT_GROUP,
        handle::SYSMGR_UPDATER_SERVICE,
        Signals::READABLE,
        1, // token 1 = Updater
    )
    .map_err(ErrorCode::kernel_error)?;

    syscall::wait_group_add(
        handle::SYSMGR_WAIT_GROUP,
        handle::SYSMGR_USB_SERVICE,
        Signals::READABLE,
        2, // token 2 = USB
    )
    .map_err(ErrorCode::kernel_error)?;

    let updater_channel = IpcHandle::new(handle::SYSMGR_UPDATER_SERVICE);
    let usb_channel = IpcHandle::new(handle::SYSMGR_USB_SERVICE);
    let mut buf = [0u8; 1024];

    loop {
        // Wait for incoming IPC request.
        let wait_result =
            syscall::object_wait(handle::SYSMGR_WAIT_GROUP, Signals::READABLE, Instant::MAX)
                .map_err(ErrorCode::kernel_error)?;

        let token = wait_result.user_data;
        if token == 1 {
            server.handle_one(&updater_channel, &mut buf)?;
        } else if token == 2 {
            server.handle_one(&usb_channel, &mut buf)?;
        }
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
