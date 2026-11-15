// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Simulated verifier service.
//!
//! How long a check takes is the service's own business, fixed when the image
//! is built: each component has a latency per attempt, and the orchestrator
//! only names the component. The service counts the attempts itself.
//!
//! Request:  `[OP_VERIFY, id]`
//! Response: `[status, id]`    (status 0 is "authenticated")
//!
//! The reply is deferred. While it is parked the loop keeps running, but the
//! channel is out of the WaitGroup, so a client that cancels still cannot
//! have a new request read until the deferred reply is completed. The
//! scenario's timings account for that.
//!
//! This is a single-channel `ServiceLoop`: while the reply is parked the
//! WaitGroup is empty, and `next` sleeps until the reply is due.

#![no_main]
#![no_std]

use app_verifier::handle;
use pw_status::Result;
use userspace::entry;
use userspace::time::{Clock, Duration, Instant, SystemClock};
use util_service::{Channel, Event, ServiceLoop};

const OP_VERIFY: u8 = 1;
const KEY: u32 = 1;
const COMPONENTS: usize = 3;

/// How long each component's nth verify takes, in milliseconds. The last
/// entry repeats once the list runs out.
///
/// C1's first verify is slow (210 ms), long enough that C0's 110 ms boot
/// watchdog fires while it is still running, and short enough to finish inside
/// the orchestrator's 300 ms verify deadline. The three gaps are laid out in
/// orchestrator_main.rs, beside `BOOT_WINDOW`.
#[cfg(flaky_boot)]
const LATENCY_MS: [&[u64]; COMPONENTS] = [&[10], &[210, 10], &[10]];
/// C2's first verify takes 450 ms, past the orchestrator's 300 ms deadline.
///
/// Two margins decide whether this scenario is steady, with D the deadline
/// (300), H this latency (450) and 10 ms for a normal verify:
///
/// - `H - D` = 150 ms: the orchestrator must give up and cancel before this
///   reply lands. If it wakes later than that, the reply wins and no verify is
///   ever abandoned.
/// - `2D - (H + 10)` = 140 ms: after the cancel this service cannot read the
///   next request until it finishes this one, so the re-walk's first verify,
///   which starts about D in and has its own D to finish, is answered at
///   H + 10. If H grew past about 2D the restart would time out too.
///
/// So H has to sit between D and 2D, and widening only one side breaks the
/// other. Scale D and H together to buy more margin.
#[cfg(verify_hangs)]
const LATENCY_MS: [&[u64]; COMPONENTS] = [&[10], &[10], &[450, 10]];
#[cfg(not(any(flaky_boot, verify_hangs)))]
const LATENCY_MS: [&[u64]; COMPONENTS] = [&[10], &[10], &[10]];

/// Milliseconds the `n`th verify of component `id` takes.
fn latency_ms(id: u8, n: u32) -> u64 {
    LATENCY_MS
        .get(usize::from(id))
        .and_then(|row| {
            usize::try_from(n)
                .ok()
                .and_then(|i| row.get(i))
                .or_else(|| row.last())
        })
        .copied()
        .unwrap_or(10)
}

fn serve() -> Result<()> {
    let mut channels = [Channel::new(handle::IPC)];
    let mut service = ServiceLoop::new(handle::WG, &mut channels)?;
    let mut buf = [0u8; 8];
    // How many verifies each component has been asked for.
    let mut calls = [0u32; COMPONENTS];
    // When the parked reply is due, and for which component.
    let mut parked: Option<(Instant, u8)> = None;

    loop {
        let deadline = parked.map_or(Instant::MAX, |(due, _)| due);
        match service.next(deadline, &mut buf)? {
            Event::Request { channel, len } => {
                let req = buf.get(..len).unwrap_or(&[]);
                if let [OP_VERIFY, id] = *req {
                    match service.defer(channel, KEY) {
                        Ok(()) => {
                            // One millisecond late on purpose: the wake must
                            // not land a tick before the deadline.
                            let n = calls.get_mut(usize::from(id)).map_or(0, |c| {
                                let was = *c;
                                *c = c.saturating_add(1);
                                was
                            });
                            let after = Duration::from_millis(latency_ms(id, n).saturating_add(1));
                            let due = SystemClock::now()
                                .checked_add_duration(after)
                                .unwrap_or(Instant::MAX);
                            parked = Some((due, id));
                        }
                        Err(_) => {
                            pw_log::error!("verifier: defer failed");
                            let _ = service.reply(channel, &[1, id]);
                        }
                    }
                } else {
                    let _ = service.reply(channel, &[1, 0]);
                }
            }
            Event::Deadline => {
                if let Some((due, id)) = parked
                    && SystemClock::now() >= due
                {
                    parked = None;
                    // Fails if the client gave up (cancelled) while the reply
                    // was parked. The channel is served again either way.
                    if service.complete(KEY, &[0, id]).is_err() {
                        pw_log::warn!("verifier: late reply dropped (client cancelled)");
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
        pw_log::error!("verifier: loop exited, status {}", e as u32);
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
