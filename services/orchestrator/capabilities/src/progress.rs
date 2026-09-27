// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! How far a polled step has come.

/// Bytes processed so far out of the total.
///
/// Shared by every polled seam that works through a payload:
/// [`StageProgress::Transferring`](crate::StageProgress::Transferring)
/// counts bytes written to a device,
/// [`PollOutcome::Processing`](crate::PollOutcome::Processing) counts bytes
/// hashed. `written` is what a caller watching for a stall keys on, so the
/// two seams answer the same question the same way.
///
/// `written` is monotonic within one job and may hold still across calls: a
/// busy device or a retransmit makes no progress and is not an error. It
/// never exceeds `total`.
///
/// The update intake seam has its own `Progress` in
/// `orchestrator-update-api`, with the same two fields. That one is a
/// wire form the PLDM service decodes, and this crate stays out of that
/// process, so the two types are counterparts rather than one shared
/// definition. The names match so the driver's conversion is field for
/// field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Progress {
    /// Bytes the step has got through so far, at or below `total`. A
    /// verify session writes nothing, so read the name that way.
    pub written: u64,
    /// Total payload bytes.
    pub total: u64,
}

impl Progress {
    /// A job at its starting position.
    pub const fn start(total: u64) -> Self {
        Self { written: 0, total }
    }

    /// True once every byte has been processed.
    pub const fn is_complete(&self) -> bool {
        self.written >= self.total
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fresh_job_has_processed_nothing() {
        let p = Progress::start(4096);
        assert_eq!(p.written, 0);
        assert_eq!(p.total, 4096);
        assert!(!p.is_complete());
    }

    #[test]
    fn a_job_is_complete_when_written_reaches_total() {
        assert!(Progress {
            written: 10,
            total: 10
        }
        .is_complete());
        assert!(!Progress {
            written: 9,
            total: 10
        }
        .is_complete());
    }

    #[test]
    fn an_empty_payload_is_complete_from_the_start() {
        assert!(Progress::start(0).is_complete());
    }
}
