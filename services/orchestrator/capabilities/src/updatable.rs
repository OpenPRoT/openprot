// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! The [`Updatable`] update capability contract.

/// Update capability: stage a payload on one managed device and mark the
/// staged image as its boot candidate.
///
/// The two operations: write to the inactive slot only, then update
/// slot metadata to prefer the new image. Universal across device
/// archetypes: a direct-flash adapter writes the payload into the
/// device's inactive slot itself, a PLDM adapter drives the device's
/// own transfer. Slot identity never crosses the
/// seam — which slot is inactive is device state, so
/// [`activate`](Self::activate) can only ever mean "the image the last
/// completed staging delivered".
///
/// The caller is the platform driver. The state machine only decides
/// that an update runs; the driver owns the adapter, polls the transfer
/// on its own clock, and reports the outcome back as an event. The
/// update source that delivered the payload fills what backs
/// [`PayloadSource`] and never drives the device.
///
/// What this trait deliberately does not claim:
///
/// - **Verification** runs on the candidate before staging as
///   orchestrator policy; post-write read-back is the optional
///   `ReadBack` capability.
/// - **Commit.** Activation is always tentative: it proposes the staged
///   image as the preferred boot target, never commits it. The commit
///   gate is the optional `TrialBoot` capability, which resolves what
///   `activate` proposed; there is no second slot-selection owner.
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
///   [`poll_stage`](Self::poll_stage) call does one step, at most one
///   payload pull plus one device transaction, and returns without
///   waiting on the device. A busy device is not an error: the step
///   returns [`Transferring`](StageProgress::Transferring) with `written`
///   unchanged. So a single-threaded runtime stays live, the update
///   source gets progress, and abandoning mid-transfer is
///   [`abandon`](Self::abandon) instead of waiting out a blocked call.
///   Liveness policy stays with the caller: it watches `written` and
///   abandons a transfer that stalls too long, on its own clock.
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

    /// Advances staging by one step, pulling from `payload`.
    ///
    /// A step is bounded: at most one pull from `payload` and at most one
    /// device transaction, never a wait for device progress. Returning
    /// with `written` unchanged is legal (busy device, PLDM retransmit) and
    /// is not an error.
    ///
    /// The first call from idle (fresh device, after [`Ready`], an error,
    /// or [`abandon`](Self::abandon)) starts a new transfer; the caller
    /// keeps polling with the same `payload` until [`Ready`] or an error.
    /// The implementor pulls at whatever offsets its transfer needs (a
    /// PLDM device requests its own chunks, including retransmits);
    /// `payload` must serve any in-range read.
    ///
    /// Generic for static dispatch, which makes `Updatable` itself
    /// non-dyn-compatible; the associated `Error` type effectively
    /// already did.
    ///
    /// [`Ready`]: StageProgress::Ready
    fn poll_stage(&mut self, payload: &impl PayloadSource) -> Result<StageProgress, Self::Error>;

    /// Discards the in-progress transfer or staged, unactivated payload.
    ///
    /// Infallible: back to idle unconditionally. Cleanup a device needs
    /// (marking a half-written slot dirty) is the implementor's, deferred
    /// to the next staging if it must touch hardware.
    fn abandon(&mut self);

    /// Marks the staged image as the device's boot candidate
    /// (tentative; see the trait docs on commit).
    ///
    /// [`Ready`] persists until a new staging starts or
    /// [`abandon`](Self::abandon), and `activate` while [`Ready`] is
    /// idempotent: a repeated call succeeds.
    ///
    /// [`Ready`]: StageProgress::Ready
    fn activate(&mut self) -> Result<(), Self::Error>;
}

/// What one [`Updatable::poll_stage`] step established.
///
/// Intentionally exhaustive (not `#[non_exhaustive]`): adding a state is a
/// breaking change, so every consumer handles it explicitly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StageProgress {
    /// Transfer ongoing; poll again. `written`/`total` bytes feed the Update
    /// Source's progress report.
    Transferring {
        /// Bytes written to the device so far. Monotonic and below `total`,
        /// but free to hold still across calls (busy device,
        /// retransmit); a caller deciding when a transfer has stalled
        /// keys on this value.
        written: u64,
        /// Total payload bytes.
        total: u64,
    },
    /// The device holds the complete payload; `activate` may be called.
    Ready,
}

/// Chunked, random-access read seam [`Updatable::poll_stage`] pulls from.
///
/// The candidate payload is streamed and never RAM-resident;
/// this is the window a device adapter reads it through. Where the bytes
/// live — frontend staging flash, a mapped blob, a test slice — stays
/// behind the source.
pub trait PayloadSource {
    /// Total payload length in bytes, constant for the lifetime of the
    /// source: adapters allocate staging buffers from it and treat the
    /// transfer as complete once this many bytes are written.
    fn len(&self) -> u64;

    /// True if the payload is empty.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Fills `buf` from `offset`. The read is exact: short fills are a
    /// fault, and `offset + buf.len()` beyond [`len`](Self::len) is out
    /// of range. There are no partial reads: the length is known up
    /// front, so a short read can only mean the source cannot serve
    /// what `len` promised, and a partial-read API would put a retry
    /// loop into every adapter.
    fn read_at(&self, offset: u64, buf: &mut [u8]) -> Result<(), PayloadReadError>;
}

/// Why a payload read failed — the one distinction retry policy needs.
///
/// No further detail crosses the seam (mirroring `BootWatch`): the source
/// logs the concrete cause while it is still in scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PayloadReadError {
    /// The requested range is outside the payload — a caller bug, never
    /// retriable.
    OutOfRange,
    /// The backing storage failed the read — possibly transient; staging
    /// anew may succeed.
    Storage,
}

impl core::fmt::Display for PayloadReadError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            PayloadReadError::OutOfRange => "payload read out of range",
            PayloadReadError::Storage => "payload storage fault",
        })
    }
}

impl core::error::Error for PayloadReadError {}

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

        fn read_at(&self, offset: u64, buf: &mut [u8]) -> Result<(), PayloadReadError> {
            let start = usize::try_from(offset).map_err(|_| PayloadReadError::OutOfRange)?;
            let end = start
                .checked_add(buf.len())
                .ok_or(PayloadReadError::OutOfRange)?;
            buf.copy_from_slice(self.0.get(start..end).ok_or(PayloadReadError::OutOfRange)?);
            Ok(())
        }
    }

    // An Updatable implemented against no HAL or transport — the contract
    // must be satisfiable from any stack (mock, IPC proxy, simulator).
    // Pulls MOCK_STEP bytes per poll to exercise the resumable-transfer
    // shape.
    const MOCK_STEP: usize = 2;

    struct MockDevice {
        staged: Vec<u8>,
        count: usize,
        ready: bool,
        active: bool,
    }

    impl MockDevice {
        fn idle() -> Self {
            MockDevice {
                staged: Vec::new(),
                count: 0,
                ready: false,
                active: false,
            }
        }
    }

    #[derive(Debug, PartialEq, Eq)]
    enum MockFault {
        Pull(PayloadReadError),
        NothingStaged,
        ExceedsSlot,
    }

    impl core::fmt::Display for MockFault {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            match self {
                MockFault::Pull(_) => f.write_str("staging pull failed"),
                MockFault::NothingStaged => f.write_str("nothing staged"),
                MockFault::ExceedsSlot => f.write_str("payload exceeds the slot"),
            }
        }
    }

    impl core::error::Error for MockFault {
        fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
            match self {
                MockFault::Pull(fault) => Some(fault),
                MockFault::NothingStaged | MockFault::ExceedsSlot => None,
            }
        }
    }

    impl Updatable for MockDevice {
        type Error = MockFault;

        fn poll_stage(&mut self, payload: &impl PayloadSource) -> Result<StageProgress, MockFault> {
            if self.ready {
                self.abandon(); // a poll after Ready starts a new transfer
            }
            let total = usize::try_from(payload.len()).unwrap();
            if self.count == 0 {
                self.staged = vec![0; total];
            }
            let end = (self.count + MOCK_STEP).min(total);
            if let Err(fault) =
                payload.read_at(self.count as u64, &mut self.staged[self.count..end])
            {
                self.abandon();
                return Err(MockFault::Pull(fault));
            }
            self.count = end;
            if self.count == total {
                self.ready = true;
                Ok(StageProgress::Ready)
            } else {
                Ok(StageProgress::Transferring {
                    written: self.count as u64,
                    total: total as u64,
                })
            }
        }

        fn abandon(&mut self) {
            self.staged = Vec::new();
            self.count = 0;
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
    fn contract_is_implementable() {
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

        let mut pulled = MOCK_STEP as u64;
        while pulled < payload.len() {
            assert_eq!(
                dev.poll_stage(&payload),
                Ok(StageProgress::Transferring {
                    written: pulled,
                    total: payload.len(),
                })
            );
            pulled += MOCK_STEP as u64;
        }
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

            fn read_at(&self, offset: u64, buf: &mut [u8]) -> Result<(), PayloadReadError> {
                SliceSource(b"shrt").read_at(offset, buf)
            }
        }

        let mut dev = MockDevice::idle();

        let err = stage_all(&mut dev, &Lying).expect_err("expected the pull fault");

        assert_eq!(err, MockFault::Pull(PayloadReadError::OutOfRange));
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
            Ok(StageProgress::Transferring {
                written: MOCK_STEP as u64,
                total: payload.len(),
            })
        );
    }

    #[test]
    fn staging_anew_discards_the_previous_payload() {
        let mut dev = MockDevice::idle();

        stage_all(&mut dev, &SliceSource(b"first")).expect("first staging failed");
        // No abandon: a fresh staging from Ready replaces the payload.
        stage_all(&mut dev, &SliceSource(b"image")).expect("re-staging failed");
        dev.activate().expect("activate failed");

        assert_eq!(dev.staged, b"image");
    }

    #[test]
    fn a_busy_step_is_not_an_error_and_holds_written_still() {
        // A device that needs a bus turn between steps: every other poll
        // is busy and returns with `written` unchanged. The contract says
        // that is a legal step, so the plain polling loop still finishes.
        struct BusyDevice {
            inner: MockDevice,
            busy: bool,
        }

        impl Updatable for BusyDevice {
            type Error = MockFault;

            fn poll_stage(
                &mut self,
                payload: &impl PayloadSource,
            ) -> Result<StageProgress, MockFault> {
                self.busy = !self.busy;
                if self.busy {
                    return Ok(StageProgress::Transferring {
                        written: self.inner.count as u64,
                        total: payload.len(),
                    });
                }
                self.inner.poll_stage(payload)
            }

            fn abandon(&mut self) {
                self.inner.abandon();
            }

            fn activate(&mut self) -> Result<(), MockFault> {
                self.inner.activate()
            }
        }

        let mut dev = BusyDevice {
            inner: MockDevice::idle(),
            busy: false,
        };
        let payload = SliceSource(b"image");

        // The caller-side stall accounting the contract prescribes:
        // count polls since `written` last moved, abandon past a budget.
        let mut last_written = 0;
        let mut stalled_polls = 0;
        loop {
            match dev.poll_stage(&payload).expect("staging failed") {
                StageProgress::Ready => break,
                StageProgress::Transferring { written, .. } => {
                    if written > last_written {
                        last_written = written;
                        stalled_polls = 0;
                    } else {
                        stalled_polls += 1;
                    }
                    assert!(stalled_polls < 3, "transfer stalled");
                }
            }
        }

        dev.activate().expect("activate failed");
        assert_eq!(dev.inner.staged, b"image");
    }

    // An adapter for a device that drives its own transfer, the PLDM
    // shape: one fixed-size chunk request per step, at offsets the device
    // picks, including a retransmit. Out-of-range reads are a fault, so
    // the adapter clamps the device's last request to the payload end.
    const PLDM_CHUNK: usize = 3;

    struct MockPldmDevice {
        staged: Vec<u8>,
        offset: usize,
        retransmitted: bool,
        ready: bool,
        active: bool,
    }

    impl MockPldmDevice {
        fn idle() -> Self {
            MockPldmDevice {
                staged: Vec::new(),
                offset: 0,
                retransmitted: false,
                ready: false,
                active: false,
            }
        }
    }

    impl Updatable for MockPldmDevice {
        type Error = MockFault;

        fn poll_stage(&mut self, payload: &impl PayloadSource) -> Result<StageProgress, MockFault> {
            if self.ready {
                self.abandon();
            }
            let total = usize::try_from(payload.len()).unwrap();
            if self.staged.len() != total {
                self.staged = vec![0; total];
            }
            // The device re-requests the previous chunk once mid-transfer.
            let request = if self.offset == 2 * PLDM_CHUNK && !self.retransmitted {
                self.retransmitted = true;
                self.offset - PLDM_CHUNK
            } else {
                self.offset
            };
            let len = PLDM_CHUNK.min(total - request);
            payload
                .read_at(request as u64, &mut self.staged[request..request + len])
                .map_err(MockFault::Pull)?;
            if request == self.offset {
                self.offset += len;
            }
            if self.offset == total {
                self.ready = true;
                Ok(StageProgress::Ready)
            } else {
                Ok(StageProgress::Transferring {
                    written: self.offset as u64,
                    total: total as u64,
                })
            }
        }

        fn abandon(&mut self) {
            self.staged = Vec::new();
            self.offset = 0;
            self.retransmitted = false;
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

    #[test]
    fn a_pldm_shaped_device_fits_the_seam() {
        let mut dev = MockPldmDevice::idle();
        // Two full chunks, then a short one.
        let payload = SliceSource(b"chunked");
        let total = payload.len();
        let chunk = PLDM_CHUNK as u64;

        assert_eq!(
            dev.poll_stage(&payload),
            Ok(StageProgress::Transferring {
                written: chunk,
                total
            })
        );
        assert_eq!(
            dev.poll_stage(&payload),
            Ok(StageProgress::Transferring {
                written: 2 * chunk,
                total,
            })
        );
        // The retransmit step pulls again but holds `written` still.
        assert_eq!(
            dev.poll_stage(&payload),
            Ok(StageProgress::Transferring {
                written: 2 * chunk,
                total,
            })
        );
        assert_eq!(dev.poll_stage(&payload), Ok(StageProgress::Ready));

        dev.activate().expect("activate failed");
        assert_eq!(dev.staged, b"chunked");
    }

    // A direct-flash adapter, the erase-before-write shape: one step is
    // one flash operation, either erasing the next sector or programming
    // the next page. Erase steps pull nothing and hold `written` still.
    const FLASH_PAGE: usize = 2; // bytes programmed per write step
    const FLASH_SECTOR: usize = 4; // bytes erased per erase step
    const FLASH_SECTORS: usize = 2; // sectors in the slot

    struct MockFlashDevice {
        slot: [u8; FLASH_SECTORS * FLASH_SECTOR],
        erased: [bool; FLASH_SECTORS],
        count: usize,
        ready: bool,
        active: bool,
    }

    impl MockFlashDevice {
        fn idle() -> Self {
            MockFlashDevice {
                slot: [0; FLASH_SECTORS * FLASH_SECTOR],
                erased: [false; FLASH_SECTORS],
                count: 0,
                ready: false,
                active: false,
            }
        }
    }

    impl Updatable for MockFlashDevice {
        type Error = MockFault;

        fn poll_stage(&mut self, payload: &impl PayloadSource) -> Result<StageProgress, MockFault> {
            if self.ready {
                self.abandon();
            }
            let total = usize::try_from(payload.len()).unwrap();
            // An oversized payload is a device error, not a caller bug:
            // rejected before anything is erased or written.
            if total > self.slot.len() {
                return Err(MockFault::ExceedsSlot);
            }
            let sector = self.count / FLASH_SECTOR;
            if !self.erased[sector] {
                self.slot[sector * FLASH_SECTOR..(sector + 1) * FLASH_SECTOR].fill(0xff);
                self.erased[sector] = true;
                return Ok(StageProgress::Transferring {
                    written: self.count as u64,
                    total: total as u64,
                });
            }
            let end = (self.count + FLASH_PAGE).min(total);
            payload
                .read_at(self.count as u64, &mut self.slot[self.count..end])
                .map_err(MockFault::Pull)?;
            self.count = end;
            if self.count == total {
                self.ready = true;
                Ok(StageProgress::Ready)
            } else {
                Ok(StageProgress::Transferring {
                    written: self.count as u64,
                    total: total as u64,
                })
            }
        }

        fn abandon(&mut self) {
            self.erased = [false; FLASH_SECTORS];
            self.count = 0;
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

    #[test]
    fn a_flash_shaped_device_fits_the_seam() {
        let mut dev = MockFlashDevice::idle();
        // Exactly fills the slot: FLASH_SECTORS sectors, two pages each.
        let payload = SliceSource(b"8 bytes!");
        let total = payload.len();
        let page = FLASH_PAGE as u64;
        let sector = FLASH_SECTOR as u64;
        assert_eq!(total as usize, FLASH_SECTORS * FLASH_SECTOR);

        // Erase sector 0: a step with no pull, `written` holds still.
        assert_eq!(
            dev.poll_stage(&payload),
            Ok(StageProgress::Transferring { written: 0, total })
        );
        assert_eq!(
            dev.poll_stage(&payload),
            Ok(StageProgress::Transferring {
                written: page,
                total
            })
        );
        assert_eq!(
            dev.poll_stage(&payload),
            Ok(StageProgress::Transferring {
                written: sector,
                total
            })
        );
        // Erase sector 1.
        assert_eq!(
            dev.poll_stage(&payload),
            Ok(StageProgress::Transferring {
                written: sector,
                total
            })
        );
        assert_eq!(
            dev.poll_stage(&payload),
            Ok(StageProgress::Transferring {
                written: sector + page,
                total,
            })
        );
        assert_eq!(dev.poll_stage(&payload), Ok(StageProgress::Ready));

        dev.activate().expect("activate failed");
        assert_eq!(&dev.slot, b"8 bytes!");
    }

    #[test]
    fn a_payload_beyond_the_slot_is_rejected_before_any_flash_op() {
        let mut dev = MockFlashDevice::idle();
        let payload = SliceSource(b"ninebytes");

        assert_eq!(dev.poll_stage(&payload), Err(MockFault::ExceedsSlot));
        assert!(!dev.erased[0], "nothing was erased");
    }

    #[test]
    fn activate_is_idempotent_while_ready() {
        let mut dev = MockDevice::idle();
        stage_all(&mut dev, &SliceSource(b"image")).expect("staging failed");

        dev.activate().expect("first activate failed");
        dev.activate().expect("repeated activate failed");
    }

    #[test]
    fn activate_without_ready_is_an_error() {
        let mut dev = MockDevice::idle();

        let err = dev.activate().expect_err("expected nothing staged");

        assert_eq!(err.to_string(), "nothing staged");
    }
}
