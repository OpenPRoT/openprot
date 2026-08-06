// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Deterministic boot-window observation on top of [`BootMonitor`].

use crate::{BootMonitor, BootStatus};

/// The outcome of watching one boot window: evidence (`Booted`/`Failed`)
/// or an expired window (`Timeout`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootProgress {
    /// Boot completion observed within the window.
    Booted,
    /// The device reported a boot failure within the window.
    Failed,
    /// The window expired with no evidence either way.
    Timeout,
}

/// Watch a boot window by polling `monitor` at most `poll_budget` times.
///
/// The poll budget is the deterministic rendering of a checkpoint's wall
/// clock window, so tests never sleep.
///
/// # Errors
///
/// Propagates the first monitor read error.
pub fn await_boot<M: BootMonitor>(
    monitor: &M,
    poll_budget: usize,
) -> Result<BootProgress, M::Error> {
    for _ in 0..poll_budget {
        match monitor.boot_status()? {
            BootStatus::Booted => return Ok(BootProgress::Booted),
            BootStatus::Failed => return Ok(BootProgress::Failed),
            BootStatus::Booting => {}
        }
    }
    Ok(BootProgress::Timeout)
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::cell::Cell;

    /// A monitor scripted by "boots after N polls", HAL-free.
    struct ScriptedMonitor {
        polls_left: Cell<Option<usize>>,
        report_failure: bool,
    }

    #[derive(Debug)]
    struct Unreachable;

    impl core::fmt::Display for Unreachable {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            f.write_str("monitor read failed")
        }
    }

    impl core::error::Error for Unreachable {}

    impl BootMonitor for ScriptedMonitor {
        type Error = Unreachable;

        fn boot_status(&self) -> Result<BootStatus, Unreachable> {
            match self.polls_left.get() {
                Some(0) => Ok(if self.report_failure {
                    BootStatus::Failed
                } else {
                    BootStatus::Booted
                }),
                Some(n) => {
                    self.polls_left.set(Some(n - 1));
                    Ok(BootStatus::Booting)
                }
                None => Ok(BootStatus::Booting),
            }
        }
    }

    #[test]
    fn boot_within_budget_is_booted() {
        let monitor = ScriptedMonitor {
            polls_left: Cell::new(Some(3)),
            report_failure: false,
        };
        assert_eq!(await_boot(&monitor, 10).unwrap(), BootProgress::Booted);
    }

    #[test]
    fn reported_failure_ends_the_window_early() {
        let monitor = ScriptedMonitor {
            polls_left: Cell::new(Some(2)),
            report_failure: true,
        };
        assert_eq!(await_boot(&monitor, 10).unwrap(), BootProgress::Failed);
    }

    #[test]
    fn exhausted_budget_is_timeout() {
        let monitor = ScriptedMonitor {
            polls_left: Cell::new(None),
            report_failure: false,
        };
        assert_eq!(await_boot(&monitor, 10).unwrap(), BootProgress::Timeout);
    }

    #[test]
    fn zero_budget_never_polls_and_times_out() {
        let monitor = ScriptedMonitor {
            polls_left: Cell::new(Some(0)),
            report_failure: false,
        };
        assert_eq!(await_boot(&monitor, 0).unwrap(), BootProgress::Timeout);
    }
}
