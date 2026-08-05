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
    /// Judges the walk at `now_millis` (monotonic). Never sleeps — time is
    /// injected, so every decision is host-testable.
    fn poll(&mut self, now_millis: u64) -> WalkVerdict;
}

/// Everything the orchestrator needs to know about a boot walk.
///
/// Deliberately free of device and error types: the orchestrator acts the
/// same whatever the cause, so the concrete detail is logged by the walk
/// while it is still in scope, not carried across the seam.
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
    /// Every checkpoint passed — the device is up.
    Complete,
    /// A window expired or the device reported failure, with retry budget
    /// left; the window is re-armed. The caller re-resets the device and
    /// keeps polling — what a retry re-runs is the caller's policy.
    Retry {
        /// The checkpoint that failed.
        checkpoint: &'static str,
        /// Attempts left after this one.
        retries_left: u8,
    },
    /// Retry budget exhausted — this boot is dead. Recovery is the
    /// caller's move.
    Dead {
        /// The checkpoint the boot died at.
        checkpoint: &'static str,
    },
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
                WalkVerdict::Retry {
                    checkpoint: "heartbeat",
                    retries_left: 1,
                },
                WalkVerdict::Dead {
                    checkpoint: "heartbeat",
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
                WalkVerdict::Retry {
                    checkpoint: "heartbeat",
                    retries_left: 1
                },
            ]
        );
        assert_eq!(
            second,
            [
                WalkVerdict::Complete,
                WalkVerdict::Dead {
                    checkpoint: "heartbeat"
                },
            ]
        );
    }
}
