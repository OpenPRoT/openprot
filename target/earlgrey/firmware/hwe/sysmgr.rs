// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#![no_std]
#![no_main]

use pw_status::{Result, StatusCode};
use userspace::process_entry;
use userspace::time::{sleep_until, Clock, Duration, SystemClock};

use zfmt::Zfmt;

#[derive(Zfmt)]
#[zfmt(format = "Hello from sysmgr")]
struct SysmgrHello;

const DELAY: Duration = Duration::from_millis(1000);

fn sysmgr_server() -> Result<()> {
    loop {
        let wake_time = SystemClock::now() + DELAY;
        sleep_until(wake_time)?;
        util_zfmt::info!(SysmgrHello);
    }
}

#[process_entry("sysmgr")]
fn entry() -> Result<()> {
    let ret = sysmgr_server();
    pw_log::error!("sysmgr status = {}", ret.status_code());
    ret
}
