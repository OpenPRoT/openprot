// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

use orchestrator_capabilities::{BootStatus, BootWatch, EvidenceReader, FailureCause, WalkVerdict};
use orchestrator_config::{BootCheckpoint, DeviceConfig};

/// Walks a device's [`BootCheckpoint`]s in declaration order, reading
/// evidence from an [`EvidenceReader`] at each poll. The checkpoint list
/// comes from a [`DeviceConfig`], which validates non-empty and unique
/// names at build time; lifetimes match the board config (`&'static`).
///
/// A read error is treated as silence
/// (`Booting`), so a transient bus glitch does not kill a healthy boot.
/// A lapsed window is a timeout with no final read: a device that has not
/// reported cannot be judged on a race. Each checkpoint's deadline
/// starts from the first poll after arm, not from the arm call, so
/// time between arming and polling does not count against the window.
/// An unarmed or post-terminal poll
/// returns `Waiting { deadline_millis: u64::MAX }` (no deadline).
pub struct CheckpointWalk<R, P: 'static> {
    reader: R,
    checkpoints: &'static [BootCheckpoint<P>],
    phase: Phase,
}

enum Phase {
    Idle,
    Armed,
    Walking { cursor: usize, deadline_millis: u64 },
}

impl<R, P> CheckpointWalk<R, P> {
    /// Binds a reader and a device's checkpoints into a walk. The
    /// checkpoints were already checked non-empty and uniquely named at
    /// build time by `DeviceConfig::new`.
    pub fn new<S>(reader: R, device: &'static DeviceConfig<S, P>) -> Self {
        Self {
            reader,
            checkpoints: device.checkpoints(),
            phase: Phase::Idle,
        }
    }

    /// Mutable access to the reader.
    pub fn reader_mut(&mut self) -> &mut R {
        &mut self.reader
    }
}

impl<R: EvidenceReader<P>, P> BootWatch for CheckpointWalk<R, P> {
    fn arm(&mut self) {
        self.phase = Phase::Armed;
    }

    fn poll(&mut self, now_millis: u64) -> WalkVerdict {
        if let Phase::Armed = self.phase {
            let timeout_millis = self.checkpoints[0].timeout().as_millis() as u64;
            let deadline = now_millis.saturating_add(timeout_millis);
            self.phase = Phase::Walking {
                cursor: 0,
                deadline_millis: deadline,
            };
        }

        let (cursor, deadline) = match self.phase {
            Phase::Walking {
                cursor,
                deadline_millis,
            } => (cursor, deadline_millis),
            _ => {
                return WalkVerdict::Waiting {
                    deadline_millis: u64::MAX,
                }
            }
        };

        if now_millis >= deadline {
            self.phase = Phase::Idle;
            return WalkVerdict::Failed {
                checkpoint: self.checkpoints[cursor].name(),
                cause: FailureCause::TimedOut,
            };
        }

        let status = self
            .reader
            .read(self.checkpoints[cursor].probe())
            .unwrap_or(BootStatus::Booting);

        match status {
            BootStatus::Booting => WalkVerdict::Waiting {
                deadline_millis: deadline,
            },
            BootStatus::Booted => {
                let next = cursor + 1;
                if next == self.checkpoints.len() {
                    self.phase = Phase::Idle;
                    WalkVerdict::Complete
                } else {
                    let timeout_millis = self.checkpoints[next].timeout().as_millis() as u64;
                    let new_deadline = now_millis.saturating_add(timeout_millis);
                    self.phase = Phase::Walking {
                        cursor: next,
                        deadline_millis: new_deadline,
                    };
                    WalkVerdict::Waiting {
                        deadline_millis: new_deadline,
                    }
                }
            }
            BootStatus::FailedRetriable => {
                self.phase = Phase::Idle;
                WalkVerdict::Failed {
                    checkpoint: self.checkpoints[cursor].name(),
                    cause: FailureCause::DeviceRetriable,
                }
            }
            BootStatus::FailedFatal => {
                self.phase = Phase::Idle;
                WalkVerdict::Failed {
                    checkpoint: self.checkpoints[cursor].name(),
                    cause: FailureCause::DeviceFatal,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::time::Duration;
    use orchestrator_config::DeviceConfig;

    const BL1: BootCheckpoint<u8> = BootCheckpoint::new("bl1", 1, Duration::from_millis(100));
    const KERNEL: BootCheckpoint<u8> = BootCheckpoint::new("kernel", 2, Duration::from_millis(200));
    const CHECKPOINTS: &[BootCheckpoint<u8>] = &[BL1, KERNEL];

    static DEVICE: DeviceConfig<u8, u8> = DeviceConfig::new("test-dev", 0, CHECKPOINTS, None);
    static ONE_CP_DEVICE: DeviceConfig<u8, u8> = DeviceConfig::new("one-cp-dev", 0, &[BL1], None);

    // A progress-register reader: probe N is Booted once progress >= N.
    // Mirrors the SocReader archetype in the evidence tests.
    struct ProgressReader {
        level: u8,
        fault: Option<BootStatus>,
        fail_read: bool,
    }

    impl ProgressReader {
        fn new() -> Self {
            Self {
                level: 0,
                fault: None,
                fail_read: false,
            }
        }
    }

    #[derive(Debug)]
    struct ReadFault;

    impl core::fmt::Display for ReadFault {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            f.write_str("read fault")
        }
    }

    impl core::error::Error for ReadFault {}

    impl EvidenceReader<u8> for ProgressReader {
        type Error = ReadFault;

        fn read(&mut self, probe: &u8) -> Result<BootStatus, ReadFault> {
            if self.fail_read {
                return Err(ReadFault);
            }
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

    fn walk() -> CheckpointWalk<ProgressReader, u8> {
        CheckpointWalk::new(ProgressReader::new(), &DEVICE)
    }

    // ── Happy path ──────────────────────────────────────────────────────

    #[test]
    fn two_checkpoint_walk_completes_when_both_pass() {
        let mut w = walk();
        w.reader_mut().level = 2;
        w.arm();

        let v = w.poll(0);
        assert_eq!(
            v,
            WalkVerdict::Waiting {
                deadline_millis: 200
            },
            "bl1 passed, now waiting on kernel"
        );

        let v = w.poll(0);
        assert_eq!(v, WalkVerdict::Complete);
    }

    #[test]
    fn single_checkpoint_walk_completes_in_one_poll() {
        let mut w = CheckpointWalk::new(ProgressReader::new(), &ONE_CP_DEVICE);
        w.reader_mut().level = 1;
        w.arm();

        assert_eq!(w.poll(0), WalkVerdict::Complete);
    }

    #[test]
    fn progress_between_polls_advances_the_walk() {
        let mut w = walk();
        w.arm();

        let v = w.poll(0);
        assert_eq!(
            v,
            WalkVerdict::Waiting {
                deadline_millis: 100
            }
        );

        w.reader_mut().level = 1;
        let v = w.poll(50);
        assert_eq!(
            v,
            WalkVerdict::Waiting {
                deadline_millis: 250
            },
            "bl1 passed at t=50, kernel deadline = 50 + 200"
        );

        w.reader_mut().level = 2;
        let v = w.poll(100);
        assert_eq!(v, WalkVerdict::Complete);
    }

    // ── Timeout ─────────────────────────────────────────────────────────

    #[test]
    fn first_checkpoint_times_out_when_device_is_silent() {
        let mut w = walk();
        w.arm();

        let v = w.poll(0);
        assert_eq!(
            v,
            WalkVerdict::Waiting {
                deadline_millis: 100
            }
        );

        let v = w.poll(100);
        assert_eq!(
            v,
            WalkVerdict::Failed {
                checkpoint: "bl1",
                cause: FailureCause::TimedOut,
            }
        );
    }

    #[test]
    fn second_checkpoint_times_out_after_first_passes() {
        let mut w = walk();
        w.arm();

        w.reader_mut().level = 1;
        let v = w.poll(0);
        assert_eq!(
            v,
            WalkVerdict::Waiting {
                deadline_millis: 200
            }
        );

        let v = w.poll(200);
        assert_eq!(
            v,
            WalkVerdict::Failed {
                checkpoint: "kernel",
                cause: FailureCause::TimedOut,
            }
        );
    }

    // ── Device-reported failures ────────────────────────────────────────

    #[test]
    fn retriable_failure_ends_the_walk_early() {
        let mut w = walk();
        w.arm();
        w.reader_mut().fault = Some(BootStatus::FailedRetriable);

        let v = w.poll(0);
        assert_eq!(
            v,
            WalkVerdict::Failed {
                checkpoint: "bl1",
                cause: FailureCause::DeviceRetriable,
            }
        );
    }

    #[test]
    fn fatal_failure_ends_the_walk_early() {
        let mut w = walk();
        w.arm();
        w.reader_mut().fault = Some(BootStatus::FailedFatal);

        let v = w.poll(0);
        assert_eq!(
            v,
            WalkVerdict::Failed {
                checkpoint: "bl1",
                cause: FailureCause::DeviceFatal,
            }
        );
    }

    // ── Read errors ─────────────────────────────────────────────────────

    #[test]
    fn read_error_treated_as_silence() {
        let mut w = walk();
        w.arm();
        w.reader_mut().fail_read = true;

        let v = w.poll(0);
        assert_eq!(
            v,
            WalkVerdict::Waiting {
                deadline_millis: 100
            },
            "bus glitch does not kill a healthy boot"
        );

        // Clear the fault and advance: the walk continues.
        w.reader_mut().fail_read = false;
        w.reader_mut().level = 2;
        let v = w.poll(10);
        assert_eq!(
            v,
            WalkVerdict::Waiting {
                deadline_millis: 210
            }
        );
        assert_eq!(w.poll(10), WalkVerdict::Complete);
    }

    // ── arm() rewinds ───────────────────────────────────────────────────

    #[test]
    fn arm_rewinds_to_the_first_checkpoint() {
        let mut w = walk();
        w.reader_mut().level = 2;
        w.arm();
        assert_eq!(
            w.poll(0),
            WalkVerdict::Waiting {
                deadline_millis: 200
            }
        );
        assert_eq!(w.poll(0), WalkVerdict::Complete);

        // Re-arm: back to checkpoint 0.
        w.reader_mut().level = 0;
        w.arm();
        let v = w.poll(1000);
        assert_eq!(
            v,
            WalkVerdict::Waiting {
                deadline_millis: 1100
            },
            "fresh deadline from the re-arm"
        );
    }

    #[test]
    fn arm_mid_walk_restarts_from_the_beginning() {
        let mut w = walk();
        w.reader_mut().level = 1;
        w.arm();
        w.poll(0); // passes bl1, now at kernel

        w.arm(); // restart
        w.reader_mut().level = 0;
        let v = w.poll(500);
        assert_eq!(
            v,
            WalkVerdict::Waiting {
                deadline_millis: 600
            },
            "restarted at bl1 with a fresh deadline"
        );
    }

    // ── Unarmed / idle ─────────────────────────────────────────────────

    #[test]
    fn unarmed_walk_waits_indefinitely() {
        let mut w = walk();
        assert_eq!(
            w.poll(0),
            WalkVerdict::Waiting {
                deadline_millis: u64::MAX
            }
        );
    }

    #[test]
    fn idle_after_terminal_waits_indefinitely() {
        let mut w = walk();
        w.arm();
        w.poll(0); // Armed -> Walking, deadline = 100
        let v = w.poll(100); // now >= deadline -> TimedOut
        assert!(matches!(v, WalkVerdict::Failed { .. }));

        assert_eq!(
            w.poll(200),
            WalkVerdict::Waiting {
                deadline_millis: u64::MAX
            }
        );
    }

    // ── Deadline arithmetic ─────────────────────────────────────────────

    #[test]
    fn deadline_is_relative_to_first_poll_not_arm() {
        let mut w = walk();
        w.arm();
        // First poll at t=1000: deadline should be 1000 + 100, not 0 + 100.
        let v = w.poll(1000);
        assert_eq!(
            v,
            WalkVerdict::Waiting {
                deadline_millis: 1100
            }
        );
    }

    #[test]
    fn next_checkpoint_deadline_is_relative_to_the_passing_poll() {
        let mut w = walk();
        w.reader_mut().level = 1;
        w.arm();

        // bl1 passes at t=50, kernel deadline = 50 + 200.
        let v = w.poll(50);
        assert_eq!(
            v,
            WalkVerdict::Waiting {
                deadline_millis: 250
            }
        );
    }

    // ── Lapsed window = timeout, no last-chance read ────────────────────

    #[test]
    fn booted_at_expiry_is_still_timeout() {
        let mut w = walk();
        w.arm();
        w.poll(0); // Armed -> Walking, deadline = 100

        w.reader_mut().level = 1;
        let v = w.poll(100); // device ready, but window already lapsed
        assert_eq!(
            v,
            WalkVerdict::Failed {
                checkpoint: "bl1",
                cause: FailureCause::TimedOut,
            },
            "timeout checked before reading: a lapsed window is final"
        );
    }

    // ── Fault at a later checkpoint names it correctly ────────────────

    #[test]
    fn device_fault_at_second_checkpoint_names_it() {
        let mut w = walk();
        w.reader_mut().level = 1;
        w.arm();
        w.poll(0); // bl1 passes, now at kernel

        w.reader_mut().fault = Some(BootStatus::FailedFatal);
        let v = w.poll(10);
        assert_eq!(
            v,
            WalkVerdict::Failed {
                checkpoint: "kernel",
                cause: FailureCause::DeviceFatal,
            }
        );
    }
}
