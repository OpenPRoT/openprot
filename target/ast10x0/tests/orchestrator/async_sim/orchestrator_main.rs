// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! The orchestrator under test: the real `orchestrator-sm` core driven by an
//! asynchronous effect executor.
//!
//! Nothing here blocks on a service. An effect becomes a command queued for
//! one of three channels (verifier, reset, events); the loop starts it when
//! the channel is idle, then sleeps in one WaitGroup wait whose deadline is
//! the earliest of the boot watchdogs and the in-flight verify deadlines.
//! Whatever wakes it is decoded into an `Event` and fed back to the core.
//!
//! A channel carries one transaction at a time, so commands for the same
//! service queue behind each other. The events channel always has one
//! `WaitBoot` long-poll outstanding.
//!
//! The services decide how they behave (a device's boot script, how long a
//! verify takes), so nothing in this app tells them. One image runs one
//! scenario, chosen by a build flag, so each service's script is fixed when
//! the image is built and a failure names its scenario.
//!
//! | Flag           | Shows                                                      |
//! |----------------|------------------------------------------------------------|
//! | `flaky_boot`   | C0 does not boot on its first release and C1's verify is   |
//! |                | slow. C0's boot watchdog fires *while* C1 is still         |
//! |                | verifying, the core recovers C0, C1's late verdict is      |
//! |                | dropped as stale, and the re-walk ends in `Ready`.         |
//! | `verify_hangs` | C2's verify outlives the executor's deadline. The executor |
//! |                | cancels it and reports `VerificationFailed`; the           |
//! |                | verifier's late reply is dropped; the re-walk ends in      |
//! |                | `Ready`.                                                   |
//! | `never_boots`  | C0 never boots on any release. The platform has to give up |
//! |                | after its retry limit, report the failure and lock, not    |
//! |                | recover forever.                                           |

#![no_main]
#![no_std]

#[cfg(not(any(flaky_boot, verify_hangs, never_boots)))]
compile_error!("build one scenario: pass --cfg flaky_boot, verify_hangs or never_boots");
#[cfg(any(
    all(flaky_boot, verify_hangs),
    all(flaky_boot, never_boots),
    all(verify_hangs, never_boots)
))]
compile_error!("build one scenario, not several");

use app_orchestrator::handle;
use openprot_orchestrator_server::BootWatchdogs;
use openprot_orchestrator_sm::{
    Chain, ComponentAttrs, ComponentId, Effect, EffectError, Event, Orchestrator, PowerOnResult,
    State,
};
use pw_status::{Error, Result};
use userspace::entry;
use userspace::syscall::{self, Signals};
use userspace::time::{Clock, Duration, Instant, SystemClock};
use util_ipc::{AsyncChannelTransport, IpcHandle};
use util_requester::Command;

const N: usize = 3;
const E: usize = 2 * N + 2;
const MAX_RETRY: u8 = 3;

type Core = Orchestrator<N, E>;
type Transport = AsyncChannelTransport<IpcHandle>;

// Wire opcodes, matching verifier_main.rs and devices_main.rs.
const OP_VERIFY: u8 = 1;
const OP_RELEASE: u8 = 2;
const OP_ASSERT: u8 = 3;
const OP_RESTORE: u8 = 4;
const OP_WAIT: u8 = 5;

/// A component that has not reported in this long after release is failed.
///
/// The scenarios race four durations, and each gap has to stay wide on QEMU's
/// wall clock. In order: a healthy device's boot `B` (devices_main.rs, 20 ms),
/// this window `W`, the slow verify `V` that `flaky_boot` arranges
/// (verifier_main.rs, 210 ms), and the verify deadline `D` below (300 ms).
///
/// - `W - B` = 90 ms: a healthy device must report in before its watchdog.
/// - `V - W` = 100 ms: the flaky device's watchdog must fire *while* the slow
///   verify is still running, so the timeout is seen before that verdict.
/// - `D - V` = 90 ms: the slow verify must still finish inside its deadline.
///
/// Move one and the others move with it. `verify_hangs` has its own pair of
/// margins against `D`, described in verifier_main.rs.
const BOOT_WINDOW: Duration = Duration::from_millis(110);
/// A verify not answered this long after it started is abandoned.
///
/// Tied to the `verify_hangs` latency in verifier_main.rs, which must sit
/// between this and twice this (see the margins there). Both are scaled
/// together, not just widened.
const VERIFY_DEADLINE: Duration = Duration::from_millis(300);
/// A whole scenario that takes longer than this has hung.
const SCENARIO_LIMIT: Duration = Duration::from_millis(4000);

const VERIFIER: usize = 0;
const RESET: usize = 1;
const EVENTS: usize = 2;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Cmd {
    Verify { id: u8 },
    Release { id: u8 },
    Assert { id: u8 },
    Restore { id: u8 },
    WaitBoot,
}

impl Cmd {
    /// A verify that has not answered this long after it started is
    /// abandoned. Nothing else has a deadline.
    fn deadline(&self) -> Option<Instant> {
        match self {
            Cmd::Verify { .. } => Some(
                SystemClock::now()
                    .checked_add_duration(VERIFY_DEADLINE)
                    .unwrap_or(Instant::MAX),
            ),
            _ => None,
        }
    }
}

impl Command for Cmd {
    fn encode(&self, out: &mut [u8]) -> Option<usize> {
        let (bytes, len) = match *self {
            Cmd::Verify { id } => ([OP_VERIFY, id, 0], 2),
            Cmd::Release { id } => ([OP_RELEASE, id, 0], 2),
            Cmd::Assert { id } => ([OP_ASSERT, id, 0], 2),
            Cmd::Restore { id } => ([OP_RESTORE, id, 0], 2),
            Cmd::WaitBoot => ([OP_WAIT, 0, 0], 1),
        };
        out.get_mut(..len)?.copy_from_slice(bytes.get(..len)?);
        Some(len)
    }
}

/// A `util_requester` lane plus the kernel half of it: the transport's handle
/// is in the WaitGroup exactly while a command is in flight.
struct Lane {
    index: usize,
    inner: util_requester::Lane<Transport, Cmd, Instant, 8>,
}

impl Lane {
    fn new(index: usize, tr: Transport) -> Self {
        Self {
            index,
            inner: util_requester::Lane::new(tr),
        }
    }

    fn unwatch(&self) -> Result<()> {
        syscall::wait_group_remove(handle::WG, self.inner.transport().as_raw())
    }

    /// Start the next queued command if the channel is free.
    fn start_next(&mut self) -> Result<()> {
        match self.inner.start_next(|cmd| cmd.deadline()) {
            Ok(Some(_)) => syscall::wait_group_add(
                handle::WG,
                self.inner.transport().as_raw(),
                Signals::READABLE,
                self.index,
            ),
            Ok(None) => Ok(()),
            Err(_) => Err(Error::Internal),
        }
    }

    /// Collect the response if it has landed.
    fn finish(&mut self) -> Result<Option<(Cmd, [u8; 2])>> {
        let mut resp = [0u8; 4];
        match self.inner.poll(&mut resp) {
            Ok(None) => Ok(None),
            Ok(Some(done)) => {
                self.unwatch()?;
                match (done.len >= 2, resp.first(), resp.get(1)) {
                    (true, Some(status), Some(id)) => Ok(Some((done.cmd, [*status, *id]))),
                    _ => Err(Error::Internal),
                }
            }
            Err(_) => {
                let _ = self.unwatch();
                Err(Error::Internal)
            }
        }
    }

    /// Abandon the command in flight and return it.
    fn cancel(&mut self) -> Result<Option<Cmd>> {
        let cancelled = self.inner.cancel();
        let unwatched = self.unwatch();
        let cmd = cancelled.map_err(|_| Error::Internal)?;
        if cmd.is_some() {
            unwatched?;
        }
        Ok(cmd)
    }
}

/// What the core was told, in the order the executor delivered it.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Seen {
    Timeout(u8),
    Passed(u8),
    Failed(u8),
    Booted(u8),
    Restored(u8),
}

/// What a scenario run is waiting for.
#[derive(Clone, Copy)]
enum Goal {
    /// Every component verified, released and booted, the core in `Ready`.
    #[cfg_attr(never_boots, allow(dead_code))]
    Ready,
    /// The core latched `Locked`. Gives up, as a failure, if the core has
    /// recovered more than `max_recoveries` times and is still running.
    #[cfg_attr(not(never_boots), allow(dead_code))]
    Locked { max_recoveries: u32 },
}

struct Exec {
    lanes: [Lane; 3],
    wd: BootWatchdogs<N>,
    booted: [bool; N],
    seen: heapless::Vec<Seen, 48>,
    verify_cancels: u32,
    /// Components restored so far: one per `Restored` delivered to the core.
    recoveries: u32,
    /// Whether the core reported that recovery was exhausted. Only
    /// `never_boots` reads it.
    #[cfg_attr(not(never_boots), allow(dead_code))]
    recovery_failed: bool,
}

impl Exec {
    fn begin(&mut self) {
        self.wd = BootWatchdogs::new();
        self.booted = [false; N];
        self.seen.clear();
        self.verify_cancels = 0;
        self.recoveries = 0;
        self.recovery_failed = false;
    }

    fn push(&mut self, lane: usize, cmd: Cmd) -> core::result::Result<(), EffectError> {
        self.lanes
            .get_mut(lane)
            .ok_or(EffectError)?
            .inner
            .push(cmd)
            .map_err(|_| EffectError)
    }

    fn mark(&mut self, id: u8, booted: bool) {
        if let Some(b) = self.booted.get_mut(usize::from(id)) {
            *b = booted;
        }
    }

    /// Turn one effect into a command. Never blocks and never calls a
    /// service: the answer comes back later as an event.
    fn effect(&mut self, effect: Effect) -> core::result::Result<Option<Event>, EffectError> {
        match effect {
            Effect::VerifyFirmware(id) => {
                self.push(VERIFIER, Cmd::Verify { id: id.get() })?;
            }
            Effect::ReleaseReset(id) => {
                let id = id.get();
                self.mark(id, false);
                self.push(RESET, Cmd::Release { id })?;
                self.wd
                    .arm_boot(ComponentId::new(id), BOOT_WINDOW)
                    .map_err(|_| EffectError)?;
            }
            Effect::AssertReset(id) => {
                let id = id.get();
                self.mark(id, false);
                self.wd.cancel_boot(ComponentId::new(id));
                self.push(RESET, Cmd::Assert { id })?;
            }
            Effect::RecoverComponent { id, .. } => {
                let id = id.get();
                self.mark(id, false);
                self.wd.cancel_boot(ComponentId::new(id));
                // The machine expects the component held in reset while it is
                // restored. A device that is still running (or hung) with its
                // line released ignores a second release, so hold it first.
                self.push(RESET, Cmd::Assert { id })?;
                self.push(RESET, Cmd::Restore { id })?;
            }
            // Local, nothing to ask a service for.
            Effect::ReadFirmware(_) | Effect::CommitSvnFloor(_) => {}
            Effect::ReportRecoveryFailed(_) => self.recovery_failed = true,
            Effect::ReportIsolated(_)
            | Effect::ReportBootFailed { .. }
            | Effect::ReportUpdateDeferred
            | Effect::ReportUpdateAborted => {}
            // Not exercised by this simulation.
            Effect::SignAttestation
            | Effect::AuthenticateUpdate
            | Effect::StageUpdate
            | Effect::ActivateUpdate
            | Effect::DiscardStaged
            | Effect::LatchLockdown => {}
            Effect::Emit(_) => return Err(EffectError),
        }
        Ok(None)
    }

    fn record(&mut self, event: Event) {
        let seen = match event {
            Event::Timeout(id) => {
                pw_log::info!("EV timeout {}", id.get() as u32);
                Seen::Timeout(id.get())
            }
            Event::VerificationPassed(id) => {
                pw_log::info!("EV passed {}", id.get() as u32);
                Seen::Passed(id.get())
            }
            Event::VerificationFailed(id) => {
                pw_log::info!("EV failed {}", id.get() as u32);
                Seen::Failed(id.get())
            }
            Event::Booted(id) => {
                pw_log::info!("EV booted {}", id.get() as u32);
                Seen::Booted(id.get())
            }
            Event::Restored(id) => {
                self.recoveries = self.recoveries.saturating_add(1);
                pw_log::info!("EV restored {}", id.get() as u32);
                Seen::Restored(id.get())
            }
            _ => return,
        };
        let _ = self.seen.push(seen);
    }

    /// Start whatever can start. The events lane always keeps a long-poll
    /// queued.
    fn pump(&mut self) -> Result<()> {
        if let Some(events) = self.lanes.get_mut(EVENTS)
            && events.inner.is_idle()
        {
            let _ = events.inner.push(Cmd::WaitBoot);
        }
        for lane in self.lanes.iter_mut() {
            lane.start_next()?;
        }
        Ok(())
    }

    /// Decode a lane's response into an event for the core, if it has one.
    fn service(&mut self, lane: usize) -> Result<Option<Event>> {
        let Some(l) = self.lanes.get_mut(lane) else {
            return Ok(None);
        };
        let Some((cmd, resp)) = l.finish()? else {
            return Ok(None);
        };
        let [status, reported] = resp;
        let ok = status == 0;
        Ok(match cmd {
            Cmd::Verify { id, .. } if ok => Some(Event::VerificationPassed(ComponentId::new(id))),
            Cmd::Verify { id, .. } => Some(Event::VerificationFailed(ComponentId::new(id))),
            Cmd::Restore { id } if ok => Some(Event::Restored(ComponentId::new(id))),
            Cmd::Release { .. } | Cmd::Assert { .. } | Cmd::Restore { .. } if !ok => {
                return Err(Error::Internal);
            }
            Cmd::Release { .. } | Cmd::Assert { .. } => None,
            Cmd::WaitBoot if ok => {
                let id = reported;
                self.mark(id, true);
                self.wd.cancel_boot(ComponentId::new(id));
                Some(Event::Booted(ComponentId::new(id)))
            }
            Cmd::WaitBoot => None,
            Cmd::Restore { .. } => None,
        })
    }

    /// Give up on a verify that has run past its deadline. The unknown
    /// outcome counts as a failed verification.
    fn expire_verify(&mut self) -> Result<Option<Event>> {
        let now = SystemClock::now();
        let Some(lane) = self.lanes.get_mut(VERIFIER) else {
            return Ok(None);
        };
        let Some(&cmd) = lane.inner.in_flight() else {
            return Ok(None);
        };
        if !lane.inner.expired(&now) {
            return Ok(None);
        }
        let _ = lane.cancel()?;
        self.verify_cancels = self.verify_cancels.saturating_add(1);
        Ok(match cmd {
            Cmd::Verify { id, .. } => Some(Event::VerificationFailed(ComponentId::new(id))),
            _ => None,
        })
    }

    /// The earliest moment the loop must wake without a response.
    fn next_deadline(&self) -> Instant {
        let mut at = self.wd.wait_deadline();
        if let Some(due) = self.lanes.get(VERIFIER).and_then(|l| l.inner.deadline())
            && due < at
        {
            at = due;
        }
        at
    }

    /// Nothing but the long-poll is outstanding.
    fn quiescent(&self) -> bool {
        self.lanes
            .iter()
            .take(EVENTS)
            .all(|lane| lane.inner.is_idle())
    }

    fn all_booted(&self) -> bool {
        self.booted.iter().all(|b| *b)
    }

    // Only the scenarios that check event order use it.
    #[cfg_attr(never_boots, allow(dead_code))]
    fn first(&self, what: Seen) -> Option<usize> {
        self.seen.iter().position(|s| *s == what)
    }
}

fn deliver(core: &mut Core, exec: &mut Exec, event: Event) {
    exec.record(event);
    core.dispatch_with(event, |effect| exec.effect(effect));
}

fn new_core() -> Result<Core> {
    let mut entries = heapless::Vec::<(ComponentId, ComponentAttrs), N>::new();
    for id in 0..N as u8 {
        entries
            .push((ComponentId::new(id), ComponentAttrs::passive_required()))
            .map_err(|_| Error::ResourceExhausted)?;
    }
    let chain: Chain<N> = entries.try_into().map_err(|_| Error::Internal)?;
    Ok(Orchestrator::new(chain, MAX_RETRY))
}

/// Run one scenario to `Ready` with every device booted.
fn run_scenario(exec: &mut Exec, goal: Goal) -> Result<Core> {
    exec.begin();
    let mut core = new_core()?;
    let start = SystemClock::now();
    let limit = start
        .checked_add_duration(SCENARIO_LIMIT)
        .unwrap_or(Instant::MAX);

    deliver(
        &mut core,
        exec,
        Event::PowerGood(PowerOnResult::Provisioned),
    );

    loop {
        exec.pump()?;

        match goal {
            Goal::Ready => {
                if core.state() == State::Locked {
                    pw_log::error!("scenario: core latched Locked");
                    return Err(Error::Internal);
                }
                if core.state() == State::Ready && exec.quiescent() && exec.all_booted() {
                    return Ok(core);
                }
            }
            Goal::Locked { max_recoveries } => {
                if core.state() == State::Locked {
                    return Ok(core);
                }
                if exec.recoveries > max_recoveries {
                    pw_log::error!(
                        "scenario: still not locked after {} recoveries",
                        exec.recoveries as u32
                    );
                    return Err(Error::Internal);
                }
            }
        }
        if SystemClock::now() >= limit {
            pw_log::error!("scenario: timed out");
            return Err(Error::DeadlineExceeded);
        }

        let deadline = if exec.next_deadline() < limit {
            exec.next_deadline()
        } else {
            limit
        };
        match syscall::object_wait(handle::WG, Signals::READABLE, deadline) {
            Ok(wake) => {
                if let Some(event) = exec.service(wake.user_data)? {
                    deliver(&mut core, exec, event);
                }
            }
            Err(Error::DeadlineExceeded) => {}
            Err(e) => return Err(e),
        }
        while let Some(event) = exec.wd.poll_expired() {
            deliver(&mut core, exec, event);
        }
        if let Some(event) = exec.expire_verify()? {
            deliver(&mut core, exec, event);
        }
    }
}

/// C0 never boots the first time and C1 verifies slowly. C0's watchdog has
/// to fire during C1's verify, so the core is told about the timeout before
/// C1's verdict, which then arrives for a component the re-walk has moved
/// past.
#[cfg(flaky_boot)]
fn scenario(exec: &mut Exec) -> Result<()> {
    pw_log::info!("scenario flaky_boot: flaky boot beside a slow verify");
    run_scenario(exec, Goal::Ready)?;

    let timeout = exec.first(Seen::Timeout(0));
    let late_verdict = exec.first(Seen::Passed(1));
    match (timeout, late_verdict) {
        (Some(t), Some(v)) if t < v => {}
        _ => {
            pw_log::error!("flaky_boot: C0 timeout did not precede C1's verdict");
            return Err(Error::Internal);
        }
    }
    if exec.first(Seen::Restored(0)).is_none() {
        pw_log::error!("flaky_boot: C0 was never restored");
        return Err(Error::Internal);
    }
    if exec.verify_cancels != 0 {
        pw_log::error!("flaky_boot: a verify was abandoned unexpectedly");
        return Err(Error::Internal);
    }
    Ok(())
}

/// A verify outlives its deadline: abandon it, fail the component, recover,
/// and finish.
#[cfg(verify_hangs)]
fn scenario(exec: &mut Exec) -> Result<()> {
    pw_log::info!("scenario verify_hangs: verify outlives its deadline");
    run_scenario(exec, Goal::Ready)?;

    if exec.verify_cancels != 1 {
        pw_log::error!("verify_hangs: expected exactly one abandoned verify");
        return Err(Error::Internal);
    }
    if exec.first(Seen::Failed(2)).is_none() || exec.first(Seen::Restored(2)).is_none() {
        pw_log::error!("verify_hangs: C2 was not failed and restored");
        return Err(Error::Internal);
    }
    Ok(())
}

/// C0 never boots, on any release. Every attempt verifies, releases and times
/// out, so the machine recovers and re-walks again and again. It has to stop:
/// give up after its retry limit, report that recovery is exhausted, and lock.
/// The run allows a few attempts past the limit, so a machine that never stops
/// fails here instead of running out the clock.
#[cfg(never_boots)]
fn scenario(exec: &mut Exec) -> Result<()> {
    pw_log::info!("scenario never_boots: a device that never boots");
    let limit = u32::from(MAX_RETRY);
    run_scenario(
        exec,
        Goal::Locked {
            max_recoveries: limit.saturating_add(3),
        },
    )?;

    if exec.recoveries != limit {
        pw_log::error!(
            "never_boots: {} recoveries before locking, expected {}",
            exec.recoveries as u32,
            limit as u32
        );
        return Err(Error::Internal);
    }
    if !exec.recovery_failed {
        pw_log::error!("never_boots: locked without reporting recovery failed");
        return Err(Error::Internal);
    }
    Ok(())
}

/// A send and a receive buffer for each of the three lanes.
static mut BUFS: [[[u8; 8]; 2]; 3] = [[[0; 8]; 2]; 3];

/// # Safety
/// Called once. Hands out each buffer exactly once.
unsafe fn lanes() -> [Lane; 3] {
    // Safety: see function doc; `BUFS` is not touched anywhere else.
    let bufs: &'static mut [[[u8; 8]; 2]; 3] = unsafe { &mut *core::ptr::addr_of_mut!(BUFS) };
    let [verifier, reset, events] = bufs;
    let make = |index: usize, handle: u32, [send, recv]: &'static mut [[u8; 8]; 2]| {
        Lane::new(
            index,
            AsyncChannelTransport::new(IpcHandle::new(handle), send, recv),
        )
    };
    [
        make(VERIFIER, handle::VERIFIER, verifier),
        make(RESET, handle::RESET, reset),
        make(EVENTS, handle::EVENTS, events),
    ]
}

fn run_all() -> Result<()> {
    // Safety: called once.
    let mut exec = Exec {
        lanes: unsafe { lanes() },
        wd: BootWatchdogs::new(),
        booted: [false; N],
        seen: heapless::Vec::new(),
        verify_cancels: 0,
        recoveries: 0,
        recovery_failed: false,
    };
    // One run per image: the services' scripts count from the start of the
    // image. Repeat the run with `--runs_per_test`, not a loop.
    scenario(&mut exec)
}

#[entry]
fn entry() {
    let ret = run_all();
    match &ret {
        Ok(()) => pw_log::info!("scenario PASSED"),
        Err(e) => pw_log::error!("FAILED: status code {}", *e as u32),
    }
    let _ = syscall::debug_shutdown(ret);
    loop {}
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    let _ = userspace::syscall::debug_shutdown(Err(pw_status::Error::Internal));
    loop {}
}
