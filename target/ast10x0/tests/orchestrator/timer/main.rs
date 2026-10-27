// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Timer QEMU test: proves the TimerManager <-> object_wait seam with the
//! production types (userspace Instant, orchestrator-sm ComponentId).
//!
//! The wait loop is the intended orchestrator runtime shape: block in
//! object_wait on {interrupt object, next timer deadline}; DeadlineExceeded
//! drains TimerManager::poll, Ok is the event path.

#![no_main]
#![no_std]

use app_test_timer::{constants, handle, signals};
use openprot_orchestrator_sm::ComponentId;
use openprot_orchestrator_timer::{Expired, TimerManager};
use pw_status::{Error, Result};
use userspace::time::{Clock, Duration, Instant, SystemClock};
use userspace::{entry, syscall};

const C0: ComponentId = ComponentId::new(0);
const C1: ComponentId = ComponentId::new(1);

/// Chain capacity 4 — matches the crate's own unit-test sizing.
type Tm = TimerManager<Instant, ComponentId, 4>;

/// Scenario 1: arm Boot(C0)@+50ms, Boot(C1)@+100ms, Commit@+100ms. Expect
/// expiries in exactly that order (Boot(C1) before Commit exercises the
/// boot-before-commit tie-break at the shared deadline), each at or after its
/// armed offset (lower bound only — no upper bound under QEMU), one-shot.
fn scenario_ordered_expiry() -> Result<()> {
    pw_log::info!("scenario 1: ordered expiry + tie-break + one-shot");
    let mut tm = Tm::new();
    let t0 = SystemClock::now();
    tm.arm_boot(C0, t0 + Duration::from_millis(50))
        .map_err(|_| Error::ResourceExhausted)?;
    tm.arm_boot(C1, t0 + Duration::from_millis(100))
        .map_err(|_| Error::ResourceExhausted)?;
    tm.arm_commit(t0 + Duration::from_millis(100));

    let expected: [(Expired<ComponentId>, Duration); 3] = [
        (Expired::Boot(C0), Duration::from_millis(50)),
        (Expired::Boot(C1), Duration::from_millis(100)),
        (Expired::Commit, Duration::from_millis(100)),
    ];

    let mut idx = 0;
    while idx < expected.len() {
        let deadline = tm.next_deadline().ok_or(Error::Internal)?;
        match syscall::object_wait(handle::TIMER_IRQ, signals::TEST_IRQ, deadline) {
            Err(Error::DeadlineExceeded) => {
                let now = SystemClock::now();
                while let Some(fired) = tm.poll(now) {
                    let (want, offset) = expected[idx];
                    if fired != want {
                        pw_log::error!("scenario 1: wrong expiry at index {}", idx as u32);
                        return Err(Error::Internal);
                    }
                    if now < t0 + offset {
                        pw_log::error!("scenario 1: expiry {} fired early", idx as u32);
                        return Err(Error::Internal);
                    }
                    idx += 1;
                }
            }
            Ok(_) => {
                pw_log::error!("scenario 1: unexpected event wakeup");
                return Err(Error::Internal);
            }
            Err(e) => return Err(e),
        }
    }

    if tm.poll(SystemClock::now()).is_some() {
        pw_log::error!("scenario 1: watchdog fired twice (not one-shot)");
        return Err(Error::Internal);
    }
    if tm.next_deadline().is_some() {
        pw_log::error!("scenario 1: deadline outstanding after full drain");
        return Err(Error::Internal);
    }
    pw_log::info!("scenario 1: PASS");
    Ok(())
}

/// Scenario 2: arm Boot(C0)@+30ms then immediately re-arm @+80ms. The 30 ms
/// deadline must be gone: the single wait must run to >= 80 ms and yield
/// exactly one Boot(C0). (If the re-arm stacked, next_deadline would be the
/// 30 ms entry and the elapsed-time check below would fail.)
fn scenario_rearm_replaces() -> Result<()> {
    pw_log::info!("scenario 2: re-arm replaces");
    let mut tm = Tm::new();
    let t0 = SystemClock::now();
    tm.arm_boot(C0, t0 + Duration::from_millis(30))
        .map_err(|_| Error::ResourceExhausted)?;
    tm.arm_boot(C0, t0 + Duration::from_millis(80))
        .map_err(|_| Error::ResourceExhausted)?;

    let deadline = tm.next_deadline().ok_or(Error::Internal)?;
    match syscall::object_wait(handle::TIMER_IRQ, signals::TEST_IRQ, deadline) {
        Err(Error::DeadlineExceeded) => {}
        Ok(_) => {
            pw_log::error!("scenario 2: unexpected event wakeup");
            return Err(Error::Internal);
        }
        Err(e) => return Err(e),
    }

    let now = SystemClock::now();
    if now < t0 + Duration::from_millis(80) {
        pw_log::error!("scenario 2: woke before the replaced 80ms deadline");
        return Err(Error::Internal);
    }
    match tm.poll(now) {
        Some(Expired::Boot(id)) if id == C0 => {}
        _ => {
            pw_log::error!("scenario 2: expected exactly Boot(C0)");
            return Err(Error::Internal);
        }
    }
    if tm.poll(SystemClock::now()).is_some() || tm.next_deadline().is_some() {
        pw_log::error!("scenario 2: stacked entry survived the re-arm");
        return Err(Error::Internal);
    }
    pw_log::info!("scenario 2: PASS");
    Ok(())
}

/// Scenario 3: arm Boot(C0)@+500ms, then fire the test IRQ. object_wait must
/// return Ok (event beat the deadline); the "component reported in" analogue
/// then cancels the watchdog and nothing is left armed. The trigger is issued
/// before the wait — interrupt objects latch the pending signal, so this does
/// not race.
fn scenario_cancel_via_event() -> Result<()> {
    pw_log::info!("scenario 3: cancel via event");
    let mut tm = Tm::new();
    let t0 = SystemClock::now();
    tm.arm_boot(C0, t0 + Duration::from_millis(500))
        .map_err(|_| Error::ResourceExhausted)?;

    syscall::debug_trigger_interrupt(constants::TEST_IRQ)?;

    let deadline = tm.next_deadline().ok_or(Error::Internal)?;
    let wait = match syscall::object_wait(handle::TIMER_IRQ, signals::TEST_IRQ, deadline) {
        Ok(wait) => wait,
        Err(Error::DeadlineExceeded) => {
            pw_log::error!("scenario 3: deadline fired although the event was pending");
            return Err(Error::Internal);
        }
        Err(e) => return Err(e),
    };
    if !wait.pending_signals.contains(signals::TEST_IRQ) {
        pw_log::error!("scenario 3: woke without TEST_IRQ pending");
        return Err(Error::Internal);
    }
    syscall::interrupt_ack(handle::TIMER_IRQ, signals::TEST_IRQ)?;

    if SystemClock::now() >= t0 + Duration::from_millis(500) {
        pw_log::error!("scenario 3: event did not beat the 500ms deadline");
        return Err(Error::Internal);
    }
    tm.cancel_boot(C0);
    if tm.next_deadline().is_some() {
        pw_log::error!("scenario 3: deadline still armed after cancel");
        return Err(Error::Internal);
    }
    if tm.poll(SystemClock::now()).is_some() {
        pw_log::error!("scenario 3: cancelled watchdog fired");
        return Err(Error::Internal);
    }
    pw_log::info!("scenario 3: PASS");
    Ok(())
}

fn run_test() -> Result<()> {
    scenario_ordered_expiry()?;
    scenario_rearm_replaces()?;
    scenario_cancel_via_event()?;
    Ok(())
}

#[entry]
fn entry() {
    match run_test() {
        Ok(()) => {
            pw_log::info!("timer test: all scenarios PASSED");
            let _ = syscall::debug_shutdown(Ok(()));
        }
        Err(e) => {
            pw_log::error!("timer test FAILED: {}", e as u32);
            let _ = syscall::debug_shutdown(Err(e));
        }
    }
    loop {}
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
