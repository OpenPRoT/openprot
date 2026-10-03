// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! The RoT side of the mock-BMC integration test.
//!
//! Scenario: release the managed device from reset, wait for it to report
//! ready, and fail if it never does. That is the shape every boot-sequence
//! scenario has; the richer ones differ in what the device does and what
//! deadline the RoT allows, not in this structure.
//!
//! Evidence arrives as a request on `boot_evt` and is latched here. The
//! orchestrator never asks the device whether it is up, because
//! `EvidenceReader::read` is synchronous and may not block. Reading a latch
//! is the only shape that satisfies that, and it matches the contract that
//! evidence is cleared by the reset path rather than by the reader.

#![no_main]
#![no_std]

use userspace::syscall::Signals;
use userspace::time::{Clock, Instant, SystemClock};
use userspace::{entry, syscall};

use app_orchestrator::handle;

/// How long the RoT is willing to wait for the device to report ready. The
/// device boots in 50 ms, so this is slack, not a race.
const BOOT_WINDOW_MILLIS: u64 = 2_000;

/// Reset command bytes, read by the device as its reset line.
const RESET_ASSERT: u8 = 0;
const RESET_RELEASE: u8 = 1;

fn millis_to_ticks(millis: u64) -> u64 {
    millis * SystemClock::TICKS_PER_SEC / 1000
}

fn deadline_in(millis: u64) -> Instant {
    Instant::from_ticks(SystemClock::now().ticks() + millis_to_ticks(millis))
}

/// Drives the device's reset line. Returns an error if the command could
/// not be delivered, which is a wiring failure rather than a boot failure.
fn drive_reset(level: u8) -> Result<(), ()> {
    let mut ack = [0u8; 1];
    syscall::channel_transact(handle::RESET_CMD, &[level], &mut ack, deadline_in(1_000))
        .map(|_| ())
        .map_err(|_| ())
}

fn run() -> Result<(), ()> {
    syscall::wait_group_add(handle::WG, handle::BOOT_EVT, Signals::READABLE, 0usize)
        .map_err(|_| ())?;

    // Hold the device in reset, then let it go. Asserting first means the
    // test does not depend on what the line happened to be at boot.
    drive_reset(RESET_ASSERT)?;
    drive_reset(RESET_RELEASE)?;

    // Latch the ready report. A real reader would return this latch from
    // EvidenceReader::read without blocking; here the assertion is the
    // latch itself being set before the window closes.
    let mut booted = false;
    let window = deadline_in(BOOT_WINDOW_MILLIS);
    let mut evt = [0u8; 1];

    while !booted {
        if syscall::object_wait(handle::WG, Signals::READABLE, window).is_err() {
            // The window closed with no report: the device never came up.
            pw_log::error!("mock BMC never reported ready");
            return Err(());
        }

        if let Ok(len) = syscall::channel_read(handle::BOOT_EVT, 0usize, &mut evt) {
            let _ = syscall::channel_respond(handle::BOOT_EVT, &[0u8; 0]);
            if len == 1usize && evt[0] == 1 {
                booted = true;
            }
        }
    }

    pw_log::info!("mock BMC reported ready");
    Ok(())
}

#[entry]
fn entry() {
    match run() {
        Ok(()) => {
            let _ = syscall::debug_shutdown(Ok(()));
        }
        Err(()) => {
            let _ = syscall::debug_shutdown(Err(pw_status::Error::Internal));
        }
    }
    #[expect(clippy::empty_loop)]
    loop {}
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
