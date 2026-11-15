// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Server side of the util/service single-deferred-channel test.
//!
//! One channel, one `ServiceLoop`. Each request is deferred and completed
//! `DELAY_MS` later from the loop's deadline. While the reply is parked the
//! only channel is out of the WaitGroup, so the WaitGroup has no members. The
//! kernel refuses to wait on an empty WaitGroup, so `next` must wait for the
//! deadline some other way. If it does not, `next` returns an error, this
//! server exits, and the client's call never gets its reply.

#![no_main]
#![no_std]

use app_server::handle;
use pw_status::Result;
use userspace::entry;
use userspace::time::{Clock, Duration, Instant, SystemClock};
use util_service::{Channel, Event, ServiceLoop};

const DELAY_MS: u64 = 50;
const KEY: u32 = 1;

fn serve() -> Result<()> {
    let mut channels = [Channel::new(handle::SVC)];
    let mut service = ServiceLoop::new(handle::WG, &mut channels)?;
    let mut buf = [0u8; 8];
    // When the parked reply is due, if one is parked.
    let mut due: Option<Instant> = None;

    loop {
        match service.next(due.unwrap_or(Instant::MAX), &mut buf)? {
            Event::Request { channel, .. } => {
                if service.defer(channel, KEY).is_ok() {
                    due = SystemClock::now().checked_add_duration(Duration::from_millis(DELAY_MS));
                } else {
                    pw_log::error!("server: defer failed");
                    let _ = service.reply(channel, b"err");
                }
            }
            Event::Deadline => {
                if let Some(at) = due
                    && SystemClock::now() >= at
                {
                    due = None;
                    if service.complete(KEY, b"ok").is_err() {
                        pw_log::error!("server: complete failed");
                    }
                }
            }
            Event::Source(_) => {}
        }
    }
}

#[entry]
fn entry() {
    if let Err(e) = serve() {
        pw_log::error!("server: loop exited, status {}", e as u32);
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
