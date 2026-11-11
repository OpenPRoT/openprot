// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Permanently-runnable sibling process for the I3C interrupt-preemption test.
//!
//! Does nothing useful on purpose. Its only job is to stay in the run queue so
//! that the `receiver` process is *not* the current thread while it is blocked
//! in `object_wait`.
//!
//! That is what makes this test differ from `tests/i3c_user_irq`. When the I3C
//! interrupt fires, the scheduler wakes `receiver` while this process is
//! running, so it selects a thread other than the interrupted one. The
//! `current_thread_id == new_thread.id()` early-out in
//! `pw_kernel::scheduler::context_switch` does not fire, and the kernel reaches
//! `Arch::context_switch` from inside `trap_handler` -- the path under test.
//!
//! It must never block and never exit: blocking would empty the run queue and
//! restore the single-runnable-thread condition this test exists to avoid.

#![no_main]
#![no_std]

use userspace::process_entry;

#[process_entry("spinner")]
fn entry() {
    // A volatile counter, so the loop cannot be optimized away and this
    // process stays genuinely runnable rather than being reduced to a trap.
    let mut counter: u32 = 0;
    loop {
        // SAFETY: `counter` is a live local; the volatile access exists only to
        // keep the loop observable to the compiler.
        unsafe {
            core::ptr::write_volatile(&raw mut counter, counter.wrapping_add(1));
        }
    }
}
