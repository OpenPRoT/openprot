// Licensed under the Apache-2.0 license

//! IRQ listener process
//!
//! Waits for hardware IRQ 42, acknowledges it, then raises `Signals::USER` on
//! the peer `channel_initiator` (`app_thread`'s `notify` handle) via
//! `object_raise_peer_user_signal`.  No data is exchanged — the signal itself
//! is the notification.

#![no_main]
#![no_std]

use irq_listener_codegen::{handle, signals};
use pw_status::{Error, Result};
use userspace::process_entry;
use userspace::syscall::{self};
use userspace::time::Instant;

fn run() -> Result<()> {
    loop {
        // Block until IRQ 42 fires (signals::TEST_IRQ == Signals::INTERRUPT_A).
        let wait_return =
            syscall::object_wait(handle::IRQ, signals::TEST_IRQ, Instant::MAX)
                .map_err(|_| Error::Internal)?;

        if !wait_return.pending_signals.contains(signals::TEST_IRQ) {
            // Spurious wakeup — should not happen, but be defensive.
            continue;
        }

        // Re-enable the interrupt at the hardware level.
        syscall::interrupt_ack(handle::IRQ, wait_return.pending_signals)?;

        // Raise Signals::USER on app_thread's channel_initiator.
        // This is a fire-and-forget notification: no data, no response needed.
        syscall::object_raise_peer_user_signal(handle::NOTIFY)?;
    }
}

#[process_entry("irq_listener")]
fn entry() -> ! {
    match run() {
        Ok(()) => loop {},
        Err(e) => {
            pw_log::error!("irq_listener error: {}", e as u32);
            let _ = syscall::debug_shutdown(Err(e));
            loop {}
        }
    }
}
