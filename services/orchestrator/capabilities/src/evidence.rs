// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Evidence reading: resolve a board-defined signal id to boot liveness.

use crate::BootStatus;

/// Reads a device's boot evidence, one signal at a time.
///
/// Implemented by board wiring — typically once per managed device, so
/// each device's boot walk borrows only its own reader. `G` is the
/// board's signal vocabulary; an exhaustive `match` on it keeps dispatch
/// direct and makes a forgotten signal a compile error, not a runtime
/// hole.
///
/// The status must describe the **current** boot cycle — see
/// [`BootStatus`] for the latching contract (evidence is cleared by the
/// reset path, never by the reader).
pub trait EvidenceReader<G> {
    /// The error type reported by this reader.
    ///
    /// Requires [`core::error::Error`] (in `core` since Rust 1.81) so the
    /// orchestrator gets `Display` and a `source()` cause chain, not just
    /// a `Debug` dump. Error categories stay implementation-defined —
    /// this crate names no error vocabulary of its own.
    type Error: core::error::Error;

    /// Returns the current liveness evidence for `signal`.
    ///
    /// # Errors
    ///
    /// Returns an error if the evidence channel behind `signal` cannot be
    /// read.
    fn read(&mut self, signal: &G) -> Result<BootStatus, Self::Error>;
}

#[cfg(test)]
mod tests {
    use super::*;

    // A reader implemented against no HAL at all — the contract must be
    // satisfiable from any stack. One monotonic progress register serves
    // four staged-boot signals through one reader (the pattern a real SoC
    // board is expected to use); fault codes in the same register carry
    // the device's own judgment, fatal or retriable, for every signal.

    const POISON: u8 = 0xFF;
    const TRANSIENT: u8 = 0xEE;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum TestSignal {
        /// Booted once the progress register reaches this level
        /// (1 = bl1, 2 = bl2, 3 = kernel, 4 = service).
        Progress(u8),
    }

    struct SocReader {
        level: u8,
        fail: bool,
    }

    #[derive(Debug, PartialEq, Eq)]
    struct RegFault;

    impl core::fmt::Display for RegFault {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            f.write_str("progress register unreadable")
        }
    }

    impl core::error::Error for RegFault {}

    impl EvidenceReader<TestSignal> for SocReader {
        type Error = RegFault;

        fn read(&mut self, signal: &TestSignal) -> Result<BootStatus, RegFault> {
            if self.fail {
                return Err(RegFault);
            }
            let TestSignal::Progress(threshold) = *signal;
            Ok(match self.level {
                POISON => BootStatus::FailedFatal,
                TRANSIENT => BootStatus::FailedRetriable,
                l if l >= threshold => BootStatus::Booted,
                _ => BootStatus::Booting,
            })
        }
    }

    // One register, four signals: each read sees exactly its own
    // threshold, so a device mid-boot passes the early stages and not the
    // late ones.
    #[test]
    fn one_reader_serves_a_staged_boot() {
        let mut soc = SocReader {
            level: 2,
            fail: false,
        };
        let mut read = |threshold| {
            soc.read(&TestSignal::Progress(threshold))
                .expect("read failed")
        };

        assert_eq!(read(1), BootStatus::Booted); // bl1
        assert_eq!(read(2), BootStatus::Booted); // bl2
        assert_eq!(read(3), BootStatus::Booting); // kernel
        assert_eq!(read(4), BootStatus::Booting); // service
    }

    // Fault codes must read the same for every signal, whichever stage
    // the walk happens to be awaiting — and they carry the device's own
    // retriability judgment.
    #[test]
    fn a_poisoned_register_fails_every_signal_fatally() {
        let mut soc = SocReader {
            level: POISON,
            fail: false,
        };

        for threshold in 1..=4 {
            assert_eq!(
                soc.read(&TestSignal::Progress(threshold))
                    .expect("read failed"),
                BootStatus::FailedFatal
            );
        }
    }

    #[test]
    fn a_transient_fault_reads_retriable_for_every_signal() {
        let mut soc = SocReader {
            level: TRANSIENT,
            fail: false,
        };

        for threshold in 1..=4 {
            assert_eq!(
                soc.read(&TestSignal::Progress(threshold))
                    .expect("read failed"),
                BootStatus::FailedRetriable
            );
        }
    }

    #[test]
    fn errors_surface_through_the_reader() {
        let mut soc = SocReader {
            level: 0,
            fail: true,
        };

        let err = soc
            .read(&TestSignal::Progress(1))
            .expect_err("expected the register fault");

        // Display comes from the core::error::Error bound, not a Debug dump.
        assert_eq!(err.to_string(), "progress register unreadable");
    }
}
