// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Boot-progress confirmation: walk one device's configured checkpoints and
//! judge its boot good or failed.
//!
//! After the orchestrator releases a device from reset
//! ([`BootControl::release`](fwmanager_api::BootControl::release)), the
//! device's boot is confirmed by observing each of its configured
//! [`BootCheckpoint`]s within that checkpoint's window. Windows are
//! sequential: checkpoint *n*'s window opens the moment checkpoint *n − 1*
//! is observed (the first opens at release). A window that expires — or a
//! device-reported failure — fails the boot, naming the checkpoint that
//! died; that failure is what triggers recovery.

use core::time::Duration;

use fwmanager_api::config::{BootCheckpoint, BootSignal, DeviceConfig};
use fwmanager_api::{BootMonitor, BootStatus};

/// How many consecutive monitor read errors [`BootWalk::poll`] tolerates
/// before it fails the walk: the read is retried on the next two polls, the
/// third consecutive error is terminal. Any successful read resets the
/// count. Transient faults on a transport that is itself still coming up
/// should not condemn a boot — but a channel that stays broken cannot
/// confirm one either. A module constant until a real transport shows a
/// per-checkpoint budget is needed; the window deadline applies unchanged
/// either way (read errors never extend it).
const MAX_MONITOR_READ_RETRIES: u8 = 2;

/// Resolves the monitor observing one boot-progress signal.
///
/// Owned by the board (or test) wiring: the walk never learns which backend
/// serves which signal. `Monitor` is a single type — a board with
/// heterogeneous backends wraps them in its own closed monitor enum, and
/// one physical transport may back several signal kinds by handing out one
/// thin monitor view per signal (an MCTP stack serving
/// [`BootSignal::MctpReady`] and [`BootSignal::Heartbeat`]).
///
/// Returns `None` when the wiring maps no backend to `signal` — a board
/// wiring bug the walk surfaces as a named failure rather than a hang,
/// since the device-table schema cannot validate wiring it never sees.
pub trait MonitorMap<G> {
    /// The wiring's monitor type (typically its closed monitor enum).
    type Monitor: BootMonitor;

    /// Returns the monitor watching `signal`, or `None` if the wiring maps
    /// no backend to it.
    fn monitor_for(&self, signal: &BootSignal<G>) -> Option<&Self::Monitor>;
}

/// A checkpoint window in the walk's time unit. `Duration::as_millis` is
/// exact for the schema's seconds-scale windows; a sub-millisecond window
/// rounds *up* to one millisecond so the schema's non-zero-window invariant
/// survives the unit change, and absurd windows saturate instead of
/// truncating.
const fn window_millis(window: Duration) -> u64 {
    let ms = window.as_millis();
    if ms == 0 {
        1
    } else if ms > u64::MAX as u128 {
        u64::MAX
    } else {
        ms as u64
    }
}

/// Progress of a walk that has not failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Progress {
    /// Still waiting on `checkpoint`; its window expires at
    /// `deadline_millis`. The polling loop must observe the signal again no
    /// later than that — it may poll sooner for a tighter verdict.
    Waiting {
        /// The awaited checkpoint's name.
        checkpoint: &'static str,
        /// When its window expires, in the caller's monotonic milliseconds.
        deadline_millis: u64,
    },
    /// Every checkpoint was observed within its window: the boot is good.
    Complete,
}

/// Why a device's boot walk failed.
///
/// Every variant names the device and the checkpoint that died — that is
/// what [`BootCheckpoint`]'s `name` field exists for. A failure is terminal
/// for the walk; recovery is the caller's decision, made on this value.
#[derive(Debug)]
pub enum BootFailure<E> {
    /// The checkpoint's window expired without its signal — the walk's own
    /// judgment; hung devices report nothing.
    WindowExpired {
        /// The device whose boot failed.
        device: &'static str,
        /// The checkpoint whose window expired.
        checkpoint: &'static str,
    },
    /// The device itself reported a boot failure
    /// ([`BootStatus::Failed`]).
    DeviceReported {
        /// The device whose boot failed.
        device: &'static str,
        /// The checkpoint being awaited when the device reported failure.
        checkpoint: &'static str,
    },
    /// The checkpoint's monitor stayed unreadable past the retry budget
    /// ([`MAX_MONITOR_READ_RETRIES`]) — the evidence channel itself broke,
    /// which is not the same fault as a silent device. The concrete monitor
    /// error is the [`source()`](core::error::Error::source).
    MonitorRead {
        /// The device whose boot walk was cut short.
        device: &'static str,
        /// The checkpoint whose monitor could not be read.
        checkpoint: &'static str,
        /// The monitor's own error.
        source: E,
    },
    /// The board wiring maps no monitor to this checkpoint's signal — a
    /// wiring bug, surfaced instead of waiting on evidence that can never
    /// arrive.
    UnmappedSignal {
        /// The device whose boot walk was cut short.
        device: &'static str,
        /// The checkpoint whose signal has no monitor.
        checkpoint: &'static str,
    },
}

impl<E> core::fmt::Display for BootFailure<E> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::WindowExpired { device, checkpoint } => {
                write!(f, "{device}: checkpoint '{checkpoint}' window expired")
            }
            Self::DeviceReported { device, checkpoint } => {
                write!(
                    f,
                    "{device}: device reported boot failure at checkpoint '{checkpoint}'"
                )
            }
            Self::MonitorRead { device, checkpoint, .. } => {
                write!(
                    f,
                    "{device}: monitor read failed at checkpoint '{checkpoint}'"
                )
            }
            Self::UnmappedSignal { device, checkpoint } => {
                write!(
                    f,
                    "{device}: no monitor wired for checkpoint '{checkpoint}'"
                )
            }
        }
    }
}

impl<E: core::error::Error + 'static> core::error::Error for BootFailure<E> {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::MonitorRead { source, .. } => Some(source),
            _ => None,
        }
    }
}

/// One device's boot-progress walk.
///
/// Plain data: it holds which checkpoint is awaited and when that
/// checkpoint's window expires. It never sleeps and never reads a clock —
/// callers inject `now_millis` from their monotonic time source, so the
/// walk's every verdict is reproducible on the host with literal
/// timestamps.
///
/// A device with no checkpoints (a passive device, released blind under
/// [`CommitPolicy::None`](fwmanager_api::config::CommitPolicy::None)) is
/// complete from construction: there is no evidence to wait for.
///
/// # How the orchestrator uses it
///
/// Declaration order in the device table is the boot order, one device at a
/// time; releasing each device is sequenced by the flow that owns its
/// [`BootControl`](fwmanager_api::BootControl):
///
/// ```ignore
/// for dev in MANAGED_DEVICES {
///     control_for(dev).release()?;          // run the just-verified image
///     let mut walk = BootWalk::for_device(dev, now_millis());
///     loop {
///         match walk.poll(&monitors, now_millis())? {
///             Progress::Waiting { deadline_millis, .. } => {
///                 // Sleep until the next poll, never past the deadline.
///                 sleep_until_millis(deadline_millis.min(now_millis() + POLL_PERIOD));
///             }
///             Progress::Complete => break,  // boot confirmed: next device
///         }
///     }
/// }
/// ```
pub struct BootWalk<G: 'static> {
    device: &'static str,
    checkpoints: &'static [BootCheckpoint<G>],
    /// Index of the checkpoint currently awaited; `== checkpoints.len()`
    /// when the walk is complete.
    next: usize,
    /// When the awaited checkpoint's window expires, in the caller's
    /// monotonic milliseconds. Meaningless once the walk is complete.
    deadline_millis: u64,
    /// Consecutive monitor read errors on the awaited checkpoint; reset by
    /// any successful read.
    read_errors: u8,
}

impl<G: 'static> BootWalk<G> {
    /// Starts the walk at `now_millis` — the moment the device left reset.
    /// The first checkpoint's window opens here.
    pub fn new(
        device: &'static str,
        checkpoints: &'static [BootCheckpoint<G>],
        now_millis: u64,
    ) -> Self {
        let deadline_millis = match checkpoints.first() {
            Some(first) => now_millis.saturating_add(window_millis(first.window)),
            None => 0,
        };
        Self {
            device,
            checkpoints,
            next: 0,
            deadline_millis,
            read_errors: 0,
        }
    }

    /// Convenience constructor over a device-table entry.
    pub fn for_device<R>(device: &DeviceConfig<R, G>, now_millis: u64) -> Self {
        Self::new(device.name, device.checkpoints, now_millis)
    }

    /// Folds one observation of the awaited checkpoint's signal into the
    /// walk.
    ///
    /// This is the pure core: it never touches a monitor — obtaining the
    /// observation is [`poll`](Self::poll)'s job, and a caller that already
    /// holds boot evidence as events can drive this directly. `E` is free
    /// because this layer produces no monitor-flavored failures.
    ///
    /// Decision order:
    ///
    /// - [`BootStatus::Booted`] advances to the next checkpoint even when
    ///   `now_millis` is at or past the deadline — the observation is
    ///   proof, and a polling loop that oversleeps its final poll must not
    ///   condemn a device that is provably up. The next checkpoint's window
    ///   opens at `now_millis`.
    /// - [`BootStatus::Failed`] fails the walk immediately, with window
    ///   time left or not.
    /// - [`BootStatus::Booting`] is judged against the window: the walk
    ///   fails once `now_millis` reaches the deadline (the window is
    ///   half-open — expiry is `now_millis >= deadline`).
    ///
    /// A completed walk ignores `status` and keeps returning
    /// [`Progress::Complete`]. An `Err` is terminal: the caller stops
    /// observing and hands the failure to recovery.
    pub fn observe<E>(
        &mut self,
        status: BootStatus,
        now_millis: u64,
    ) -> Result<Progress, BootFailure<E>> {
        let Some(awaited) = self.checkpoints.get(self.next) else {
            return Ok(Progress::Complete);
        };
        match status {
            BootStatus::Booted => {
                self.next += 1;
                let Some(opened) = self.checkpoints.get(self.next) else {
                    return Ok(Progress::Complete);
                };
                self.deadline_millis = now_millis.saturating_add(window_millis(opened.window));
                Ok(Progress::Waiting {
                    checkpoint: opened.name,
                    deadline_millis: self.deadline_millis,
                })
            }
            BootStatus::Failed => Err(BootFailure::DeviceReported {
                device: self.device,
                checkpoint: awaited.name,
            }),
            BootStatus::Booting => {
                if now_millis >= self.deadline_millis {
                    Err(BootFailure::WindowExpired {
                        device: self.device,
                        checkpoint: awaited.name,
                    })
                } else {
                    Ok(Progress::Waiting {
                        checkpoint: awaited.name,
                        deadline_millis: self.deadline_millis,
                    })
                }
            }
        }
    }

    /// One poll: look up the awaited checkpoint's monitor, read it, and
    /// fold the observation in via [`observe`](Self::observe).
    ///
    /// The lookup happens on every poll with the *current* checkpoint's
    /// signal, so a device whose checkpoints ride different backends (a
    /// GPIO line, then a heartbeat) is watched by the right monitor at each
    /// step. A completed walk returns [`Progress::Complete`] without
    /// consulting the map — a passive device's monitors are never read.
    ///
    /// A read error is retried on later polls up to
    /// [`MAX_MONITOR_READ_RETRIES`] consecutive times, counted as absent
    /// evidence in the meantime (the window keeps running and may expire
    /// first); one more consecutive error fails the walk as
    /// [`BootFailure::MonitorRead`]. As with [`observe`](Self::observe),
    /// an `Err` is terminal.
    pub fn poll<M: MonitorMap<G>>(
        &mut self,
        monitors: &M,
        now_millis: u64,
    ) -> Result<Progress, BootFailure<<M::Monitor as BootMonitor>::Error>> {
        let Some(awaited) = self.checkpoints.get(self.next) else {
            return Ok(Progress::Complete);
        };
        let Some(monitor) = monitors.monitor_for(&awaited.signal) else {
            return Err(BootFailure::UnmappedSignal {
                device: self.device,
                checkpoint: awaited.name,
            });
        };
        match monitor.boot_status() {
            Ok(status) => {
                self.read_errors = 0;
                self.observe(status, now_millis)
            }
            Err(source) => {
                if self.read_errors < MAX_MONITOR_READ_RETRIES {
                    self.read_errors += 1;
                    self.observe(BootStatus::Booting, now_millis)
                } else {
                    Err(BootFailure::MonitorRead {
                        device: self.device,
                        checkpoint: awaited.name,
                        source,
                    })
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::cell::Cell;
    use core::time::Duration;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct MockFault;

    impl core::fmt::Display for MockFault {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            f.write_str("mock monitor fault")
        }
    }

    impl core::error::Error for MockFault {}

    // Checkpoint shapes mirroring the mock board's archetypes: a single
    // GPIO boot-complete line (the bmc) and a two-checkpoint message path
    // (the nic).

    const BMC: &[BootCheckpoint<u8>] = &[BootCheckpoint {
        name: "boot-complete",
        signal: BootSignal::GpioBootComplete(12),
        window: Duration::from_secs(90),
    }];

    const NIC: &[BootCheckpoint<u8>] = &[
        BootCheckpoint {
            name: "mctp-ready",
            signal: BootSignal::MctpReady,
            window: Duration::from_secs(20),
        },
        BootCheckpoint {
            name: "heartbeat",
            signal: BootSignal::Heartbeat,
            window: Duration::from_secs(10),
        },
    ];

    fn observe(
        walk: &mut BootWalk<u8>,
        status: BootStatus,
        now_millis: u64,
    ) -> Result<Progress, BootFailure<MockFault>> {
        walk.observe(status, now_millis)
    }

    #[test]
    fn a_single_checkpoint_observed_in_time_completes_the_walk() {
        let mut walk = BootWalk::new("bmc", BMC, 0);

        assert_eq!(
            observe(&mut walk, BootStatus::Booting, 1_000).expect("walk failed"),
            Progress::Waiting {
                checkpoint: "boot-complete",
                deadline_millis: 90_000,
            }
        );
        assert_eq!(
            observe(&mut walk, BootStatus::Booted, 5_000).expect("walk failed"),
            Progress::Complete
        );
    }

    #[test]
    fn an_expired_window_fails_the_walk_naming_the_checkpoint() {
        let mut walk = BootWalk::new("bmc", BMC, 0);

        // One millisecond before the deadline the walk still waits...
        assert_eq!(
            observe(&mut walk, BootStatus::Booting, 89_999).expect("walk failed"),
            Progress::Waiting {
                checkpoint: "boot-complete",
                deadline_millis: 90_000,
            }
        );

        // ...at the deadline it fails, and the report names device and
        // checkpoint (Display comes from the core::error::Error contract).
        let err = observe(&mut walk, BootStatus::Booting, 90_000)
            .expect_err("expected the window to expire");
        assert!(matches!(
            err,
            BootFailure::WindowExpired {
                device: "bmc",
                checkpoint: "boot-complete",
            }
        ));
        assert_eq!(err.to_string(), "bmc: checkpoint 'boot-complete' window expired");
    }

    // The second checkpoint's window opens when the first is observed, not
    // at release: mctp-ready at t=15s arms heartbeat's 10s window to expire
    // at t=25s, not t=30s.
    #[test]
    fn each_observation_opens_the_next_checkpoints_window() {
        let mut walk = BootWalk::new("nic", NIC, 0);

        assert_eq!(
            observe(&mut walk, BootStatus::Booted, 15_000).expect("walk failed"),
            Progress::Waiting {
                checkpoint: "heartbeat",
                deadline_millis: 25_000,
            }
        );

        let err = observe(&mut walk, BootStatus::Booting, 25_000)
            .expect_err("expected the second window to expire");
        assert!(matches!(
            err,
            BootFailure::WindowExpired {
                device: "nic",
                checkpoint: "heartbeat",
            }
        ));
    }

    // The observation is proof: a polling loop that oversleeps its final
    // poll must not condemn a device that is provably up.
    #[test]
    fn booted_at_the_deadline_still_advances() {
        let mut walk = BootWalk::new("bmc", BMC, 0);

        assert_eq!(
            observe(&mut walk, BootStatus::Booted, 90_000).expect("walk failed"),
            Progress::Complete
        );
    }

    #[test]
    fn a_device_reported_failure_fails_the_walk_with_window_time_left() {
        let mut walk = BootWalk::new("nic", NIC, 0);

        let err = observe(&mut walk, BootStatus::Failed, 1_000)
            .expect_err("expected the device-reported failure");
        assert!(matches!(
            err,
            BootFailure::DeviceReported {
                device: "nic",
                checkpoint: "mctp-ready",
            }
        ));
        assert_eq!(
            err.to_string(),
            "nic: device reported boot failure at checkpoint 'mctp-ready'"
        );
    }

    // A completed walk ignores further observations — even a Failed one:
    // the verdict was already delivered.
    #[test]
    fn a_completed_walk_stays_complete() {
        let mut walk = BootWalk::new("bmc", BMC, 0);
        observe(&mut walk, BootStatus::Booted, 5_000).expect("walk failed");

        assert_eq!(
            observe(&mut walk, BootStatus::Failed, 6_000).expect("walk failed"),
            Progress::Complete
        );
    }

    #[test]
    fn a_device_with_no_checkpoints_is_complete_from_construction() {
        let mut walk = BootWalk::new("cpld", &[], 0);

        assert_eq!(
            observe(&mut walk, BootStatus::Booting, 0).expect("walk failed"),
            Progress::Complete
        );
    }

    // Sub-millisecond windows round up to one millisecond rather than
    // expiring at the instant they open.
    #[test]
    fn a_sub_millisecond_window_is_not_born_expired() {
        const TIGHT: &[BootCheckpoint<u8>] = &[BootCheckpoint {
            name: "instant",
            signal: BootSignal::Heartbeat,
            window: Duration::from_nanos(1),
        }];
        let mut walk = BootWalk::new("dev", TIGHT, 0);

        assert_eq!(
            observe(&mut walk, BootStatus::Booting, 0).expect("walk failed"),
            Progress::Waiting {
                checkpoint: "instant",
                deadline_millis: 1,
            }
        );
    }

    // ── The poll layer: monitors behind the MonitorMap seam ─────────────

    /// Replays a fixed script of reads; the last entry repeats forever.
    struct ScriptedMonitor {
        script: &'static [Result<BootStatus, MockFault>],
        cursor: Cell<usize>,
    }

    impl ScriptedMonitor {
        fn new(script: &'static [Result<BootStatus, MockFault>]) -> Self {
            Self {
                script,
                cursor: Cell::new(0),
            }
        }
    }

    impl BootMonitor for ScriptedMonitor {
        type Error = MockFault;

        fn boot_status(&self) -> Result<BootStatus, MockFault> {
            let i = self.cursor.get();
            self.cursor.set(i + 1);
            self.script[i.min(self.script.len() - 1)]
        }
    }

    /// One MCTP endpoint backing two signal kinds: the wiring hands out one
    /// thin view per signal, all reading this shared backend.
    struct MctpEndpoint {
        ready: Cell<bool>,
        heartbeat_seen: Cell<bool>,
    }

    enum MctpSignalKind {
        Ready,
        Heartbeat,
    }

    /// The wiring's closed monitor enum — the shape a board declares.
    enum MockMonitor<'a> {
        Gpio(&'a ScriptedMonitor),
        Mctp(&'a MctpEndpoint, MctpSignalKind),
    }

    impl BootMonitor for MockMonitor<'_> {
        type Error = MockFault;

        fn boot_status(&self) -> Result<BootStatus, MockFault> {
            let up = match self {
                Self::Gpio(mon) => return mon.boot_status(),
                Self::Mctp(ep, MctpSignalKind::Ready) => ep.ready.get(),
                Self::Mctp(ep, MctpSignalKind::Heartbeat) => ep.heartbeat_seen.get(),
            };
            Ok(if up {
                BootStatus::Booted
            } else {
                BootStatus::Booting
            })
        }
    }

    struct MockWiring<'a> {
        gpio: Option<MockMonitor<'a>>,
        mctp_ready: Option<MockMonitor<'a>>,
        heartbeat: Option<MockMonitor<'a>>,
    }

    impl<'a> MonitorMap<u8> for MockWiring<'a> {
        type Monitor = MockMonitor<'a>;

        fn monitor_for(&self, signal: &BootSignal<u8>) -> Option<&MockMonitor<'a>> {
            match signal {
                BootSignal::GpioBootComplete(_) => self.gpio.as_ref(),
                BootSignal::MctpReady => self.mctp_ready.as_ref(),
                BootSignal::Heartbeat => self.heartbeat.as_ref(),
                BootSignal::VersionQuery => None,
            }
        }
    }

    /// Wiring for walks that must never consult a monitor.
    struct PanickingMap;

    impl MonitorMap<u8> for PanickingMap {
        type Monitor = ScriptedMonitor;

        fn monitor_for(&self, _: &BootSignal<u8>) -> Option<&ScriptedMonitor> {
            panic!("this walk must never consult a monitor")
        }
    }

    /// Wiring that maps nothing — a board wiring bug.
    struct NoMonitors;

    impl MonitorMap<u8> for NoMonitors {
        type Monitor = ScriptedMonitor;

        fn monitor_for(&self, _: &BootSignal<u8>) -> Option<&ScriptedMonitor> {
            None
        }
    }

    // The lookup happens per poll with the current checkpoint's signal, so
    // both of the nic's checkpoints are answered by the one MCTP endpoint
    // through its per-signal views.
    #[test]
    fn one_backend_serves_several_signal_kinds_through_per_signal_views() {
        let endpoint = MctpEndpoint {
            ready: Cell::new(false),
            heartbeat_seen: Cell::new(false),
        };
        let wiring = MockWiring {
            gpio: None,
            mctp_ready: Some(MockMonitor::Mctp(&endpoint, MctpSignalKind::Ready)),
            heartbeat: Some(MockMonitor::Mctp(&endpoint, MctpSignalKind::Heartbeat)),
        };
        let mut walk = BootWalk::new("nic", NIC, 0);

        assert_eq!(
            walk.poll(&wiring, 1_000).expect("walk failed"),
            Progress::Waiting {
                checkpoint: "mctp-ready",
                deadline_millis: 20_000,
            }
        );

        endpoint.ready.set(true);
        assert_eq!(
            walk.poll(&wiring, 2_000).expect("walk failed"),
            Progress::Waiting {
                checkpoint: "heartbeat",
                deadline_millis: 12_000,
            }
        );

        endpoint.heartbeat_seen.set(true);
        assert_eq!(
            walk.poll(&wiring, 3_000).expect("walk failed"),
            Progress::Complete
        );
    }

    // Two consecutive read errors are tolerated as absent evidence, a
    // successful read resets the count, and the third consecutive error is
    // terminal — with the concrete monitor error kept on the source() chain.
    #[test]
    fn transient_read_errors_are_retried_then_terminal() {
        const SCRIPT: &[Result<BootStatus, MockFault>] = &[
            Err(MockFault),
            Err(MockFault),
            Ok(BootStatus::Booting),
            Err(MockFault),
            Err(MockFault),
            Err(MockFault),
        ];
        let gpio = ScriptedMonitor::new(SCRIPT);
        let wiring = MockWiring {
            gpio: Some(MockMonitor::Gpio(&gpio)),
            mctp_ready: None,
            heartbeat: None,
        };
        let mut walk = BootWalk::new("bmc", BMC, 0);

        for poll in 1..=5u64 {
            assert!(
                matches!(
                    walk.poll(&wiring, poll * 1_000),
                    Ok(Progress::Waiting { .. })
                ),
                "poll {poll} should still be waiting"
            );
        }

        let err = walk
            .poll(&wiring, 6_000)
            .expect_err("expected the third consecutive read error to be terminal");
        assert!(matches!(
            err,
            BootFailure::MonitorRead {
                device: "bmc",
                checkpoint: "boot-complete",
                ..
            }
        ));
        assert_eq!(
            err.to_string(),
            "bmc: monitor read failed at checkpoint 'boot-complete'"
        );
        let source = core::error::Error::source(&err).expect("MonitorRead must carry a source");
        assert!(source.downcast_ref::<MockFault>().is_some());
    }

    // A backend that can only answer "up yet?" (a single ready line) never
    // produces Failed; the failure verdict must not depend on it — expiry
    // is the walk's own judgment.
    #[test]
    fn a_monitor_that_never_reports_failed_still_yields_a_verdict() {
        const STUCK: &[Result<BootStatus, MockFault>] = &[Ok(BootStatus::Booting)];
        let gpio = ScriptedMonitor::new(STUCK);
        let wiring = MockWiring {
            gpio: Some(MockMonitor::Gpio(&gpio)),
            mctp_ready: None,
            heartbeat: None,
        };
        let mut walk = BootWalk::new("bmc", BMC, 0);

        assert!(matches!(
            walk.poll(&wiring, 89_999),
            Ok(Progress::Waiting { .. })
        ));
        assert!(matches!(
            walk.poll(&wiring, 90_000),
            Err(BootFailure::WindowExpired {
                device: "bmc",
                checkpoint: "boot-complete",
            })
        ));
    }

    #[test]
    fn an_unmapped_signal_is_a_named_failure_not_a_hang() {
        let mut walk = BootWalk::new("nic", NIC, 0);

        let err = walk
            .poll(&NoMonitors, 0)
            .expect_err("expected the missing wiring to surface");
        assert!(matches!(
            err,
            BootFailure::UnmappedSignal {
                device: "nic",
                checkpoint: "mctp-ready",
            }
        ));
        assert_eq!(
            err.to_string(),
            "nic: no monitor wired for checkpoint 'mctp-ready'"
        );
    }

    // A passive device is released blind: complete from the first poll,
    // and its monitors — it has none — are provably never consulted.
    #[test]
    fn a_passive_device_never_consults_the_map() {
        let mut walk = BootWalk::new("cpld", &[], 0);

        assert_eq!(
            walk.poll(&PanickingMap, 0).expect("walk failed"),
            Progress::Complete
        );
        // Polling past completion stays Complete without a lookup.
        assert_eq!(
            walk.poll(&PanickingMap, 1_000).expect("walk failed"),
            Progress::Complete
        );
    }

    // Compile fence: BootFailure satisfies the full error contract, so it
    // can ride any seam that expects a core::error::Error.
    fn _assert_error_contract<E: core::error::Error>() {}

    #[test]
    fn boot_failure_is_a_core_error() {
        _assert_error_contract::<BootFailure<MockFault>>();
    }

    // ── The mock board, end to end ───────────────────────────────────────
    // The composition loop from the type-level docs, run over the real
    // mock device table: bmc, then nic, then the passive cpld.

    use std::cell::RefCell;

    struct FakeClock(Cell<u64>);

    impl FakeClock {
        fn now(&self) -> u64 {
            self.0.get()
        }

        fn advance(&self, millis: u64) {
            self.0.set(self.0.get() + millis);
        }
    }

    /// Records every signal lookup, proving which device's evidence was
    /// consulted when (each poll makes exactly one lookup).
    struct RecordingWiring<'a> {
        inner: MockWiring<'a>,
        lookups: RefCell<Vec<&'static str>>,
    }

    impl<'a> MonitorMap<u8> for RecordingWiring<'a> {
        type Monitor = MockMonitor<'a>;

        fn monitor_for(&self, signal: &BootSignal<u8>) -> Option<&MockMonitor<'a>> {
            self.lookups.borrow_mut().push(match signal {
                BootSignal::GpioBootComplete(_) => "gpio",
                BootSignal::MctpReady => "mctp-ready",
                BootSignal::Heartbeat => "heartbeat",
                BootSignal::VersionQuery => "version-query",
            });
            self.inner.monitor_for(signal)
        }
    }

    /// The doc-sketch loop made concrete: release is out of scope here, so
    /// each device's walk starts at the current fake time and waiting
    /// advances the clock by a fixed poll period.
    fn walk_the_table<M>(wiring: &M, clock: &FakeClock) -> Result<(), BootFailure<MockFault>>
    where
        M: MonitorMap<u8>,
        M::Monitor: BootMonitor<Error = MockFault>,
    {
        for dev in board_devices::MANAGED_DEVICES {
            let mut walk = BootWalk::for_device(dev, clock.now());
            while let Progress::Waiting { .. } = walk.poll(wiring, clock.now())? {
                clock.advance(1_000);
            }
        }
        Ok(())
    }

    #[test]
    fn the_mock_board_boots_in_declaration_order() {
        // Scripted evidence per signal (the Gpio arm is just "a scripted
        // backend" here; which transport it models is irrelevant).
        let gpio = ScriptedMonitor::new(&[
            Ok(BootStatus::Booting),
            Ok(BootStatus::Booting),
            Ok(BootStatus::Booted),
        ]);
        let mctp_ready = ScriptedMonitor::new(&[Ok(BootStatus::Booting), Ok(BootStatus::Booted)]);
        let heartbeat = ScriptedMonitor::new(&[Ok(BootStatus::Booted)]);
        let wiring = RecordingWiring {
            inner: MockWiring {
                gpio: Some(MockMonitor::Gpio(&gpio)),
                mctp_ready: Some(MockMonitor::Gpio(&mctp_ready)),
                heartbeat: Some(MockMonitor::Gpio(&heartbeat)),
            },
            lookups: RefCell::new(Vec::new()),
        };
        let clock = FakeClock(Cell::new(0));

        walk_the_table(&wiring, &clock).expect("the mock board must boot");

        // bmc's evidence is exhausted before nic's is first consulted —
        // one device at a time — and the passive cpld consumes no polls.
        assert_eq!(
            *wiring.lookups.borrow(),
            ["gpio", "gpio", "gpio", "mctp-ready", "mctp-ready", "heartbeat"]
        );
    }

    #[test]
    fn a_boot_failure_stops_the_table_walk_naming_the_culprit() {
        let gpio = ScriptedMonitor::new(&[Ok(BootStatus::Booted)]);
        let mctp_ready = ScriptedMonitor::new(&[Ok(BootStatus::Booted)]);
        // The heartbeat never arrives: nic's second window must expire.
        let heartbeat = ScriptedMonitor::new(&[Ok(BootStatus::Booting)]);
        let wiring = RecordingWiring {
            inner: MockWiring {
                gpio: Some(MockMonitor::Gpio(&gpio)),
                mctp_ready: Some(MockMonitor::Gpio(&mctp_ready)),
                heartbeat: Some(MockMonitor::Gpio(&heartbeat)),
            },
            lookups: RefCell::new(Vec::new()),
        };
        let clock = FakeClock(Cell::new(0));

        let err = walk_the_table(&wiring, &clock)
            .expect_err("the missing heartbeat must fail the walk");
        assert!(matches!(
            err,
            BootFailure::WindowExpired {
                device: "nic",
                checkpoint: "heartbeat",
            }
        ));
    }
}
