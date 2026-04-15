// Licensed under the Apache-2.0 license

//! App thread process
//!
//! Drives the test: triggers IRQ 42 three times, waiting for `Signals::USER`
//! on the `notify` channel_initiator after each trigger.  The `USER` signal
//! is raised by `irq_listener` via `object_raise_peer_user_signal` once it has
//! acknowledged the interrupt.

#![no_main]
#![no_std]

use app_thread_codegen::handle;
use pw_status::{Result, StatusCode};
use userspace::process_entry;
use userspace::syscall::{self, Signals};
use userspace::time::Instant;

const EXPECTED_NOTIFICATIONS: u32 = 3;
const TEST_IRQ: u32 = 42;

fn run() -> Result<()> {
    for i in 1..=EXPECTED_NOTIFICATIONS {
        // Trigger the hardware interrupt.  In production this would be a real
        // hardware event; here we use the kernel debug syscall.
        syscall::debug_trigger_interrupt(TEST_IRQ)?;

        // Block until irq_listener raises Signals::USER on our channel handle.
        let wait_return =
            syscall::object_wait(handle::NOTIFY, Signals::USER, Instant::MAX)?;

        if !wait_return.pending_signals.contains(Signals::USER) {
            pw_log::error!("Unexpected signals: {}", wait_return.pending_signals.bits() as u32);
            return Err(pw_status::Error::Internal);
        }

        pw_log::info!("Notification {} / {} received", i as u32, EXPECTED_NOTIFICATIONS as u32);
    }

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
