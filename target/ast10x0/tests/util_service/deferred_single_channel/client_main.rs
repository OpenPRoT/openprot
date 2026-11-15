// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Client side of the util/service single-deferred-channel test.
//!
//! Makes two blocking calls to a server whose only channel is deferred for
//! 50 ms, and checks that each reply arrives, and not before the delay. The
//! second call shows the channel is served again after the first reply: a
//! server that exits when its WaitGroup goes empty fails the first call, and
//! one that never re-arms its channel fails the second.
//!
//! Calls `debug_shutdown(Ok|Err)` to report the result.

#![no_main]
#![no_std]

use app_client::handle;
use pw_status::{Error, Result};
use userspace::entry;
use userspace::syscall;
use userspace::time::{Clock, Duration, Instant, SystemClock};

/// Long enough for a healthy run, short enough that a dead server fails the
/// test promptly instead of waiting for the test runner's timeout.
const CALL_LIMIT: Duration = Duration::from_millis(2000);
/// The server holds each reply for 50 ms. A reply faster than this means the
/// server did not wait for its deadline.
const AT_LEAST: Duration = Duration::from_millis(40);

fn call() -> Result<()> {
    let start = SystemClock::now();
    let deadline = start
        .checked_add_duration(CALL_LIMIT)
        .unwrap_or(Instant::MAX);
    let mut resp = [0u8; 8];

    let n = syscall::channel_transact(handle::SVC, b"hi", &mut resp, deadline)?;

    if resp.get(..n) != Some(b"ok") {
        pw_log::error!("deferred single channel: unexpected reply");
        return Err(Error::Internal);
    }
    if SystemClock::now() - start < AT_LEAST {
        pw_log::error!("deferred single channel: reply came before the delay");
        return Err(Error::Internal);
    }
    Ok(())
}

fn run() -> Result<()> {
    call()?;
    call()
}

#[entry]
fn entry() {
    let ret = run();
    match &ret {
        Ok(()) => pw_log::info!("deferred single channel: PASSED"),
        Err(e) => pw_log::error!("deferred single channel: FAILED, status {}", *e as u32),
    }
    let _ = syscall::debug_shutdown(ret);
    loop {}
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    let _ = userspace::syscall::debug_shutdown(Err(pw_status::Error::Internal));
    loop {}
}
