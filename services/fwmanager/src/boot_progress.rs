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

use fwmanager_api::config::BootCheckpoint;
use fwmanager_api::BootStatus;

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
pub enum BootFailure {
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
}

impl core::fmt::Display for BootFailure {
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
        }
    }
}

impl core::error::Error for BootFailure {}

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
pub struct BootWalk<G: 'static> {
    device: &'static str,
    checkpoints: &'static [BootCheckpoint<G>],
    /// Index of the checkpoint currently awaited; `== checkpoints.len()`
    /// when the walk is complete.
    next: usize,
    /// When the awaited checkpoint's window expires, in the caller's
    /// monotonic milliseconds. Meaningless once the walk is complete.
    deadline_millis: u64,
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
        }
    }

    /// Folds one observation of the awaited checkpoint's signal into the
    /// walk.
    ///
    /// This is the pure core: it never touches a monitor — obtaining the
    /// observation is the polling layer's job, and a caller that already
    /// holds boot evidence as events can drive this directly.
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
    pub fn observe(
        &mut self,
        status: BootStatus,
        now_millis: u64,
    ) -> Result<Progress, BootFailure> {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::time::Duration;
    use fwmanager_api::config::BootSignal;

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
    ) -> Result<Progress, BootFailure> {
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
}
