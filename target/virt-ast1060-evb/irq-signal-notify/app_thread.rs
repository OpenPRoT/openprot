// Licensed under the Apache-2.0 license

//! App thread process
//!
//! Drives the test: triggers IRQ 42 three times.  After each trigger it
//! waits via a wait group that contains the `notify` channel_initiator handle.
//! Using a wait group demonstrates how the app can later multiplex across
//! multiple notification sources (e.g. a second IRQ or a timer) without
//! changing the wait call-site.
//!
//! Wait group setup (done once before the loop):
//!   wait_group_add(wg, notify, Signals::USER, user_data=1)
//!
//! Each iteration:
//!   1. debug_trigger_interrupt(42)   – fires IRQ in QEMU
//!   2. object_wait(wg, Signals::USER) – blocks until irq_listener raises USER
//!   3. Validate user_data and pending_signals from WaitReturn

#![no_main]
#![no_std]

use app_thread_codegen::handle;
use pw_status::{Result, StatusCode};
use userspace::process_entry;
use userspace::syscall::{self, Signals};
use userspace::time::Instant;

const EXPECTED_NOTIFICATIONS: u32 = 3;
const TEST_IRQ: u32 = 42;
// Arbitrary tag returned by object_wait when the notify handle fires
const NOTIFY_USER_DATA: usize = 0xBEEF;

fn run() -> Result<()> {
    // Register the notify channel_initiator in the wait group.
    // From this point on we call object_wait on `wg`, not on `notify` directly.
    // This makes it trivial to add a second notification source later:
    //   wait_group_add(handle::WG, handle::OTHER, Signals::USER, OTHER_DATA)
    syscall::wait_group_add(
        handle::WG,
        handle::NOTIFY,
        Signals::USER,
        NOTIFY_USER_DATA,
    )?;
    pw_log::info!("Registered notify handle in wait group");

    for i in 1..=EXPECTED_NOTIFICATIONS {
        // Fire the hardware interrupt (QEMU debug syscall).
        syscall::debug_trigger_interrupt(TEST_IRQ)?;

        // Block on the wait group — wakes when any member's signal fires.
        let wait_return =
            syscall::object_wait(handle::WG, Signals::USER, Instant::MAX)?;

        if !wait_return.pending_signals.contains(Signals::USER) {
            pw_log::error!(
                "Unexpected signals: {:#x}",
                wait_return.pending_signals.bits() as u32,
            );
            return Err(pw_status::Error::Internal);
        }

        if wait_return.user_data != NOTIFY_USER_DATA {
            pw_log::error!(
                "Unexpected user_data: {} (expected {})",
                wait_return.user_data as usize,
                NOTIFY_USER_DATA as usize,
            );
            return Err(pw_status::Error::Internal);
        }

        pw_log::info!(
            "Notification {} / {} received (user_data={:#x})",
            i as u32,
            EXPECTED_NOTIFICATIONS as u32,
            wait_return.user_data as usize,
        );
    }

    syscall::wait_group_remove(handle::WG, handle::NOTIFY)?;
    pw_log::info!("All {} notifications received", EXPECTED_NOTIFICATIONS as u32);
    Ok(())
}

#[process_entry("app_thread")]
fn entry() -> ! {
    pw_log::info!("🔄 RUNNING");

    let ret = run();

    if ret.is_err() {
        pw_log::error!("❌ FAILED: {}", ret.status_code() as u32);
    } else {
        pw_log::info!("✅ PASSED");
    }

    let _ = syscall::debug_shutdown(ret);
    loop {}
}

