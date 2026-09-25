// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Orchestrator integration QEMU test: the pure core (`Orchestrator`,
//! `orchestrator-sm`), the server runtime (`BootWatchdogs`,
//! `orchestrator-server`), the device table (`DeviceConfig`,
//! `orchestrator-config`), and the checkpoint walker (`CheckpointWalk`,
//! `orchestrator-checkpoint-walk`) wired end to end under the kernel.
//!
//! Scenarios 1-3 exercise `CheckpointWalk` end to end: the walk judges
//! per-checkpoint windows via `poll(now_millis)` and an `EvidenceReader`,
//! the runtime converts `deadline_millis` to kernel ticks via `object_wait`,
//! and a `DeadlineExceeded` re-poll proves the timeout path. Scenario 3
//! drives a device-reported fault, not just silence, through that path.
//!
//! Scenarios 4-5 exercise `BootWatchdogs` multiplexing across a
//! multi-component chain (nearest-of-many deadlines, correct-id recovery).
//! Scenario 6 covers the commit watchdog.
//!
//! Scenarios 7-8 exercise recovery end to end under real kernel deadlines.
//! Scenario 7: a timeout triggers recovery, the platform restores the
//! image, the re-walk succeeds, and the component is released a second
//! time. Scenario 8: recovery reports source exhaustion immediately, and
//! the required component locks the platform down.

#![no_main]
#![no_std]

use app_test_runtime::{constants, handle, signals};
use openprot_orchestrator_server::BootWatchdogs;
use openprot_orchestrator_sm::{
    BootFailureKind, Chain, ComponentAttrs, ComponentId, Effect, EffectError, Event, Orchestrator,
    Platform, PowerOnResult, State,
};
use orchestrator_capabilities::{BootStatus, BootWatch, EvidenceReader, FailureCause, WalkVerdict};
use orchestrator_checkpoint_walk::CheckpointWalk;
use orchestrator_config::{assert_retry_reaches_every_image, BootCheckpoint, DeviceConfig};
use pw_status::{Error, Result};
use userspace::time::{Clock, Duration, Instant, SystemClock};
use userspace::{entry, syscall};

/// The components this test supervises.
const C0: ComponentId = ComponentId::new(0);
const C1: ComponentId = ComponentId::new(1);

/// Chain capacity and effect-sink cap for the core (`E >= 2*N + 2`).
const N: usize = 4;
const E: usize = 2 * N + 2;
const MAX_RETRY: u8 = 3;

const _: () = assert_retry_reaches_every_image(MAX_RETRY, &[SOC]);

/// Commit watchdog window. Not a boot window, so it stays a local constant
/// rather than coming from the device table.
const COMMIT_WINDOW: Duration = Duration::from_millis(50);

type Core = Orchestrator<N, E>;
type Watchdogs = BootWatchdogs<N>;

/// The device table: per-checkpoint windows, exactly as a board would declare
/// them. Two checkpoints so the inner walk exercises re-arm-on-progress
/// (`bl1` then `kernel`). Probe ids are progress thresholds: the reader
/// reports `Booted` once its internal level reaches the threshold.
const SOC: DeviceConfig<u8, u8> = DeviceConfig::new(
    "soc",
    0,
    &[
        BootCheckpoint::new("bl1", 1, core::time::Duration::from_millis(500)),
        BootCheckpoint::new("kernel", 2, core::time::Duration::from_millis(500)),
    ],
    // The boot walk never looks at images; no layout is legal.
    None,
);

/// The device table speaks `core::time::Duration`; the runtime speaks the
/// kernel's [`Duration`]. Converting is the platform driver's job.
fn window(timeout: core::time::Duration) -> Duration {
    Duration::from_millis(timeout.as_millis() as u64)
}

/// Progress-register reader for the walk: probe N is `Booted` once
/// `level >= N`. Mirrors the ProgressReader archetype in the evidence tests,
/// including its fault channel: a device that reports its own verdict reads
/// that verdict for every probe, whichever checkpoint is being awaited.
struct ProgressReader {
    level: u8,
    fault: Option<BootStatus>,
}

impl ProgressReader {
    const fn new() -> Self {
        Self {
            level: 0,
            fault: None,
        }
    }
}

impl EvidenceReader<u8> for ProgressReader {
    type Error = core::convert::Infallible;

    fn read(&mut self, probe: &u8) -> core::result::Result<BootStatus, Self::Error> {
        if let Some(fault) = self.fault {
            return Ok(fault);
        }
        Ok(if self.level >= *probe {
            BootStatus::Booted
        } else {
            BootStatus::Booting
        })
    }
}

/// What the simulated device does when given an opportunity to report.
#[derive(Clone, Copy)]
enum DeviceBehavior {
    /// Reports progress for the first `reached` opportunities, then goes
    /// quiet: `reached == len` boots, anything less times out.
    Progresses { reached: usize },
    /// Reports this fault on the very first opportunity, instead of
    /// progress. Exercises `CheckpointWalk`'s `FailedRetriable`/`FailedFatal`
    /// arms through the real interrupt/`object_wait` path, not just through
    /// `CheckpointWalk`'s own host unit tests.
    Faults(BootStatus),
}

fn ticks_to_millis(ticks: u64) -> u64 {
    ticks * 1000 / SystemClock::TICKS_PER_SEC
}

fn millis_to_ticks(millis: u64) -> u64 {
    millis * SystemClock::TICKS_PER_SEC / 1000
}

fn failure_kind(cause: FailureCause) -> BootFailureKind {
    match cause {
        FailureCause::TimedOut => BootFailureKind::TimedOut,
        FailureCause::DeviceRetriable => BootFailureKind::DeviceRetriable,
        FailureCause::DeviceFatal => BootFailureKind::DeviceFatal,
    }
}

/// A fake [`Platform`] for the run loop. It records the `ReleaseReset(id)`
/// effects that open each component's boot supervision and optionally
/// responds to `RecoverComponent` with restore outcomes. Every other effect
/// is accepted so the core can settle.
struct FakePlatform {
    released: heapless::Vec<ComponentId, N>,
    /// Number of recovery sources the fake device has. `None` means
    /// recovery always returns `Ok(None)` (scenarios that do not exercise
    /// recovery). `Some(n)` returns `Restored` for `attempt < n` and
    /// `RecoveryUnavailable` for `attempt >= n`.
    recovery_sources: Option<u8>,
    /// Attempts the SM sent, in order, for post-hoc assertions.
    recovery_attempts: heapless::Vec<u8, N>,
}

impl FakePlatform {
    const fn new() -> Self {
        Self {
            released: heapless::Vec::new(),
            recovery_sources: None,
            recovery_attempts: heapless::Vec::new(),
        }
    }

    fn with_recovery_sources(sources: u8) -> Self {
        Self {
            released: heapless::Vec::new(),
            recovery_sources: Some(sources),
            recovery_attempts: heapless::Vec::new(),
        }
    }

    fn was_released(&self, id: ComponentId) -> bool {
        self.released.contains(&id)
    }

    fn release_count(&self, id: ComponentId) -> usize {
        self.released.iter().filter(|&&c| c == id).count()
    }
}

impl Platform for FakePlatform {
    fn execute(&mut self, effect: Effect) -> core::result::Result<Option<Event>, EffectError> {
        match effect {
            Effect::ReleaseReset(id) => {
                let _ = self.released.push(id);
            }
            Effect::RecoverComponent { id, attempt } => {
                let _ = self.recovery_attempts.push(attempt);
                if let Some(sources) = self.recovery_sources {
                    return if attempt < sources {
                        Ok(Some(Event::Restored(id)))
                    } else {
                        Ok(Some(Event::RecoveryUnavailable(id)))
                    };
                }
            }
            _ => {}
        }
        Ok(None)
    }
}

/// A fresh core with the given components, each passive/required.
fn new_core(ids: &[ComponentId]) -> Result<Core> {
    let mut v = heapless::Vec::<(ComponentId, ComponentAttrs), N>::new();
    for id in ids {
        v.push((*id, ComponentAttrs::passive_required()))
            .map_err(|_| Error::ResourceExhausted)?;
    }
    let chain: Chain<N> = v.try_into().map_err(|_| Error::Internal)?;
    Ok(Orchestrator::new(chain, MAX_RETRY))
}

/// Power on, then pass verification for each component in chain order. Each
/// `VerificationPassed` releases its component (speculative release), so all
/// are released before any boots.
fn drive_releases(core: &mut Core, plat: &mut FakePlatform, ids: &[ComponentId]) -> Result<()> {
    core.dispatch(plat, Event::PowerGood(PowerOnResult::Provisioned));
    for id in ids {
        core.dispatch(plat, Event::VerificationPassed(*id));
        if !plat.was_released(*id) {
            return Err(Error::Internal);
        }
    }
    Ok(())
}

/// Run one component's inner checkpoint walk through `CheckpointWalk` and
/// return its terminal verdict as an event. `behavior` simulates the device:
/// it either advances its progress register or reports a fault.
///
/// The walk judges per-checkpoint windows; the runtime (`object_wait`) just
/// sleeps until the walk's deadline or a device signal.
fn walk_device(
    walk: &mut CheckpointWalk<ProgressReader, u8>,
    id: ComponentId,
    behavior: DeviceBehavior,
) -> Result<Event> {
    walk.arm();
    let mut k = 0usize;

    loop {
        let now = ticks_to_millis(SystemClock::now().ticks());
        match walk.poll(now) {
            WalkVerdict::Waiting { deadline_millis } => {
                // Simulate the device after the poll returned Waiting, so
                // every trigger pairs with a wait+ack below.
                let triggered = match behavior {
                    DeviceBehavior::Progresses { reached } if k < reached => {
                        walk.reader_mut().level = (k + 1) as u8;
                        true
                    }
                    DeviceBehavior::Faults(status) if k == 0 => {
                        walk.reader_mut().fault = Some(status);
                        true
                    }
                    _ => false,
                };
                if triggered {
                    syscall::debug_trigger_interrupt(constants::BOOT_PROGRESS)?;
                }

                let deadline = Instant::from_ticks(millis_to_ticks(deadline_millis));
                match syscall::object_wait(handle::BOOT_SIGNAL, signals::BOOT_PROGRESS, deadline) {
                    Ok(wait) => {
                        if !wait.pending_signals.contains(signals::BOOT_PROGRESS) {
                            return Err(Error::Internal);
                        }
                        syscall::interrupt_ack(handle::BOOT_SIGNAL, signals::BOOT_PROGRESS)?;
                        k += 1;
                    }
                    Err(Error::DeadlineExceeded) => {
                        // Deadline lapsed; re-poll and the walk will judge timeout.
                    }
                    Err(e) => return Err(e),
                }
            }
            WalkVerdict::Complete => return Ok(Event::Booted(id)),
            WalkVerdict::Failed { checkpoint, cause } => {
                return Ok(Event::BootFailed {
                    id,
                    checkpoint,
                    kind: failure_kind(cause),
                });
            }
        }
    }
}

/// Simulate `id`'s device reporting in: latch the progress signal, wait, ack,
/// and retire its watchdog through the runtime.
fn confirm(wd: &mut Watchdogs, id: ComponentId) -> Result<()> {
    syscall::debug_trigger_interrupt(constants::BOOT_PROGRESS)?;
    match syscall::object_wait(
        handle::BOOT_SIGNAL,
        signals::BOOT_PROGRESS,
        wd.wait_deadline(),
    ) {
        Ok(wait) => {
            if !wait.pending_signals.contains(signals::BOOT_PROGRESS) {
                return Err(Error::Internal);
            }
            syscall::interrupt_ack(handle::BOOT_SIGNAL, signals::BOOT_PROGRESS)?;
            wd.cancel_boot(id);
            Ok(())
        }
        Err(e) => Err(e),
    }
}

/// Inner walk, happy path: a single component passes every checkpoint (windows
/// from the device table, judged by `CheckpointWalk`), the walk yields
/// `Booted`, and the core stays `Ready`. A late `Timeout` is then a no-op.
fn scenario_checkpoint_confirmed() -> Result<()> {
    pw_log::info!("scenario 1: checkpoint walk confirmed");
    let mut core = new_core(&[C0])?;
    let mut plat = FakePlatform::new();

    drive_releases(&mut core, &mut plat, &[C0])?;
    if core.state() != State::Ready {
        pw_log::error!("scenario 1: single component did not reach Ready on release");
        return Err(Error::Internal);
    }

    let mut walk = CheckpointWalk::new(ProgressReader::new(), &SOC);
    let reached = SOC.checkpoints().len();
    let terminal = walk_device(&mut walk, C0, DeviceBehavior::Progresses { reached })?;
    if terminal != Event::Booted(C0) {
        pw_log::error!("scenario 1: walk did not confirm boot");
        return Err(Error::Internal);
    }

    core.dispatch(&mut plat, terminal);
    if core.state() != State::Ready {
        pw_log::error!("scenario 1: core left Ready after boot confirmed");
        return Err(Error::Internal);
    }

    // A stale timeout must not re-open recovery.
    core.dispatch(&mut plat, Event::Timeout(C0));
    if core.state() != State::Ready {
        pw_log::error!("scenario 1: stale timeout re-opened recovery");
        return Err(Error::Internal);
    }

    pw_log::info!("scenario 1: PASS");
    Ok(())
}

/// Inner walk, timeout path: the device never signals, the first checkpoint's
/// window lapses (judged by `CheckpointWalk`), and the walk surfaces
/// `BootFailed` with `TimedOut`. The core recovers from the walk's verdict.
fn scenario_checkpoint_timeout() -> Result<()> {
    pw_log::info!("scenario 2: checkpoint walk timeout drives recovery");
    let mut core = new_core(&[C0])?;
    let mut plat = FakePlatform::new();

    drive_releases(&mut core, &mut plat, &[C0])?;

    let mut walk = CheckpointWalk::new(ProgressReader::new(), &SOC);
    let terminal = walk_device(&mut walk, C0, DeviceBehavior::Progresses { reached: 0 })?;
    let is_boot_failed = matches!(
        terminal,
        Event::BootFailed { id, checkpoint: "bl1", kind, .. }
            if id == C0 && kind == BootFailureKind::TimedOut
    );
    if !is_boot_failed {
        pw_log::error!("scenario 2: walk did not produce BootFailed");
        return Err(Error::Internal);
    }

    core.dispatch(&mut plat, terminal);
    if core.state() != State::Recovering(C0) {
        pw_log::error!("scenario 2: core did not enter recovery");
        return Err(Error::Internal);
    }

    pw_log::info!("scenario 2: PASS");
    Ok(())
}

/// Inner walk, device-fault path: the device is up enough to report its own
/// verdict. The walk surfaces `BootFailed` with `DeviceFatal`, and — the point
/// of the scenario — it does so *early*, without burning the checkpoint's
/// window, which is what distinguishes a reported fault from silence.
fn scenario_checkpoint_device_fault() -> Result<()> {
    pw_log::info!("scenario 3: device-reported fault ends the wait early");
    let mut core = new_core(&[C0])?;
    let mut plat = FakePlatform::new();

    drive_releases(&mut core, &mut plat, &[C0])?;

    let mut walk = CheckpointWalk::new(ProgressReader::new(), &SOC);
    let start = ticks_to_millis(SystemClock::now().ticks());
    let terminal = walk_device(
        &mut walk,
        C0,
        DeviceBehavior::Faults(BootStatus::FailedFatal),
    )?;
    let elapsed = ticks_to_millis(SystemClock::now().ticks()).saturating_sub(start);

    let is_fatal = matches!(
        terminal,
        Event::BootFailed { id, checkpoint: "bl1", kind, .. }
            if id == C0 && kind == BootFailureKind::DeviceFatal
    );
    if !is_fatal {
        pw_log::error!("scenario 3: walk did not surface the device's fatal verdict");
        return Err(Error::Internal);
    }

    // The checkpoint's window is 500ms; a fault-ended wait should finish in a
    // small fraction of that. A silence-driven timeout (scenario 2) can only
    // finish once the full window lapses, so this margin is generous enough to
    // distinguish the two without being flaky.
    if elapsed >= 100 {
        pw_log::error!(
            "scenario 3: fault took {}ms, did not end the wait early",
            elapsed as u32
        );
        return Err(Error::Internal);
    }

    core.dispatch(&mut plat, terminal);
    if core.state() != State::Recovering(C0) {
        pw_log::error!("scenario 3: core did not enter recovery");
        return Err(Error::Internal);
    }

    pw_log::info!("scenario 3: PASS");
    Ok(())
}

/// Outer walk, all confirm: a two-component chain, both boot watchdogs armed at
/// once, both components report in, the core reaches `Ready`.
fn scenario_chain_all_confirm() -> Result<()> {
    pw_log::info!("scenario 4: multi-component chain all confirm");
    let mut core = new_core(&[C0, C1])?;
    let mut plat = FakePlatform::new();
    let mut wd = Watchdogs::new();

    drive_releases(&mut core, &mut plat, &[C0, C1])?;
    if core.state() != State::Ready {
        pw_log::error!("scenario 4: chain did not reach Ready on release");
        return Err(Error::Internal);
    }

    // Both released speculatively: arm both watchdogs before either reports.
    let boot = window(SOC.checkpoints()[0].timeout());
    wd.arm_boot(C0, boot)
        .map_err(|_| Error::ResourceExhausted)?;
    wd.arm_boot(C1, boot)
        .map_err(|_| Error::ResourceExhausted)?;

    confirm(&mut wd, C0)?;
    core.dispatch(&mut plat, Event::Booted(C0));
    confirm(&mut wd, C1)?;
    core.dispatch(&mut plat, Event::Booted(C1));

    if core.state() != State::Ready {
        pw_log::error!("scenario 4: core left Ready after both booted");
        return Err(Error::Internal);
    }

    pw_log::info!("scenario 4: PASS");
    Ok(())
}

/// Outer walk, one lapses: both watchdogs armed, `C0` reports in, `C1` goes
/// quiet. With only `C1` left, the runtime's nearest deadline is `C1`'s; it
/// lapses and `poll_expired` surfaces `Timeout(C1)`, recovering the right one.
fn scenario_chain_one_timeout() -> Result<()> {
    pw_log::info!("scenario 5: multi-component chain, one times out");
    let mut core = new_core(&[C0, C1])?;
    let mut plat = FakePlatform::new();
    let mut wd = Watchdogs::new();

    drive_releases(&mut core, &mut plat, &[C0, C1])?;

    let boot = window(SOC.checkpoints()[0].timeout());
    wd.arm_boot(C0, boot)
        .map_err(|_| Error::ResourceExhausted)?;
    wd.arm_boot(C1, boot)
        .map_err(|_| Error::ResourceExhausted)?;

    confirm(&mut wd, C0)?;
    core.dispatch(&mut plat, Event::Booted(C0));

    // Only C1 remains armed; wait for its window to lapse.
    match syscall::object_wait(
        handle::BOOT_SIGNAL,
        signals::BOOT_PROGRESS,
        wd.wait_deadline(),
    ) {
        Ok(_) => {
            pw_log::error!("scenario 5: unexpected signal, C1's device is quiet");
            return Err(Error::Internal);
        }
        Err(Error::DeadlineExceeded) => {
            let event = wd.poll_expired().ok_or(Error::Internal)?;
            if event != Event::Timeout(C1) {
                pw_log::error!("scenario 5: runtime timed out the wrong component");
                return Err(Error::Internal);
            }
            core.dispatch(&mut plat, event);
        }
        Err(e) => return Err(e),
    }

    if core.state() != State::Recovering(C1) {
        pw_log::error!("scenario 5: core did not recover C1");
        return Err(Error::Internal);
    }

    pw_log::info!("scenario 5: PASS");
    Ok(())
}

/// Commit path of the runtime binding: arm the commit watchdog, let it lapse
/// against the real clock, and confirm the runtime surfaces `CommitTimeout`
/// (and nothing more).
fn scenario_commit_timeout() -> Result<()> {
    pw_log::info!("scenario 6: commit watchdog surfaces CommitTimeout");
    let mut wd = Watchdogs::new();

    wd.arm_commit(COMMIT_WINDOW);
    match syscall::object_wait(
        handle::BOOT_SIGNAL,
        signals::BOOT_PROGRESS,
        wd.wait_deadline(),
    ) {
        Ok(_) => {
            pw_log::error!("scenario 6: unexpected signal, no device is armed");
            return Err(Error::Internal);
        }
        Err(Error::DeadlineExceeded) => {
            if wd.poll_expired() != Some(Event::CommitTimeout) {
                pw_log::error!("scenario 6: runtime did not surface CommitTimeout");
                return Err(Error::Internal);
            }
        }
        Err(e) => return Err(e),
    }

    // One-shot: the watchdog is drained, nothing more is due.
    if wd.poll_expired().is_some() {
        pw_log::error!("scenario 6: commit watchdog fired twice");
        return Err(Error::Internal);
    }

    pw_log::info!("scenario 6: PASS");
    Ok(())
}

/// Timeout → recovery → re-walk → confirmed. The point of this scenario is
/// that the whole recovery cycle, re-verification, second release, and
/// re-walked boot all run under real kernel deadlines, not just in the pure
/// SM unit tests. The sharp check is that C0 is released twice: once during
/// the initial walk and again after recovery.
fn scenario_recovery_succeeds() -> Result<()> {
    pw_log::info!("scenario 7: recovery succeeds, re-walk confirms boot");
    let mut core = new_core(&[C0])?;
    let mut plat = FakePlatform::with_recovery_sources(1);

    drive_releases(&mut core, &mut plat, &[C0])?;

    let mut walk = CheckpointWalk::new(ProgressReader::new(), &SOC);
    let terminal = walk_device(&mut walk, C0, DeviceBehavior::Progresses { reached: 0 })?;
    let is_timeout = matches!(
        terminal,
        Event::BootFailed { id, kind, .. } if id == C0 && kind == BootFailureKind::TimedOut
    );
    if !is_timeout {
        pw_log::error!("scenario 7: expected timeout");
        return Err(Error::Internal);
    }

    // Dispatch the timeout. The SM enters Recovering(C0), emits
    // RecoverComponent { attempt: 0 }, and FakePlatform returns
    // Restored(C0). The SM then re-enters PreSupervision, emits
    // ReadFirmware + VerifyFirmware. All in one dispatch cycle.
    core.dispatch(&mut plat, terminal);
    if core.state() != State::PreSupervision {
        pw_log::error!("scenario 7: expected PreSupervision after restore");
        return Err(Error::Internal);
    }

    if plat.recovery_attempts.as_slice() != &[0] {
        pw_log::error!("scenario 7: expected one attempt at index 0");
        return Err(Error::Internal);
    }

    // Re-verify: the SM already emitted VerifyFirmware(C0) during the
    // PreSupervision entry, so the platform is waiting for the verdict.
    core.dispatch(&mut plat, Event::VerificationPassed(C0));
    if plat.release_count(C0) != 2 {
        pw_log::error!(
            "scenario 7: expected two releases for C0, got {}",
            plat.release_count(C0) as u32
        );
        return Err(Error::Internal);
    }

    // Re-walk: the device boots this time.
    walk = CheckpointWalk::new(ProgressReader::new(), &SOC);
    let reached = SOC.checkpoints().len();
    let terminal = walk_device(&mut walk, C0, DeviceBehavior::Progresses { reached })?;
    if terminal != Event::Booted(C0) {
        pw_log::error!("scenario 7: re-walk did not confirm boot");
        return Err(Error::Internal);
    }
    core.dispatch(&mut plat, terminal);
    if core.state() != State::Ready {
        pw_log::error!("scenario 7: core did not reach Ready after re-walk");
        return Err(Error::Internal);
    }

    pw_log::info!("scenario 7: PASS");
    Ok(())
}

/// Recovery source exhausted immediately → required component locks down.
/// Uses `RecoveryUnavailable` (zero sources) so the path settles in one
/// dispatch cycle without burning MAX_RETRY × 500ms walk windows.
fn scenario_recovery_exhausted_locks() -> Result<()> {
    pw_log::info!("scenario 8: recovery exhausted, required component locks");
    let mut core = new_core(&[C0])?;
    let mut plat = FakePlatform::with_recovery_sources(0);

    drive_releases(&mut core, &mut plat, &[C0])?;

    let mut walk = CheckpointWalk::new(ProgressReader::new(), &SOC);
    let terminal = walk_device(&mut walk, C0, DeviceBehavior::Progresses { reached: 0 })?;
    let is_timeout = matches!(
        terminal,
        Event::BootFailed { id, kind, .. } if id == C0 && kind == BootFailureKind::TimedOut
    );
    if !is_timeout {
        pw_log::error!("scenario 8: expected timeout");
        return Err(Error::Internal);
    }

    // Dispatch the timeout. RecoverComponent { attempt: 0 } →
    // RecoveryUnavailable(C0) → exhaust_recovery → ReportRecoveryFailed →
    // RecoveryFailed → Locked. All in one dispatch cycle.
    core.dispatch(&mut plat, terminal);
    if core.state() != State::Locked {
        pw_log::error!("scenario 8: expected Locked after exhaustion");
        return Err(Error::Internal);
    }

    if plat.recovery_attempts.as_slice() != &[0] {
        pw_log::error!("scenario 8: expected one attempt at index 0");
        return Err(Error::Internal);
    }

    pw_log::info!("scenario 8: PASS");
    Ok(())
}

fn run_test() -> Result<()> {
    scenario_checkpoint_confirmed()?;
    scenario_checkpoint_timeout()?;
    scenario_checkpoint_device_fault()?;
    scenario_chain_all_confirm()?;
    scenario_chain_one_timeout()?;
    scenario_commit_timeout()?;
    scenario_recovery_succeeds()?;
    scenario_recovery_exhausted_locks()?;
    Ok(())
}

#[entry]
fn entry() {
    match run_test() {
        Ok(()) => {
            pw_log::info!("runtime integration test: all scenarios PASSED");
            let _ = syscall::debug_shutdown(Ok(()));
        }
        Err(e) => {
            pw_log::error!("runtime integration test FAILED: {}", e as u32);
            let _ = syscall::debug_shutdown(Err(e));
        }
    }
    loop {}
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
