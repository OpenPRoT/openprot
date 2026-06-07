// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#![no_std]
#![no_main]

use pw_status::Error;
use userspace::time::{sleep_until, Instant};
use userspace::{process_entry, syscall};
use util_error::{AsStatus, ErrorCode};
use util_zfmt::messages::{ProcessExit, ProcessStart};

/*
 * TODO: implement flash server.
 */

fn flash_server() -> Result<(), ErrorCode> {
    loop {
        let wake_time = syscall::debug_clock_now().ticks() + 10_000_000;
        sleep_until(Instant::from_ticks(wake_time)).map_err(ErrorCode::kernel_error)?;
    }
}

#[process_entry("flash_server")]
fn entry() -> Result<(), Error> {
    pw_log::info!("flash_server");
    util_zfmt::info!(ProcessStart {
        name: "flash_server"
    });
    let ret = flash_server();
    util_zfmt::error!(ProcessExit {
        name: "flash_server",
        status: ret.as_status()
    });

    Err(Error::Unknown)
}
