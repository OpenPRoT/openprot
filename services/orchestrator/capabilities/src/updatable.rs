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
///   orchestrator policy. Post-write readback is no capability either:
///   it is the implementor's staging discipline (see `Ready` below).
/// - **Commit.** Activation is always tentative: it proposes the staged
///   image as the preferred boot target, never commits it. The commit
///   gate is the orchestrator's confirmed-boot flow (`BootConfirmed`
///   gating `SvnFloor::advance`, or the device committing internally);
///   there is no second slot-selection owner.
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
/// - **`Ready` means ready.** The device holds the complete payload,
///   verified to its archetype's discipline: a direct-flash adapter
///   reads written data back and re-verifies before reporting `Ready`,
///   a PLDM device runs its own verify step after the transfer:
///   `FdOps::verify` in pldm-lib, reported to the update agent as
///   `VerifyComplete`. A mismatch fails the step as
///   [`ReadbackMismatch`](UpdateError::ReadbackMismatch), kept apart
///   from [`Device`](UpdateError::Device) so a caller can retire a slot
///   that keeps mismatching. `activate` may be called; in any other
///   staging state it is an error.
pub trait Updatable {
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
    /// An empty payload is a caller bug: there is no image, so [`Ready`]
    /// must not be reachable vacuously. Implementations return
    /// [`EmptyPayload`](UpdateError::EmptyPayload) before touching the
    /// device or their own staging state.
    ///
    /// Dyn-compatible on purpose: a heterogeneous fleet (flash and PLDM
    /// devices side by side, the usual board) sits behind
    /// `&mut dyn Updatable`, so the payload is a trait object and the
    /// error is the erased [`UpdateError`]. A board preferring static
    /// dispatch wraps its devices in an enum and matches, as with
    /// `BootWatch`.
    ///
    /// [`Ready`]: StageProgress::Ready
    fn poll_stage(&mut self, payload: &dyn PayloadSource) -> Result<StageProgress, UpdateError>;

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
    fn activate(&mut self) -> Result<(), UpdateError>;
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
    /// The device holds the complete, verified payload; `activate` may
    /// be called.
    Ready,
}

/// Why an update operation failed, erased of device detail.
///
/// Carries the distinctions the orchestrator's policy needs and nothing
/// more (mirroring `BootWatch`): the adapter logs the concrete device
/// error while it is still in scope. Implements [`core::error::Error`],
/// so callers still get `Display` and a `source()` chain for the
/// payload fault.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateError {
    /// A payload read failed. Carries the read fault, so out-of-range
    /// (a bug on the pulling side) stays distinct from storage (maybe
    /// transient).
    Payload(PayloadReadError),
    /// The device failed or refused the step; staging anew may succeed.
    Device,
    /// Written data did not read back as written. Distinct from
    /// [`Device`](Self::Device) because the cause is narrower — the write
    /// path reported success and the storage still disagrees — so a
    /// caller can count it separately and retire a slot that keeps
    /// mismatching instead of retrying forever.
    ReadbackMismatch,
    /// `poll_stage` with an empty payload, a caller bug: an empty image
    /// must never stage to [`Ready`](StageProgress::Ready).
    EmptyPayload,
    /// `activate` without a staged, [`Ready`](StageProgress::Ready)
    /// payload — a caller bug.
    NothingStaged,
}

impl core::fmt::Display for UpdateError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            UpdateError::Payload(_) => f.write_str("staging pull failed"),
            UpdateError::Device => f.write_str("device failed the update step"),
            UpdateError::ReadbackMismatch => f.write_str("readback mismatch"),
            UpdateError::EmptyPayload => f.write_str("empty payload"),
            UpdateError::NothingStaged => f.write_str("nothing staged"),
        }
    }
}

impl core::error::Error for UpdateError {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            UpdateError::Payload(fault) => Some(fault),
            UpdateError::Device
            | UpdateError::ReadbackMismatch
            | UpdateError::EmptyPayload
            | UpdateError::NothingStaged => None,
        }
    }
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

    impl Updatable for MockDevice {
        fn poll_stage(
            &mut self,
            payload: &dyn PayloadSource,
        ) -> Result<StageProgress, UpdateError> {
            if payload.is_empty() {
                return Err(UpdateError::EmptyPayload);
            }
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
                return Err(UpdateError::Payload(fault));
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

        fn activate(&mut self) -> Result<(), UpdateError> {
            if !self.ready {
                return Err(UpdateError::NothingStaged);
            }
            self.active = true;
            Ok(())
        }
    }

    /// Polls to completion — the orchestrator's staging loop shape,
    /// written against the erased seam.
    fn stage_all(dev: &mut dyn Updatable, payload: &dyn PayloadSource) -> Result<(), UpdateError> {
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

        assert_eq!(err, UpdateError::Payload(PayloadReadError::OutOfRange));
        // The cause chain carries the fault, per the UpdateError impl.
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

        assert_eq!(dev.activate(), Err(UpdateError::NothingStaged));
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
            fn poll_stage(
                &mut self,
                payload: &dyn PayloadSource,
            ) -> Result<StageProgress, UpdateError> {
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

            fn activate(&mut self) -> Result<(), UpdateError> {
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
    // A finished transfer does not mean `Ready`: the device runs its own
    // verify first, one further step, the way a PLDM firmware device
    // enters VERIFY and reports the outcome in `VerifyComplete`.
    const PLDM_CHUNK: usize = 3;

    struct MockPldmDevice {
        staged: Vec<u8>,
        offset: usize,
        retransmitted: bool,
        // The whole image is transferred and its verify step is due.
        verifying: bool,
        ready: bool,
        active: bool,
    }

    impl MockPldmDevice {
        fn idle() -> Self {
            MockPldmDevice {
                staged: Vec::new(),
                offset: 0,
                retransmitted: false,
                verifying: false,
                ready: false,
                active: false,
            }
        }
    }

    impl Updatable for MockPldmDevice {
        fn poll_stage(
            &mut self,
            payload: &dyn PayloadSource,
        ) -> Result<StageProgress, UpdateError> {
            if payload.is_empty() {
                return Err(UpdateError::EmptyPayload);
            }
            if self.ready {
                self.abandon();
            }
            let total = usize::try_from(payload.len()).unwrap();
            if self.staged.len() != total {
                self.staged = vec![0; total];
            }
            // The device's own verify, the step that earns `Ready`. It
            // transfers nothing, so no chunk is requested.
            if self.verifying {
                self.verifying = false;
                self.ready = true;
                return Ok(StageProgress::Ready);
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
                .map_err(UpdateError::Payload)?;
            if request == self.offset {
                self.offset += len;
            }
            if self.offset == total {
                self.verifying = true;
            }
            Ok(StageProgress::Transferring {
                written: self.offset as u64,
                total: total as u64,
            })
        }

        fn abandon(&mut self) {
            self.staged = Vec::new();
            self.offset = 0;
            self.retransmitted = false;
            self.verifying = false;
            self.ready = false;
        }

        fn activate(&mut self) -> Result<(), UpdateError> {
            if !self.ready {
                return Err(UpdateError::NothingStaged);
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
        // The short last chunk completes the transfer. Still not `Ready`:
        // the device's verify has not run.
        assert_eq!(
            dev.poll_stage(&payload),
            Ok(StageProgress::Transferring {
                written: total,
                total
            })
        );
        // The verify step earns `Ready`.
        assert_eq!(dev.poll_stage(&payload), Ok(StageProgress::Ready));

        dev.activate().expect("activate failed");
        assert_eq!(dev.staged, b"chunked");
    }

    // A direct-flash adapter, the erase-before-write shape: one step is
    // one flash operation, erasing the next sector, programming the next
    // page, or reading a written page back. Erase and write steps hold
    // `written` still; only a page that passed readback advances it, so
    // `written` counts verified bytes. This is the readback discipline the
    // `Ready` contract prescribes for direct-flash devices.
    const FLASH_PAGE: usize = 2; // bytes programmed per write step
    const FLASH_SECTOR: usize = 4; // bytes erased per erase step
    const FLASH_SECTORS: usize = 2; // sectors in the slot

    // What a page written last step must read back as.
    struct ExpectedReadback {
        start: usize,
        expected: Vec<u8>,
    }

    struct MockFlashDevice {
        slot: [u8; FLASH_SECTORS * FLASH_SECTOR],
        erased: [bool; FLASH_SECTORS],
        pending: Option<ExpectedReadback>,
        count: usize,
        ready: bool,
        active: bool,
        // Test knob: corrupt the next written page so its readback fails.
        corrupt_next_write: bool,
    }

    impl MockFlashDevice {
        fn idle() -> Self {
            MockFlashDevice {
                slot: [0; FLASH_SECTORS * FLASH_SECTOR],
                erased: [false; FLASH_SECTORS],
                pending: None,
                count: 0,
                ready: false,
                active: false,
                corrupt_next_write: false,
            }
        }
    }

    impl Updatable for MockFlashDevice {
        fn poll_stage(
            &mut self,
            payload: &dyn PayloadSource,
        ) -> Result<StageProgress, UpdateError> {
            if payload.is_empty() {
                return Err(UpdateError::EmptyPayload);
            }
            if self.ready {
                self.abandon();
            }
            let total = usize::try_from(payload.len()).unwrap();
            // An oversized payload is a device error, not a caller bug:
            // rejected before anything is erased or written.
            if total > self.slot.len() {
                return Err(UpdateError::Device);
            }
            // Readback: re-verify the page written last step. Only now
            // does the page count as staged.
            if let Some(ExpectedReadback { start, expected }) = self.pending.take() {
                if self.slot[start..start + expected.len()] != expected[..] {
                    self.abandon();
                    return Err(UpdateError::ReadbackMismatch);
                }
                self.count = start + expected.len();
                return if self.count == total {
                    self.ready = true;
                    Ok(StageProgress::Ready)
                } else {
                    Ok(StageProgress::Transferring {
                        written: self.count as u64,
                        total: total as u64,
                    })
                };
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
            let mut page = vec![0; end - self.count];
            payload
                .read_at(self.count as u64, &mut page)
                .map_err(UpdateError::Payload)?;
            self.slot[self.count..end].copy_from_slice(&page);
            if self.corrupt_next_write {
                self.corrupt_next_write = false;
                self.slot[self.count] ^= 0xff;
            }
            self.pending = Some(ExpectedReadback {
                start: self.count,
                expected: page,
            });
            Ok(StageProgress::Transferring {
                written: self.count as u64,
                total: total as u64,
            })
        }

        fn abandon(&mut self) {
            self.erased = [false; FLASH_SECTORS];
            self.pending = None;
            self.count = 0;
            self.ready = false;
        }

        fn activate(&mut self) -> Result<(), UpdateError> {
            if !self.ready {
                return Err(UpdateError::NothingStaged);
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

        // `written` after each step; it advances only on a passed readback.
        let expected = [
            0,             // erase sector 0
            0,             // program page 0
            page,          // page 0 readback passed
            page,          // program page 1
            sector,        // page 1 readback passed
            sector,        // erase sector 1
            sector,        // program page 2
            sector + page, // page 2 readback passed
            sector + page, // program page 3
        ];
        for written in expected {
            assert_eq!(
                dev.poll_stage(&payload),
                Ok(StageProgress::Transferring { written, total })
            );
        }
        // The final readback completes staging.
        assert_eq!(dev.poll_stage(&payload), Ok(StageProgress::Ready));

        dev.activate().expect("activate failed");
        assert_eq!(&dev.slot, b"8 bytes!");
    }

    #[test]
    fn a_heterogeneous_fleet_stages_through_one_seam() {
        // The dyn-compatibility use case: the usual board mixes
        // archetypes, and the fleet view must not force one concrete
        // type per board.
        let mut flash = MockFlashDevice::idle();
        let mut pldm = MockPldmDevice::idle();
        let flash_payload = SliceSource(b"8 bytes!");
        let pldm_payload = SliceSource(b"chunked");

        let fleet: [(&mut dyn Updatable, &dyn PayloadSource); 2] =
            [(&mut flash, &flash_payload), (&mut pldm, &pldm_payload)];
        for (dev, payload) in fleet {
            stage_all(dev, payload).expect("staging failed");
            dev.activate().expect("activate failed");
        }

        assert_eq!(&flash.slot, b"8 bytes!");
        assert_eq!(pldm.staged, b"chunked");
    }

    #[test]
    fn a_readback_mismatch_is_a_staging_error() {
        let mut dev = MockFlashDevice::idle();
        dev.corrupt_next_write = true;
        let payload = SliceSource(b"8 bytes!");

        let err = stage_all(&mut dev, &payload).expect_err("expected the readback mismatch");
        assert_eq!(err, UpdateError::ReadbackMismatch);

        // Staging anew after the fault is allowed and verifies clean.
        stage_all(&mut dev, &payload).expect("re-staging failed");
        assert_eq!(&dev.slot, b"8 bytes!");
        dev.activate().expect("activate after re-staging failed");
    }

    #[test]
    fn a_payload_beyond_the_slot_is_rejected_before_any_flash_op() {
        let mut dev = MockFlashDevice::idle();
        let payload = SliceSource(b"ninebytes");

        assert_eq!(dev.poll_stage(&payload), Err(UpdateError::Device));
        assert!(!dev.erased[0], "nothing was erased");
    }

    #[test]
    fn an_empty_payload_is_rejected_and_never_ready() {
        // Without the guard, the `written == total` termination check
        // holds vacuously on the first poll and an image-less staging
        // reaches Ready.
        let mut plain = MockDevice::idle();
        let mut pldm = MockPldmDevice::idle();
        let mut flash = MockFlashDevice::idle();

        let fleet: [&mut dyn Updatable; 3] = [&mut plain, &mut pldm, &mut flash];
        for dev in fleet {
            assert_eq!(
                dev.poll_stage(&SliceSource(b"")),
                Err(UpdateError::EmptyPayload)
            );
            assert_eq!(dev.activate(), Err(UpdateError::NothingStaged));
        }
    }

    #[test]
    fn a_rejected_empty_poll_leaves_a_ready_staging_intact() {
        let mut dev = MockDevice::idle();
        stage_all(&mut dev, &SliceSource(b"image")).expect("staging failed");

        // The guard rejects before touching staging state, so this
        // caller bug cannot destroy a completed staging.
        assert_eq!(
            dev.poll_stage(&SliceSource(b"")),
            Err(UpdateError::EmptyPayload)
        );
        dev.activate()
            .expect("activate after the rejected poll failed");
        assert_eq!(dev.staged, b"image");
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
