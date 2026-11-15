// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Simulated managed devices: a reset controller and a boot-event source.
//!
//! Each device is an `openprot_device_model::DeviceModel` with a boot script
//! fixed when the image is built. The orchestrator can only drive the reset
//! line and listen; it never says how a device should behave.
//!
//! `reset` channel, answered at once with `[status, id]`:
//!   `[OP_RELEASE, id]`  release `id`'s reset; the model starts its next boot
//!   `[OP_ASSERT, id]`   hold `id` in reset; a held device has no news, so
//!                       any boot report not yet sent is dropped
//!   `[OP_RESTORE, id]`  restore `id` from its recovery source
//!
//! `events` channel, a long-poll: `[OP_WAIT]` is answered `[0, id]` with the
//! next device that booted. With none ready the reply is deferred, so one
//! request sits parked until a boot happens. That is how an event reaches a
//! client that never blocks.
//!
//! The service sleeps until the earliest moment any model says it will change
//! (`next_event_at`), so a boot is reported when it is due, not on a polling
//! interval.

#![no_main]
#![no_std]

use app_devices::handle;
use openprot_device_model::{Behaviour, DeviceModel};
use pw_status::Result;
use userspace::entry;
use userspace::time::{Clock, Instant, SystemClock};
use util_service::{Channel, Event, ServiceLoop};

const OP_RELEASE: u8 = 2;
const OP_ASSERT: u8 = 3;
const OP_RESTORE: u8 = 4;
const KEY_EVENTS: u32 = 1;
const RESET_INDEX: usize = 0;
const DEVICES: usize = 3;

/// How long a healthy device takes to come up after its release. It has to
/// beat the orchestrator's boot watchdog by a wide margin; see `BOOT_WINDOW`
/// in orchestrator_main.rs.
const BOOT_MS: u64 = 20;

const BOOTS: &[Behaviour] = &[Behaviour::Boots {
    after_millis: BOOT_MS,
}];

/// C0 does not boot the first time it is released, and boots the second.
#[cfg(flaky_boot)]
const C0_SCRIPT: &[Behaviour] = &[
    Behaviour::Hang,
    Behaviour::Boots {
        after_millis: BOOT_MS,
    },
];
/// C0 never boots, on any release.
#[cfg(never_boots)]
const C0_SCRIPT: &[Behaviour] = &[Behaviour::Hang];
#[cfg(not(any(flaky_boot, never_boots)))]
const C0_SCRIPT: &[Behaviour] = BOOTS;

fn now_ms() -> u64 {
    SystemClock::now().ticks().saturating_mul(1000) / SystemClock::TICKS_PER_SEC
}

/// The kernel instant at `ms` on the same clock as [`now_ms`].
fn instant_at(ms: u64) -> Instant {
    Instant::from_ticks(ms.saturating_mul(SystemClock::TICKS_PER_SEC) / 1000)
}

/// Drop every queued boot for `id`.
fn forget(ready: &mut heapless::Deque<u8, 8>, id: u8) {
    for _ in 0..ready.len() {
        if let Some(x) = ready.pop_front()
            && x != id
        {
            let _ = ready.push_back(x);
        }
    }
}

fn serve() -> Result<()> {
    let mut channels = [Channel::new(handle::RESET), Channel::new(handle::EVENTS)];
    let mut service = ServiceLoop::new(handle::WG, &mut channels)?;
    let mut buf = [0u8; 8];
    let mut devices: [DeviceModel<'static>; DEVICES] = [
        DeviceModel::new(C0_SCRIPT),
        DeviceModel::new(BOOTS),
        DeviceModel::new(BOOTS),
    ];
    // Devices that booted and have not been reported yet.
    let mut ready: heapless::Deque<u8, 8> = heapless::Deque::new();
    let mut parked = false;

    loop {
        // Wake when the earliest model is next due to change. One
        // millisecond late on purpose: the conversion rounds down, and a
        // wake a tick early would find nothing due and wait again.
        let deadline = devices
            .iter()
            .filter_map(|d| d.next_event_at())
            .min()
            .map_or(Instant::MAX, |ms| instant_at(ms.saturating_add(1)));

        if let Event::Request { channel, len } = service.next(deadline, &mut buf)? {
            let req = buf.get(..len).unwrap_or(&[]);
            if channel.index() == RESET_INDEX {
                let now = now_ms();
                let reply = match *req {
                    [OP_RELEASE, id] => match devices.get_mut(usize::from(id)) {
                        Some(device) => {
                            device.set_reset(false, now);
                            [0, id]
                        }
                        None => [1, id],
                    },
                    [OP_ASSERT, id] => match devices.get_mut(usize::from(id)) {
                        Some(device) => {
                            device.set_reset(true, now);
                            // The line is held, so there is no news: drop
                            // what the device had not yet reported.
                            while device.take_transition().is_some() {}
                            forget(&mut ready, id);
                            [0, id]
                        }
                        None => [1, id],
                    },
                    [OP_RESTORE, id] => [0, id],
                    _ => [1, 0],
                };
                let _ = service.reply(channel, &reply);
            } else if let Some(id) = ready.pop_front() {
                let _ = service.reply(channel, &[0, id]);
            } else if service.defer(channel, KEY_EVENTS).is_ok() {
                parked = true;
            } else {
                pw_log::error!("devices: defer failed");
                let _ = service.reply(channel, &[1, 0]);
            }
        }

        // Whatever the wake was, let time pass for every device and report
        // each one that came up.
        let now = now_ms();
        for (id, device) in devices.iter_mut().enumerate() {
            device.poll(now);
            while let Some(change) = device.take_transition() {
                if change.ready {
                    if let Ok(id) = u8::try_from(id) {
                        let _ = ready.push_back(id);
                    }
                }
            }
        }
        if parked && let Some(id) = ready.pop_front() {
            parked = false;
            if service.complete(KEY_EVENTS, &[0, id]).is_err() {
                pw_log::warn!("devices: boot event dropped (client gone)");
            }
        }
    }
}

#[entry]
fn entry() {
    if let Err(e) = serve() {
        pw_log::error!("devices: loop exited, status {}", e as u32);
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
