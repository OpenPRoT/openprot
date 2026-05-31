// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#![no_std]
#![no_main]
use pw_status::Result;
use userspace::time::{sleep_until, Instant};
use userspace::{process_entry, syscall};

/*
 * TODO: implement flash server.
 */

#[process_entry("flash_server")]
fn entry() -> Result<()> {
    pw_log::debug!("TODO: implement flash server");
    loop {
        let wake_time = syscall::debug_clock_now().ticks() + 10_000_000;
        sleep_until(Instant::from_ticks(wake_time))?;
    }
}
