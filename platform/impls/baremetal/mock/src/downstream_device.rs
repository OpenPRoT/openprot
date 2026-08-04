// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Mock downstream device: one managed device as the eRoT sees it — a reset
//! line in, a boot-complete line out, A/B firmware slots, a staging region,
//! and a slot-selection store. All handles observe the same device state,
//! so tests can bind real adapters and assert only on externally visible
//! outcomes.

use core::cell::RefCell;
use core::num::NonZero;

use fwmanager_api::SlotControl;
use hal_flash::{Flash, FlashAddress};
use openprot_hal_blocking::gpio_port::{
    GpioError, GpioErrorKind, GpioErrorType, GpioPort, PinMask,
};
use openprot_hal_blocking::system_control::{ErrorType, ResetControl};
use util_types::PowerOf2Usize;

use crate::system_control::MockSystemControlError;

/// Flash size of the mock device (per slot and for the staging region).
pub const MOCK_DEVICE_FLASH_SIZE: usize = 256;

struct DeviceState {
    in_reset: bool,
    /// Boot-complete polls after release until the device asserts ready;
    /// `None` means the device never comes up.
    boots_after: Option<usize>,
    polls_since_release: usize,
    /// Hardware latch on the boot-complete line. Cleared by the reset line,
    /// as the platform wiring requires.
    ready_latched: bool,
    /// The device's two firmware slots (A = 0, B = 1).
    slots: [[u8; MOCK_DEVICE_FLASH_SIZE]; 2],
    /// Staging area an update is written to before activation.
    staging: [u8; MOCK_DEVICE_FLASH_SIZE],
    /// The slot the device boots by default; survives resets.
    committed_slot: usize,
    /// Slot armed for a trial boot. One-shot as a boot selector (consumed
    /// by the release that boots it); promotable until commit/rollback.
    trial_slot: Option<usize>,
    /// Whether the armed trial was already consumed by a boot.
    trial_consumed: bool,
    flash_reads: usize,
    /// Recorded at the first release: had the device's flash been read by
    /// then? Lets tests prove verification preceded release.
    release_preceded_by_flash_read: Option<bool>,
    /// The slot the device booted at its most recent release.
    last_boot_slot: Option<usize>,
    /// Program operations that touched a *bootable slot* while the device
    /// was running — must stay zero in a correct flow. Staging writes
    /// don't count.
    programs_while_running: usize,
    /// Recorded at the first commit: had the device reported boot-complete
    /// by then? Proves commit waited for boot confirmation.
    commit_preceded_by_ready: Option<bool>,
    commit_count: usize,
    rollback_count: usize,
    reset_fault: Option<MockSystemControlError>,
    gpio_fault: Option<GpioErrorKind>,
    /// When set, every flash program operation fails, simulating a device
    /// whose firmware write path is broken.
    program_fault: bool,
}

impl DeviceState {
    /// One boot-complete sample: a running device asserts the line once it
    /// has finished booting; the latch holds it until the next reset.
    fn sample_ready(&mut self) -> bool {
        if !self.in_reset {
            self.polls_since_release += 1;
            if let Some(n) = self.boots_after
                && self.polls_since_release > n
            {
                self.ready_latched = true;
            }
        }
        self.ready_latched
    }

    /// The slot the next release would boot: an armed, not-yet-consumed
    /// trial wins, otherwise the committed slot.
    fn boot_slot(&self) -> usize {
        match self.trial_slot {
            Some(trial) if !self.trial_consumed => trial,
            _ => self.committed_slot,
        }
    }
}

/// One simulated downstream device (for example a BMC), initially held in
/// reset with erased flash.
pub struct MockDownstreamDevice {
    state: RefCell<DeviceState>,
}

impl MockDownstreamDevice {
    /// A device that asserts boot-complete `boots_after` status polls after
    /// its release from reset; `None` builds a device that never comes up.
    pub fn held_in_reset(boots_after: Option<usize>) -> Self {
        Self {
            state: RefCell::new(DeviceState {
                in_reset: true,
                boots_after,
                polls_since_release: 0,
                ready_latched: false,
                slots: [[0xFF; MOCK_DEVICE_FLASH_SIZE]; 2],
                staging: [0xFF; MOCK_DEVICE_FLASH_SIZE],
                committed_slot: 0,
                trial_slot: None,
                trial_consumed: false,
                flash_reads: 0,
                release_preceded_by_flash_read: None,
                last_boot_slot: None,
                programs_while_running: 0,
                commit_preceded_by_ready: None,
                commit_count: 0,
                rollback_count: 0,
                reset_fault: None,
                gpio_fault: None,
                program_fault: false,
            }),
        }
    }

    /// Writes `image` to the start of the device's current boot slot.
    ///
    /// # Panics
    ///
    /// Panics if `image` exceeds [`MOCK_DEVICE_FLASH_SIZE`].
    pub fn load_firmware(&self, image: &[u8]) {
        let mut state = self.state.borrow_mut();
        let slot = state.boot_slot();
        state.slots[slot][..image.len()].copy_from_slice(image);
    }

    /// Latches the boot-complete line, simulating evidence left over from a
    /// previous boot cycle.
    pub fn latch_ready(&self) {
        self.state.borrow_mut().ready_latched = true;
    }

    /// Makes every reset-line operation fail with `err`.
    pub fn inject_reset_fault(&self, err: MockSystemControlError) {
        self.state.borrow_mut().reset_fault = Some(err);
    }

    /// Makes every boot-complete sample fail with `kind`.
    pub fn inject_gpio_fault(&self, kind: GpioErrorKind) {
        self.state.borrow_mut().gpio_fault = Some(kind);
    }

    /// The device's reset line, for the eRoT's reset controller binding.
    pub fn reset_line(&self) -> MockDeviceResetLine<'_> {
        MockDeviceResetLine { state: &self.state }
    }

    /// The device's boot-complete line, read back as `pin` of a GPIO input
    /// bank.
    pub fn ready_line(&self, pin: MockPinMask) -> MockDeviceReadyLine<'_> {
        MockDeviceReadyLine {
            state: &self.state,
            pin,
        }
    }

    /// The device's firmware flash, as the eRoT's interposer sees it:
    /// reads and writes land in the slot the next release would boot.
    pub fn flash(&self) -> MockDeviceFlash<'_> {
        MockDeviceFlash {
            state: &self.state,
            region: MockRegion::BootSlot,
        }
    }

    /// A specific flash region of the device. Accesses through this handle
    /// don't count toward [`Self::flash_reads`] — that instrumentation
    /// proves boot-image verification, not update plumbing.
    pub fn region_flash(&self, region: MockRegion) -> MockDeviceFlash<'_> {
        MockDeviceFlash {
            state: &self.state,
            region,
        }
    }

    /// The device's slot-selection store, for the eRoT's [`SlotControl`]
    /// binding.
    pub fn slot_store(&self) -> MockDeviceSlotStore<'_> {
        MockDeviceSlotStore { state: &self.state }
    }

    /// Re-script when the device asserts boot-complete after its next
    /// release; `None` makes it never come up. Lets one test cover several
    /// boot cycles with different outcomes.
    pub fn set_boots_after(&self, boots_after: Option<usize>) {
        self.state.borrow_mut().boots_after = boots_after;
    }

    /// Makes every flash program operation fail, simulating a device whose
    /// firmware write path is broken.
    pub fn inject_program_fault(&self) {
        self.state.borrow_mut().program_fault = true;
    }

    pub fn is_in_reset(&self) -> bool {
        self.state.borrow().in_reset
    }

    pub fn polls_since_release(&self) -> usize {
        self.state.borrow().polls_since_release
    }

    pub fn flash_reads(&self) -> usize {
        self.state.borrow().flash_reads
    }

    /// `Some(true)` if the device's flash had been read before its first
    /// release, `Some(false)` if not, `None` if it was never released.
    pub fn release_preceded_by_flash_read(&self) -> Option<bool> {
        self.state.borrow().release_preceded_by_flash_read
    }

    /// The slot committed as the device's default boot selection.
    pub fn committed_slot(&self) -> usize {
        self.state.borrow().committed_slot
    }

    /// The slot the device booted at its most recent release, `None` if it
    /// was never released.
    pub fn last_boot_slot(&self) -> Option<usize> {
        self.state.borrow().last_boot_slot
    }

    /// Program operations that touched a bootable slot while the device was
    /// running — nonzero means firmware was written under a live device.
    /// Staging writes are exempt.
    pub fn programs_while_running(&self) -> usize {
        self.state.borrow().programs_while_running
    }

    /// `Some(true)` if the device had reported boot-complete before its
    /// first commit, `Some(false)` if not, `None` if nothing was committed.
    pub fn commit_preceded_by_ready(&self) -> Option<bool> {
        self.state.borrow().commit_preceded_by_ready
    }

    pub fn commit_count(&self) -> usize {
        self.state.borrow().commit_count
    }

    pub fn rollback_count(&self) -> usize {
        self.state.borrow().rollback_count
    }

    /// The raw content of `slot`, for image-landed-where-expected checks.
    pub fn slot_content(&self, slot: usize) -> [u8; MOCK_DEVICE_FLASH_SIZE] {
        self.state.borrow().slots[slot]
    }
}

/// The device's reset line. The line selection already happened when this
/// handle was made, so the `ResetId` is ignored.
pub struct MockDeviceResetLine<'d> {
    state: &'d RefCell<DeviceState>,
}

impl ErrorType for MockDeviceResetLine<'_> {
    type Error = MockSystemControlError;
}

impl ResetControl for MockDeviceResetLine<'_> {
    type ResetId = u8;

    fn reset_assert(&mut self, _: &u8) -> Result<(), MockSystemControlError> {
        let mut state = self.state.borrow_mut();
        if let Some(err) = state.reset_fault {
            return Err(err);
        }
        state.in_reset = true;
        // The latch's clear input is tied to the device's reset line.
        state.ready_latched = false;
        Ok(())
    }

    fn reset_deassert(&mut self, _: &u8) -> Result<(), MockSystemControlError> {
        let mut state = self.state.borrow_mut();
        if let Some(err) = state.reset_fault {
            return Err(err);
        }
        if state.release_preceded_by_flash_read.is_none() {
            state.release_preceded_by_flash_read = Some(state.flash_reads > 0);
        }
        state.in_reset = false;
        state.polls_since_release = 0;
        let slot = state.boot_slot();
        state.last_boot_slot = Some(slot);
        // One-shot arming: this boot consumes the trial selector.
        if state.trial_slot.is_some() {
            state.trial_consumed = true;
        }
        Ok(())
    }

    fn reset_pulse(
        &mut self,
        id: &u8,
        _: core::time::Duration,
    ) -> Result<(), MockSystemControlError> {
        self.reset_assert(id)?;
        self.reset_deassert(id)
    }

    fn reset_is_asserted(&self, _: &u8) -> Result<bool, MockSystemControlError> {
        Ok(self.state.borrow().in_reset)
    }
}

/// Bitmask over the mock GPIO input bank the device's boot-complete line is
/// wired to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MockPinMask(pub u8);

impl PinMask for MockPinMask {
    fn empty() -> Self {
        Self(0)
    }

    fn all() -> Self {
        Self(u8::MAX)
    }

    fn is_empty(&self) -> bool {
        self.0 == 0
    }

    fn contains(&self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    fn union(&self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    fn intersection(&self, other: Self) -> Self {
        Self(self.0 & other.0)
    }

    fn toggle(&self) -> Self {
        Self(!self.0)
    }
}

/// Error of the mock GPIO input bank.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MockGpioFault(pub GpioErrorKind);

impl GpioError for MockGpioFault {
    fn kind(&self) -> GpioErrorKind {
        self.0
    }
}

/// GPIO input bank carrying the device's boot-complete line. Input-only:
/// the output-side methods panic.
pub struct MockDeviceReadyLine<'d> {
    state: &'d RefCell<DeviceState>,
    pin: MockPinMask,
}

impl GpioErrorType for MockDeviceReadyLine<'_> {
    type Error = MockGpioFault;
}

impl GpioPort for MockDeviceReadyLine<'_> {
    type Config = ();
    type Mask = MockPinMask;

    fn read_input(&self) -> Result<MockPinMask, MockGpioFault> {
        let mut state = self.state.borrow_mut();
        if let Some(kind) = state.gpio_fault {
            return Err(MockGpioFault(kind));
        }
        Ok(if state.sample_ready() {
            self.pin
        } else {
            MockPinMask::empty()
        })
    }

    fn configure(&mut self, _: MockPinMask, _: ()) -> Result<(), MockGpioFault> {
        panic!("the mock device's ready line is input-only");
    }

    fn set_reset(&mut self, _: MockPinMask, _: MockPinMask) -> Result<(), MockGpioFault> {
        panic!("the mock device's ready line is input-only");
    }

    fn toggle(&mut self, _: MockPinMask) -> Result<(), MockGpioFault> {
        panic!("the mock device's ready line is input-only");
    }
}

/// Error of the mock device flash.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MockFlashError {
    /// Access past [`MOCK_DEVICE_FLASH_SIZE`].
    OutOfRange,
    /// The device's firmware write path is broken (injected fault).
    ProgramFault,
}

/// One addressable flash region of the mock device.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MockRegion {
    /// The slot the device's next release would boot (an armed, unconsumed
    /// trial, else the committed slot).
    BootSlot,
    /// An explicit firmware slot (A = 0, B = 1).
    Slot(usize),
    /// The staging area updates are written to before activation.
    Staging,
}

/// A flash view of the device. Only the [`MockRegion::BootSlot`] view
/// counts reads toward [`MockDownstreamDevice::flash_reads`].
pub struct MockDeviceFlash<'d> {
    state: &'d RefCell<DeviceState>,
    region: MockRegion,
}

impl MockDeviceFlash<'_> {
    fn range(addr: FlashAddress, len: usize) -> Result<core::ops::Range<usize>, MockFlashError> {
        let start = addr.offset() as usize;
        let end = start.checked_add(len).ok_or(MockFlashError::OutOfRange)?;
        if end > MOCK_DEVICE_FLASH_SIZE {
            return Err(MockFlashError::OutOfRange);
        }
        Ok(start..end)
    }

    fn region_mut<'s>(&self, state: &'s mut DeviceState) -> &'s mut [u8; MOCK_DEVICE_FLASH_SIZE] {
        match self.region {
            MockRegion::BootSlot => {
                let slot = state.boot_slot();
                &mut state.slots[slot]
            }
            MockRegion::Slot(slot) => &mut state.slots[slot],
            MockRegion::Staging => &mut state.staging,
        }
    }
}

impl Flash for MockDeviceFlash<'_> {
    type Error = MockFlashError;

    fn geometry(&mut self) -> Result<(NonZero<usize>, PowerOf2Usize, u32), MockFlashError> {
        let size = NonZero::new(MOCK_DEVICE_FLASH_SIZE).unwrap();
        let page = PowerOf2Usize::new(MOCK_DEVICE_FLASH_SIZE).unwrap();
        Ok((size, page, 1 << MOCK_DEVICE_FLASH_SIZE.trailing_zeros()))
    }

    fn read(&mut self, start_addr: FlashAddress, buf: &mut [u8]) -> Result<(), MockFlashError> {
        let range = Self::range(start_addr, buf.len())?;
        let mut state = self.state.borrow_mut();
        let region = self.region_mut(&mut state);
        buf.copy_from_slice(&region[range]);
        if self.region == MockRegion::BootSlot {
            state.flash_reads += 1;
        }
        Ok(())
    }

    fn erase(
        &mut self,
        start_addr: FlashAddress,
        size: PowerOf2Usize,
    ) -> Result<(), MockFlashError> {
        let range = Self::range(start_addr, size.get())?;
        let mut state = self.state.borrow_mut();
        self.region_mut(&mut state)[range].fill(0xFF);
        Ok(())
    }

    fn program(&mut self, start_addr: FlashAddress, data: &[u8]) -> Result<(), MockFlashError> {
        let range = Self::range(start_addr, data.len())?;
        let mut state = self.state.borrow_mut();
        if state.program_fault {
            return Err(MockFlashError::ProgramFault);
        }
        if !state.in_reset && self.region != MockRegion::Staging {
            state.programs_while_running += 1;
        }
        self.region_mut(&mut state)[range].copy_from_slice(data);
        Ok(())
    }
}

/// Error of the mock device's slot store.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MockSlotError {
    /// Commit or rollback was called with no trial armed.
    NoTrialArmed,
    /// A slot index outside A/B.
    InvalidSlot,
}

impl core::fmt::Display for MockSlotError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NoTrialArmed => f.write_str("no trial slot armed"),
            Self::InvalidSlot => f.write_str("slot index outside A/B"),
        }
    }
}

impl core::error::Error for MockSlotError {}

/// The device's slot-selection store, implementing [`SlotControl`] with
/// one-shot arming. Records whether the first commit happened after the
/// device reported boot-complete.
pub struct MockDeviceSlotStore<'d> {
    state: &'d RefCell<DeviceState>,
}

impl SlotControl for MockDeviceSlotStore<'_> {
    type SlotId = usize;
    type Error = MockSlotError;

    fn active_slot(&self) -> Result<usize, MockSlotError> {
        Ok(self.state.borrow().committed_slot)
    }

    fn trial_slot(&self) -> Result<Option<usize>, MockSlotError> {
        Ok(self.state.borrow().trial_slot)
    }

    fn set_trial(&mut self, slot: usize) -> Result<(), MockSlotError> {
        if slot > 1 {
            return Err(MockSlotError::InvalidSlot);
        }
        let mut state = self.state.borrow_mut();
        state.trial_slot = Some(slot);
        state.trial_consumed = false;
        Ok(())
    }

    fn commit(&mut self) -> Result<(), MockSlotError> {
        let mut state = self.state.borrow_mut();
        let trial = state.trial_slot.take().ok_or(MockSlotError::NoTrialArmed)?;
        if state.commit_preceded_by_ready.is_none() {
            state.commit_preceded_by_ready = Some(state.ready_latched);
        }
        state.committed_slot = trial;
        state.trial_consumed = false;
        state.commit_count += 1;
        Ok(())
    }

    fn rollback(&mut self) -> Result<(), MockSlotError> {
        let mut state = self.state.borrow_mut();
        state.trial_slot.take().ok_or(MockSlotError::NoTrialArmed)?;
        state.trial_consumed = false;
        state.rollback_count += 1;
        Ok(())
    }
}
