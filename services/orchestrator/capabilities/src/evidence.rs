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
///
/// # Wiring a concrete reader
///
/// Concrete readers (e.g. `GpioBootMonitor` in `orchestrator-hal-adapters`)
/// stay signal-agnostic — an adapter crate cannot know a board's `G`.
/// The board impl owns the match; the hardware binding is made once, at
/// construction, and the signal id just proves the right reader was
/// wired:
///
/// ```ignore
/// /// bmc wiring: one ready line behind the board's signal vocabulary.
/// struct BmcReader<'a, P: GpioPort> {
///     // (port, pin, polarity) bound at bring-up from the table's Gpio(12).
///     ready: GpioBootMonitor<'a, P>,
/// }
///
/// impl<P: GpioPort> EvidenceReader<MockSignal> for BmcReader<'_, P>
/// where
///     P::Error: 'static,
/// {
///     type Error = MonitorError<P::Error>;
///
///     fn read(&mut self, signal: &MockSignal) -> Result<BootStatus, Self::Error> {
///         match signal {
///             MockSignal::Gpio(_) => self.ready.boot_status(),
///             other => unreachable!("bmc reader wired to {other:?}"),
///         }
///     }
/// }
/// ```
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

    // ── Message-path evidence (NIC archetype) ───────────────────────────
    // A timeout is never on the wire: a hung device sends nothing, the
    // reader reports Booting forever, and only the orchestrator's clock
    // (the checkpoint's window, judged by the walker) turns that silence
    // into a verdict. The three channels stay separate: silence → Booting;
    // the device speaks → FailedRetriable/FailedFatal ends the wait early;
    // the channel breaks → Err.

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum NicSignal {
        /// The endpoint answers a control query as ready.
        MctpReady,
        /// A heartbeat message arrived (latched; reset clears it).
        Heartbeat,
    }

    struct MockNicEndpoint {
        /// Control queries are answered after this many reads; `None` =
        /// the device is hung. Silence is the only "timeout signal" a
        /// device has — there is no message for it.
        responds_after: Option<usize>,
        reads: usize,
        /// Device-sent failure notification, latched (reset clears it) —
        /// what the message path *can* carry: an active verdict.
        fault_code: Option<u8>,
        /// Heartbeat arrival, latched by the transport.
        heartbeat_seen: bool,
        /// Injected transport fault: the channel itself breaks.
        bus_fault: bool,
    }

    impl MockNicEndpoint {
        fn silent() -> Self {
            Self {
                responds_after: None,
                reads: 0,
                fault_code: None,
                heartbeat_seen: false,
                bus_fault: false,
            }
        }
    }

    #[derive(Debug, PartialEq, Eq)]
    struct MctpFault;

    impl core::fmt::Display for MctpFault {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            f.write_str("mctp transport fault")
        }
    }

    impl core::error::Error for MctpFault {}

    impl EvidenceReader<NicSignal> for MockNicEndpoint {
        type Error = MctpFault;

        fn read(&mut self, signal: &NicSignal) -> Result<BootStatus, MctpFault> {
            if self.bus_fault {
                return Err(MctpFault);
            }
            match signal {
                NicSignal::MctpReady => {
                    if let Some(code) = self.fault_code {
                        return Ok(match code {
                            0xEE => BootStatus::FailedRetriable,
                            _ => BootStatus::FailedFatal,
                        });
                    }
                    // Query answered => evidence; no answer => no evidence
                    // yet. NOT an error — the channel is fine, the device
                    // is silent.
                    self.reads += 1;
                    Ok(match self.responds_after {
                        Some(n) if self.reads > n => BootStatus::Booted,
                        _ => BootStatus::Booting,
                    })
                }
                NicSignal::Heartbeat => Ok(match self.heartbeat_seen {
                    true => BootStatus::Booted,
                    false => BootStatus::Booting,
                }),
            }
        }
    }

    // A hung endpoint is Booting on every read, forever — turning that
    // into a timeout is the walker's job, on the orchestrator's clock.
    #[test]
    fn a_hung_endpoint_reads_booting_forever() {
        let mut nic = MockNicEndpoint::silent();

        for _ in 0..100 {
            assert_eq!(
                nic.read(&NicSignal::MctpReady).expect("read failed"),
                BootStatus::Booting
            );
        }
    }

    #[test]
    fn silence_ends_once_the_endpoint_answers() {
        let mut nic = MockNicEndpoint {
            responds_after: Some(2),
            ..MockNicEndpoint::silent()
        };

        assert_eq!(
            nic.read(&NicSignal::MctpReady).expect("read failed"),
            BootStatus::Booting
        );
        assert_eq!(
            nic.read(&NicSignal::MctpReady).expect("read failed"),
            BootStatus::Booting
        );
        assert_eq!(
            nic.read(&NicSignal::MctpReady).expect("read failed"),
            BootStatus::Booted
        );
    }

    // A device that is up enough to talk reports its own verdict and ends
    // the wait early — no window needs to expire.
    #[test]
    fn a_talking_device_reports_its_own_verdict() {
        let mut nic = MockNicEndpoint {
            fault_code: Some(0xEE),
            ..MockNicEndpoint::silent()
        };
        assert_eq!(
            nic.read(&NicSignal::MctpReady).expect("read failed"),
            BootStatus::FailedRetriable
        );

        let mut nic = MockNicEndpoint {
            fault_code: Some(0x03),
            ..MockNicEndpoint::silent()
        };
        assert_eq!(
            nic.read(&NicSignal::MctpReady).expect("read failed"),
            BootStatus::FailedFatal
        );
    }

    // Channel trouble is the reader's Error — distinct from both silence
    // and a device-reported verdict.
    #[test]
    fn a_broken_channel_is_an_error_not_evidence() {
        let mut nic = MockNicEndpoint {
            bus_fault: true,
            ..MockNicEndpoint::silent()
        };

        let err = nic
            .read(&NicSignal::MctpReady)
            .expect_err("expected the transport fault");
        assert_eq!(err.to_string(), "mctp transport fault");
    }

    #[test]
    fn a_latched_heartbeat_reads_booted() {
        let mut nic = MockNicEndpoint {
            heartbeat_seen: true,
            ..MockNicEndpoint::silent()
        };

        assert_eq!(
            nic.read(&NicSignal::Heartbeat).expect("read failed"),
            BootStatus::Booted
        );
    }
}
