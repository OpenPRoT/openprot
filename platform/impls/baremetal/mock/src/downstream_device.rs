// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Mock downstream device: one managed device as the eRoT sees it — a reset
//! line in, a boot-complete line out, and its firmware flash.
//!
//! All three handles observe the same device state, so a test can bind them
//! to real adapters and treat the eRoT plus device as a black box: load a
//! firmware image, run the boot flow, and check only externally visible
//! outcomes (was the device released, did it report boot-complete).

use core::cell::RefCell;
use core::num::NonZero;

use hal_flash::{Flash, FlashAddress};
use openprot_hal_blocking::gpio_port::{
    GpioError, GpioErrorKind, GpioErrorType, GpioPort, PinMask,
};
use openprot_hal_blocking::system_control::{ErrorType, ResetControl};
use util_types::PowerOf2Usize;

use crate::system_control::MockSystemControlError;

/// Flash size of the mock device.
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
    flash: [u8; MOCK_DEVICE_FLASH_SIZE],
    flash_reads: usize,
    /// Recorded at the first release: had the device's flash been read by
    /// then? Lets tests prove verification preceded release.
    release_preceded_by_flash_read: Option<bool>,
    reset_fault: Option<MockSystemControlError>,
    gpio_fault: Option<GpioErrorKind>,
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
                flash: [0xFF; MOCK_DEVICE_FLASH_SIZE],
                flash_reads: 0,
                release_preceded_by_flash_read: None,
                reset_fault: None,
                gpio_fault: None,
            }),
        }
    }

    /// Writes `image` to the start of the device's flash.
    ///
    /// # Panics
    ///
    /// Panics if `image` exceeds [`MOCK_DEVICE_FLASH_SIZE`].
    pub fn load_firmware(&self, image: &[u8]) {
        let mut state = self.state.borrow_mut();
        state.flash[..image.len()].copy_from_slice(image);
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

    /// The device's firmware flash, as the eRoT's interposer sees it.
    pub fn flash(&self) -> MockDeviceFlash<'_> {
        MockDeviceFlash { state: &self.state }
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
}

/// The device's firmware flash. Reads count toward
/// [`MockDownstreamDevice::flash_reads`], so tests can prove the eRoT
/// verified firmware before releasing the device.
pub struct MockDeviceFlash<'d> {
    state: &'d RefCell<DeviceState>,
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
        buf.copy_from_slice(&state.flash[range]);
        state.flash_reads += 1;
        Ok(())
    }

    fn erase(
        &mut self,
        start_addr: FlashAddress,
        size: PowerOf2Usize,
    ) -> Result<(), MockFlashError> {
        let range = Self::range(start_addr, size.get())?;
        self.state.borrow_mut().flash[range].fill(0xFF);
        Ok(())
    }

    fn program(&mut self, start_addr: FlashAddress, data: &[u8]) -> Result<(), MockFlashError> {
        let range = Self::range(start_addr, data.len())?;
        self.state.borrow_mut().flash[range].copy_from_slice(data);
        Ok(())
    }
}
