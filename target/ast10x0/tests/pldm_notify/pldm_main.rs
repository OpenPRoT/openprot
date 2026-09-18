// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Initiator side of the PLDM notification channel test.
//!
//! Sends one `UpdateRequested` notification to the orchestrator process and
//! checks the answer, then reports the result: `debug_shutdown(Ok(()))` on a
//! clean round trip, `Err` otherwise. The kernel target turns that into the
//! UART sentinel.
//!
//! The firmware device is not here. Building `services/pldm` for ast10x0 is
//! separate work, and what this test covers is the channel.

#![no_main]
#![no_std]

use openprot_pldm_notify_api::{Request, Response, MAX_REQUEST_SIZE, MAX_RESPONSE_SIZE};
use pw_status::Error;
use userspace::time::{Clock, Duration, Instant, SystemClock};
use userspace::{entry, syscall};

use app_pldm::handle;

/// A wedged orchestrator must not block the firmware device, so the transact
/// is bounded rather than waiting forever.
const ANSWER_WINDOW: Duration = Duration::from_millis(1000);

#[entry]
fn entry() {
    match notify_update_requested() {
        Ok(()) => {
            pw_log::info!("pldm-notify round trip PASSED");
            let _ = syscall::debug_shutdown(Ok(()));
        }
        Err(()) => {
            let _ = syscall::debug_shutdown(Err(Error::Internal));
        }
    }
    #[expect(clippy::empty_loop)]
    loop {}
}

fn notify_update_requested() -> Result<(), ()> {
    let mut out = [0u8; MAX_REQUEST_SIZE];
    let len = Request::UpdateRequested.encode(&mut out).map_err(|_| {
        pw_log::error!("encoding the notification failed");
    })?;

    let deadline = SystemClock::now()
        .checked_add_duration(ANSWER_WINDOW)
        .unwrap_or(Instant::MAX);

    let mut back = [0u8; MAX_RESPONSE_SIZE];
    let answer_len =
        syscall::channel_transact(handle::PLDM_NOTIFY, &out[..len], &mut back, deadline).map_err(
            |_| {
                pw_log::error!("the notification did not reach the orchestrator");
            },
        )?;

    match Response::decode(&back[..answer_len]) {
        Ok(Response::Accepted) => Ok(()),
        Ok(Response::Rejected) => {
            pw_log::error!("the orchestrator rejected the request");
            Err(())
        }
        _ => {
            pw_log::error!("the orchestrator's answer did not decode");
            Err(())
        }
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
