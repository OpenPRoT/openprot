// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! The [`Updatable`] update capability contract.

/// Update capability: stage a payload on one managed device and mark the
/// staged image as its boot candidate.
///
/// The two operations are steps 3 and 5 of the AMD draft's authenticated
/// update sequence (§5.3.1): write to the inactive slot only, then update
/// slot metadata to prefer the new image. Universal across device
/// archetypes (capability-split ADR 0002): a direct-flash adapter writes
/// the payload into the device's inactive slot itself, a PLDM adapter
/// drives the device's own transfer. Slot identity never crosses the
/// seam — which slot is inactive is device state, so
/// [`activate`](Self::activate) can only ever mean "the image the last
/// completed staging delivered".
///
/// What this trait deliberately does not claim:
///
/// - **Verification** (§5.3.1 step 2) runs on the candidate before staging
///   as orchestrator policy; post-write read-back (step 4) is the optional
///   `ReadBack` capability.
/// - **Commit.** Activation is always tentative — AMD's "mark as preferred
///   boot target", proposing, never committing. The commit gate is the
///   optional `TrialBoot` capability, which resolves what `activate`
///   proposed; there is no second slot-selection owner.
/// - **Booting.** Resetting the device into the candidate is
///   [`BootControl`](crate::BootControl). When activation takes effect
///   (next reset, or a device-internal restart on self-activating
///   devices) is device-defined; sequencing belongs to the flows.
///
/// # Contract
///
/// - **Staging is inert.** However staging ends — fault, abandon, power
///   loss — the active image is untouched: the staging area is inactive
///   by construction. Staging anew is always allowed and discards any
///   previously staged, unactivated payload.
/// - **Staging is polled, never blocking.** A payload is tens of
///   megabytes and a transfer takes minutes; each
///   [`poll_stage`](Self::poll_stage) call does one bounded step, so a
///   single-threaded runtime stays live, the Update Source gets progress
///   (ADR 0004), and abandoning mid-transfer (ADR 0003) is
///   [`abandon`](Self::abandon) instead of waiting out a blocked call.
/// - **`Ready` means ready.** The device holds the complete payload and
///   `activate` may be called. `activate` in any other staging state is
///   an error.
pub trait Updatable {
    /// The error type of this device's update path.
    ///
    /// Bounded by [`core::error::Error`] so the orchestrator gets
    /// `Display` and a `source()` cause chain, not just a `Debug` dump.
    /// Error categories are implementation-defined.
    type Error: core::error::Error;

    /// Advances staging by one bounded step, pulling from `payload`.
    ///
    /// The first call from idle (fresh device, after [`Ready`], an error,
    /// or [`abandon`](Self::abandon)) starts a new transfer; the caller
    /// keeps polling with the same `payload` until [`Ready`] or an error.
    /// The implementor pulls at whatever offsets its transfer needs (a
    /// PLDM device requests its own chunks, including retransmits);
    /// `payload` must serve any in-range read.
    ///
    /// Generic for static dispatch; `?Sized` admits `&dyn PayloadSource`.
    /// This makes `Updatable` non-dyn-compatible, which the associated
    /// `Error` type effectively already did.
    ///
    /// [`Ready`]: StageProgress::Ready
    fn poll_stage(
        &mut self,
        payload: &(impl PayloadSource + ?Sized),
    ) -> Result<StageProgress, Self::Error>;

    /// Discards the in-progress transfer or staged, unactivated payload.
    ///
    /// Infallible: back to idle unconditionally. Cleanup a device needs
    /// (marking a half-written slot dirty) is the implementor's, deferred
    /// to the next staging if it must touch hardware.
    fn abandon(&mut self);

    /// Marks the staged image as the device's boot candidate
    /// (§5.3.1 step 5 — tentative; see the trait docs on commit).
    fn activate(&mut self) -> Result<(), Self::Error>;
}

/// What one [`Updatable::poll_stage`] step established.
///
/// Intentionally exhaustive (not `#[non_exhaustive]`): adding a state is a
/// breaking change, so every consumer handles it explicitly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StageProgress {
    /// Transfer ongoing; poll again. `done`/`total` bytes feed the Update
    /// Source's progress report — `done` is monotonic and below `total`.
    Transferring {
        /// Bytes the device holds so far.
        done: u64,
        /// Total payload bytes.
        total: u64,
    },
    /// The device holds the complete payload; `activate` may be called.
    Ready,
}

/// Chunked, random-access read seam [`Updatable::poll_stage`] pulls from.
///
/// The candidate payload is streamed and never RAM-resident (ADR 0008);
/// this is the window a device adapter reads it through. Where the bytes
/// live — frontend staging flash, a mapped blob, a test slice — stays
/// behind the source.
pub trait PayloadSource {
    /// Total payload length in bytes.
    fn len(&self) -> u64;

    /// True if the payload is empty.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Fills `buf` from `offset`. The read is exact: short fills are a
    /// fault, and `offset + buf.len()` beyond [`len`](Self::len) is out
    /// of range.
    fn read_at(&self, offset: u64, buf: &mut [u8]) -> Result<(), PayloadFault>;
}

/// Why a payload read failed — the one distinction retry policy needs.
///
/// No further detail crosses the seam (mirroring `BootWatch`): the source
/// logs the concrete cause while it is still in scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PayloadFault {
    /// The requested range is outside the payload — a caller bug, never
    /// retriable.
    OutOfRange,
    /// The backing storage failed the read — possibly transient; staging
    /// anew may succeed.
    Storage,
}

impl core::fmt::Display for PayloadFault {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            PayloadFault::OutOfRange => "payload read out of range",
            PayloadFault::Storage => "payload storage fault",
        })
    }
}

impl core::error::Error for PayloadFault {}

#[cfg(test)]
mod tests {
    use super::*;
    use core::error::Error as _;

    // A PayloadSource over a plain slice — the seam must be satisfiable
    // with no storage stack at all.
    struct SliceSource(&'static [u8]);

    impl PayloadSource for SliceSource {
        fn len(&self) -> u64 {
            self.0.len() as u64
        }

        fn read_at(&self, offset: u64, buf: &mut [u8]) -> Result<(), PayloadFault> {
            let start = usize::try_from(offset).map_err(|_| PayloadFault::OutOfRange)?;
            let end = start
                .checked_add(buf.len())
                .ok_or(PayloadFault::OutOfRange)?;
            buf.copy_from_slice(self.0.get(start..end).ok_or(PayloadFault::OutOfRange)?);
            Ok(())
        }
    }

    // An Updatable implemented against no HAL or transport — the contract
    // must be satisfiable from any stack (mock, IPC proxy, simulator).
    // Pulls two bytes per poll to exercise the resumable-transfer shape.
    struct MockDevice {
        staged: Vec<u8>,
        done: usize,
        ready: bool,
        active: bool,
    }

    impl MockDevice {
        fn idle() -> Self {
            MockDevice {
                staged: Vec::new(),
                done: 0,
                ready: false,
                active: false,
            }
        }
    }

    #[derive(Debug, PartialEq, Eq)]
    enum MockFault {
        Pull(PayloadFault),
        NothingStaged,
    }

    impl core::fmt::Display for MockFault {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            match self {
                MockFault::Pull(_) => f.write_str("staging pull failed"),
                MockFault::NothingStaged => f.write_str("nothing staged"),
            }
        }
    }

    impl core::error::Error for MockFault {
        fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
            match self {
                MockFault::Pull(fault) => Some(fault),
                MockFault::NothingStaged => None,
            }
        }
    }

    impl Updatable for MockDevice {
        type Error = MockFault;

        fn poll_stage(
            &mut self,
            payload: &(impl PayloadSource + ?Sized),
        ) -> Result<StageProgress, MockFault> {
            if self.ready {
                self.abandon(); // a poll after Ready starts a new transfer
            }
            let total = usize::try_from(payload.len()).unwrap();
            if self.done == 0 {
                self.staged = vec![0; total];
            }
            let end = (self.done + 2).min(total);
            if let Err(fault) = payload.read_at(self.done as u64, &mut self.staged[self.done..end])
            {
                self.abandon();
                return Err(MockFault::Pull(fault));
            }
            self.done = end;
            if self.done == total {
                self.ready = true;
                Ok(StageProgress::Ready)
            } else {
                Ok(StageProgress::Transferring {
                    done: self.done as u64,
                    total: total as u64,
                })
            }
        }

        fn abandon(&mut self) {
            self.staged = Vec::new();
            self.done = 0;
            self.ready = false;
        }

        fn activate(&mut self) -> Result<(), MockFault> {
            if !self.ready {
                return Err(MockFault::NothingStaged);
            }
            self.active = true;
            Ok(())
        }
    }

    /// Polls to completion — the orchestrator's staging loop shape.
    fn stage_all<D: Updatable>(dev: &mut D, payload: &impl PayloadSource) -> Result<(), D::Error> {
        loop {
            if let StageProgress::Ready = dev.poll_stage(payload)? {
                return Ok(());
            }
        }
    }

    #[test]
    fn contract_is_implementable_without_the_hal() {
        let mut dev = MockDevice::idle();

        stage_all(&mut dev, &SliceSource(b"image")).expect("staging failed");
        dev.activate().expect("activate failed");

        assert_eq!(dev.staged, b"image");
        assert!(dev.active);
    }

    #[test]
    fn progress_is_reportable_mid_transfer() {
        let mut dev = MockDevice::idle();
        let payload = SliceSource(b"image");

        assert_eq!(
            dev.poll_stage(&payload),
            Ok(StageProgress::Transferring { done: 2, total: 5 })
        );
        assert_eq!(
            dev.poll_stage(&payload),
            Ok(StageProgress::Transferring { done: 4, total: 5 })
        );
        assert_eq!(dev.poll_stage(&payload), Ok(StageProgress::Ready));
    }

    #[test]
    fn out_of_range_pull_aborts_staging() {
        struct Lying;

        // Claims more bytes than it can serve — the adapter's pull runs
        // past the real end and must surface the fault.
        impl PayloadSource for Lying {
            fn len(&self) -> u64 {
                8
            }

            fn read_at(&self, offset: u64, buf: &mut [u8]) -> Result<(), PayloadFault> {
                SliceSource(b"shrt").read_at(offset, buf)
            }
        }

        let mut dev = MockDevice::idle();

        let err = stage_all(&mut dev, &Lying).expect_err("expected the pull fault");

        assert_eq!(err, MockFault::Pull(PayloadFault::OutOfRange));
        // The cause chain carries the fault, per the Error bound.
        assert!(err.source().is_some());

        // Staging anew after a fault is always allowed.
        stage_all(&mut dev, &SliceSource(b"image")).expect("re-staging failed");
        dev.activate().expect("activate after re-staging failed");
    }

    #[test]
    fn abandon_returns_to_idle_and_discards() {
        let mut dev = MockDevice::idle();
        let payload = SliceSource(b"image");

        dev.poll_stage(&payload).expect("first step failed");
        dev.abandon();

        assert_eq!(dev.activate(), Err(MockFault::NothingStaged));
        // A fresh transfer starts from byte zero.
        assert_eq!(
            dev.poll_stage(&payload),
            Ok(StageProgress::Transferring { done: 2, total: 5 })
        );
    }

    #[test]
    fn activate_without_ready_is_an_error() {
        let mut dev = MockDevice::idle();

        let err = dev.activate().expect_err("expected nothing staged");

        assert_eq!(err.to_string(), "nothing staged");
    }
}
