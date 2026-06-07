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
 * TODO: implement platform server.
 */

fn platform_server() -> Result<(), ErrorCode> {
    loop {
        let wake_time = syscall::debug_clock_now().ticks() + 10_000_000;
        sleep_until(Instant::from_ticks(wake_time)).map_err(ErrorCode::kernel_error)?;
        break;
    }
    Ok(())
}

#[process_entry("platform")]
fn entry() -> Result<(), Error> {
    pw_log::info!("platform_server");
    util_zfmt::info!(ProcessStart { name: "platform" });
    let ret = platform_server();
    util_zfmt::error!(ProcessExit {
        name: "platform",
        status: ret.as_status()
    });

    let status_res = ret.map_err(|_| Error::Unknown);
    syscall::debug_shutdown(status_res)
}
