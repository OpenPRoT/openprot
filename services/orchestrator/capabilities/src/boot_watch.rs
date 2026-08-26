// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! The orchestrator-facing seam of boot supervision.

/// One device's boot walk, pollable without knowing the device type.
///
/// Everything device-specific — the driver type, its error type, the
/// checkpoint list — stays inside the concrete walk; the orchestrator's
/// fleet view is uniform. Object-safe so a heterogeneous fleet can sit
/// behind `&mut dyn BootWatch`; a board preferring static dispatch wraps
/// its walks in an enum and matches, without touching anything below the
/// seam.
pub trait BootWatch {
    /// Starts a fresh attempt: previous progress is discarded and the walk
    /// judges from its first checkpoint again. The driver calls this on
    /// every reset release, retries included. Takes no timestamp — reset
    /// actuation has no clock; the attempt starts at the next
    /// [`poll`](BootWatch::poll)'s `now_millis`.
    fn arm(&mut self);

    /// Judges the walk at `now_millis` (monotonic). Never sleeps — time is
    /// injected, so every decision is host-testable.
    fn poll(&mut self, now_millis: u64) -> WalkVerdict;
}

/// Everything the orchestrator needs to know about a boot walk.
///
/// Observation only: the walk judges checkpoint windows, never lives.
/// Retry counts and terminal calls belong to the orchestrator state
/// machine (`ComponentStatus.retry`, the `Recovering` → `RecoveryFailed`
/// path) — a verdict that carried a retry budget would be a second owner
/// for the same decision, free to disagree with the first.
///
/// Deliberately free of device and error types: the concrete detail is
/// logged by the walk while it is still in scope, not carried across the
/// seam.
///
/// Intentionally exhaustive (not `#[non_exhaustive]`): adding a verdict is
/// a breaking change, so the compiler forces every consumer — in particular
/// the orchestrator's event mapping — to handle it explicitly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WalkVerdict {
    /// Nothing to decide yet; poll again by `deadline_millis`.
    Waiting {
        /// When the awaited checkpoint's window expires.
        deadline_millis: u64,
    },
    /// Every checkpoint passed — the device is up. Which state-machine
    /// event this becomes is the platform driver's mapping, by component
    /// kind:
    /// `ComponentReady` for an iRoT-backed device, `Booted` for a
    /// symbiont.
    Complete,
    /// This boot attempt failed at `checkpoint`; the walk is over.
    /// Whether to try again, recover, or give up is the orchestrator's
    /// decision — a retry re-resets the device and starts a fresh walk.
    Failed {
        /// The checkpoint the attempt died at.
        checkpoint: &'static str,
        /// Why it died — the one input the retry decision needs.
        cause: FailureCause,
    },
}

/// Why a boot attempt failed at a checkpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureCause {
    /// The window expired; the device reported nothing.
    TimedOut,
    /// The device reported a failure worth another attempt
    /// ([`FailedRetriable`](crate::BootStatus::FailedRetriable)) — the
    /// wait ended early.
    DeviceRetriable,
    /// The device reported a terminal failure
    /// ([`FailedFatal`](crate::BootStatus::FailedFatal)) — re-running the
    /// same image cannot change the verdict, whatever retry budget the
    /// orchestrator has left.
    DeviceFatal,
}

#[cfg(test)]
mod tests {
    use super::*;

    // A BootWatch implemented against no walker at all — the seam must be
    // satisfiable by anything that can produce verdicts, and must stay
    // object-safe (the fleet array below fails to compile otherwise).

    struct ScriptedWalk {
        verdicts: &'static [WalkVerdict],
        next: usize,
    }

    impl BootWatch for ScriptedWalk {
        fn arm(&mut self) {
            self.next = 0;
        }

        fn poll(&mut self, _now_millis: u64) -> WalkVerdict {
            let v = self.verdicts[self.next];
            self.next += 1;
            v
        }
    }

    #[test]
    fn a_heterogeneous_fleet_pumps_through_the_erased_seam() {
        let mut bmc = ScriptedWalk {
            verdicts: &[
                WalkVerdict::Waiting {
                    deadline_millis: 90_000,
                },
                WalkVerdict::Complete,
            ],
            next: 0,
        };
        let mut nic = ScriptedWalk {
            verdicts: &[
                WalkVerdict::Failed {
                    checkpoint: "heartbeat",
                    cause: FailureCause::TimedOut,
                },
                WalkVerdict::Failed {
                    checkpoint: "heartbeat",
                    cause: FailureCause::DeviceFatal,
                },
            ],
            next: 0,
        };

        let fleet: &mut [&mut dyn BootWatch] = &mut [&mut bmc, &mut nic];

        let first: [WalkVerdict; 2] = [fleet[0].poll(0), fleet[1].poll(0)];
        let second: [WalkVerdict; 2] = [fleet[0].poll(1), fleet[1].poll(1)];

        assert_eq!(
            first,
            [
                WalkVerdict::Waiting {
                    deadline_millis: 90_000
                },
                WalkVerdict::Failed {
                    checkpoint: "heartbeat",
                    cause: FailureCause::TimedOut
                },
            ]
        );
        assert_eq!(
            second,
            [
                WalkVerdict::Complete,
                WalkVerdict::Failed {
                    checkpoint: "heartbeat",
                    cause: FailureCause::DeviceFatal
                },
            ]
        );
    }
}
