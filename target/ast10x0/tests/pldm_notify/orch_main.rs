// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Handler side of the PLDM notification channel test.
//!
//! Waits for a notification, runs the real `dispatch`, and answers. A
//! notification that does not decode to `Notification::UpdateRequested` fails
//! the test here; the pldm process reports the pass once its round trip
//! completes.
//!
//! One object to wait on, so it waits on it directly. The wait group arrives
//! with the run loop, when the boot signal and the intake seam are also
//! members.

#![no_main]
#![no_std]

use openprot_orchestrator_pldm_adapter::{dispatch, Notification};
use openprot_pldm_notify_api::{MAX_REQUEST_SIZE, MAX_RESPONSE_SIZE};
use pw_status::Error;
use userspace::syscall::Signals;
use userspace::time::Instant;
use userspace::{entry, syscall};

use app_orchestrator::handle;

#[entry]
fn entry() {
    let mut req = [0u8; MAX_REQUEST_SIZE];
    let mut resp = [0u8; MAX_RESPONSE_SIZE];

    loop {
        // Nothing else wakes this thread, so a failed wait cannot be retried
        // into a working one: report it rather than spinning.
        if syscall::object_wait(handle::PLDM_NOTIFY, Signals::READABLE, Instant::MAX).is_err() {
            pw_log::error!("waiting on the pldm-notify channel failed");
            let _ = syscall::debug_shutdown(Err(Error::Internal));
        }

        let len = match syscall::channel_read(handle::PLDM_NOTIFY, 0, &mut req) {
            Ok(n) => n,
            Err(_) => continue,
        };

        // PoC: always accept. Veto logic comes with the run loop.
        let (answer_len, notification) = dispatch(&req[..len], &mut resp, true);
        if notification != Some(Notification::UpdateRequested) {
            pw_log::error!("the notification did not decode to UpdateRequested");
            let _ = syscall::debug_shutdown(Err(Error::Internal));
        }

        if answer_len == 0 {
            pw_log::error!("dispatch returned no answer");
            let _ = syscall::debug_shutdown(Err(Error::Internal));
        }

        // Answer before acting on the notification: channel_respond has to be
        // prompt, and whatever the run loop does with it can take real time.
        let _ = syscall::channel_respond(handle::PLDM_NOTIFY, &resp[..answer_len]);
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
