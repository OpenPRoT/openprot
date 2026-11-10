// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Device side of the OCP Secure Firmware Recovery interface.
//!
//! In OCP recovery, a recovery agent (usually the BMC) sends recovery images
//! to a device with block reads and writes of the recovery commands
//! (`DEVICE_STATUS`, `RECOVERY_CTRL`, `INDIRECT_FIFO_DATA` and others). The
//! agent starts every transfer, and the device only responds. Over I3C a
//! target cannot stretch the clock, so hardware answers the commands. On
//! Caliptra, the i3c-core answers reads from a set of CSRs and applies the
//! agent's writes to them. Device firmware sets the values that the agent
//! will read next and reads what the agent has written.
//!
//! This crate is for that firmware. It defines:
//!
//! * the values that the agent reads from the device ([`DeviceStatus`],
//!   [`RecoveryReason`], [`RecoveryStatus`], [`ImageIndex`]);
//! * [`RecoveryDriver`], the low-level interface to the hardware that holds
//!   these values and the image FIFO (CMS 0);
//! * [`RecoveryDevice`], which runs the sequence to receive one or more images
//!   through the FIFO and blocks while it waits for the agent.
//!
//! The agent finds out what the device wants by polling `DEVICE_STATUS` and
//! `RECOVERY_STATUS`. The sequence for each image follows Caliptra's
//! `DmaRecovery`:
//!
//! 1. The device clears the FIFO, sets `DEVICE_STATUS` to
//!    [`DeviceStatus::RecoveryMode`] and `RECOVERY_STATUS` to
//!    [`RecoveryStatus::AwaitingImage`] with the image index.
//! 2. The agent writes the image size to `INDIRECT_FIFO_CTRL` and then the
//!    image to `INDIRECT_FIFO_DATA`. The device reads the FIFO.
//! 3. The device sets `DEVICE_STATUS` to [`DeviceStatus::RecoveryPending`]
//!    and waits for the agent to write "activate" to `RECOVERY_CTRL`.
//! 4. The device sets `RECOVERY_STATUS` to [`RecoveryStatus::BootingImage`].
//!
//! After the last image, the device reports success or failure with
//! [`RecoveryDevice::complete`] or [`RecoveryDevice::fail`].

#![cfg_attr(not(test), no_std)]

use core::cmp::min;
use core::num::NonZero;

use util_types::Blocking;

/// `DEVICE_STATUS` byte 0: the device state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum DeviceStatus {
    /// Status pending.
    StatusPending = 0x0,
    /// Device healthy.
    Healthy = 0x1,
    /// Device error.
    Error = 0x2,
    /// Recovery mode, ready to accept a recovery image.
    RecoveryMode = 0x3,
    /// Recovery pending, waiting for activation.
    RecoveryPending = 0x4,
    /// Running the recovery image.
    RunningRecoveryImage = 0x5,
    /// Boot failure.
    BootFailure = 0xe,
    /// Fatal error.
    FatalError = 0xf,
}

/// `DEVICE_STATUS` bytes 2 and 3: the recovery reason code.
///
/// The OCP specification defines codes below 0x80. Codes 0x80 to 0xff are
/// vendor-defined.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RecoveryReason(pub u16);

impl RecoveryReason {
    /// No boot failure detected.
    pub const NONE: Self = Self(0x00);
    /// Corrupted or missing critical data.
    pub const CORRUPTED_CRITICAL_DATA: Self = Self(0x04);
    /// Authentication failure on the main firmware image.
    pub const MAIN_FIRMWARE_AUTHENTICATION_FAILURE: Self = Self(0x0c);
    /// Forced recovery.
    pub const FORCED_RECOVERY: Self = Self(0x11);
    /// Flashless or streaming boot. Caliptra uses this code when it asks for
    /// the images that it loads after boot.
    pub const FLASHLESS_BOOT: Self = Self(0x12);
}

/// `RECOVERY_STATUS` bits 0 to 3: the device recovery status.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum RecoveryStatus {
    /// Not in recovery mode.
    NotInRecovery = 0x0,
    /// Awaiting a recovery image.
    AwaitingImage = 0x1,
    /// Booting the recovery image.
    BootingImage = 0x2,
    /// Recovery successful.
    Successful = 0x3,
    /// Recovery failed.
    Failed = 0xc,
    /// The recovery image failed authentication.
    AuthenticationError = 0xd,
    /// Error entering recovery mode.
    ErrorEnteringRecovery = 0xe,
    /// Invalid component memory space.
    InvalidCms = 0xf,
}

/// `RECOVERY_STATUS` bits 4 to 7: the index of the image the device asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ImageIndex(u8);

impl ImageIndex {
    /// Returns the index, or `None` if `index` does not fit in 4 bits.
    pub const fn new(index: u8) -> Option<Self> {
        if index < 16 {
            Some(Self(index))
        } else {
            None
        }
    }

    /// Returns the index.
    pub const fn get(self) -> u8 {
        self.0
    }
}

/// Low-level interface to the hardware that answers the recovery agent: the
/// status values the agent reads, the values the agent writes, and the image
/// FIFO.
///
/// The FIFO holds 32-bit words. Image sizes are in words, as in
/// `INDIRECT_FIFO_CTRL`.
pub trait RecoveryDriver {
    /// The error type returned by driver operations.
    type Error;

    /// Sets `DEVICE_STATUS` to `status` with recovery reason `reason`.
    fn set_device_status(
        &mut self,
        status: DeviceStatus,
        reason: RecoveryReason,
    ) -> Result<(), Self::Error>;

    /// Sets `RECOVERY_STATUS` to `status` for image `index`.
    fn set_recovery_status(
        &mut self,
        status: RecoveryStatus,
        index: ImageIndex,
    ) -> Result<(), Self::Error>;

    /// Empties the FIFO and clears the image size, so that the next call to
    /// [`image_words`](Self::image_words) returns `None` until the agent
    /// writes a new size.
    fn reset_fifo(&mut self) -> Result<(), Self::Error>;

    /// Returns the image size, in words, that the agent wrote to
    /// `INDIRECT_FIFO_CTRL`, or `None` if it has not written one since the
    /// last [`reset_fifo`](Self::reset_fifo).
    fn image_words(&mut self) -> Result<Option<NonZero<u32>>, Self::Error>;

    /// Moves words from the FIFO into `buf` until `buf` is full or the FIFO is
    /// empty, and returns the number of words moved. Does not wait for data.
    fn read_fifo(&mut self, buf: &mut [u32]) -> Result<usize, Self::Error>;

    /// Returns whether the agent has asked the device to activate the image
    /// (`RECOVERY_CTRL` byte 2 is 0xf).
    fn activation_requested(&mut self) -> Result<bool, Self::Error>;

    /// Clears an activation request, so that
    /// [`activation_requested`](Self::activation_requested) returns `false`
    /// until the agent asks again.
    fn clear_activation(&mut self) -> Result<(), Self::Error>;
}

/// Receives recovery images through a [`RecoveryDriver`].
///
/// `blocking` is called each time the device waits for the agent: for the
/// image size, for FIFO data, and for activation. A polling implementation
/// can sleep or yield there.
pub struct RecoveryDevice<TDriver: RecoveryDriver, TBlocking: Blocking> {
    /// The underlying recovery driver.
    pub driver: TDriver,
    /// The blocking mechanism used to wait for the agent.
    pub blocking: TBlocking,
}

impl<TDriver: RecoveryDriver, TBlocking: Blocking> RecoveryDevice<TDriver, TBlocking> {
    /// Reports that the device is awaiting image `index`, and waits until the
    /// agent writes the image size. The agent sees the request the next time
    /// it reads `RECOVERY_STATUS`.
    ///
    /// `reason` is the recovery reason that the device reports while it waits
    /// for the image.
    pub fn request_image(
        &mut self,
        index: ImageIndex,
        reason: RecoveryReason,
    ) -> Result<ImageTransfer<'_, TDriver, TBlocking>, TDriver::Error> {
        self.driver.clear_activation()?;
        self.driver.reset_fifo()?;
        self.driver
            .set_device_status(DeviceStatus::RecoveryMode, reason)?;
        self.driver
            .set_recovery_status(RecoveryStatus::AwaitingImage, index)?;
        let words = loop {
            if let Some(words) = self.driver.image_words()? {
                break words;
            }
            self.blocking.wait_for_notification();
        };
        Ok(ImageTransfer {
            device: self,
            index,
            len_words: words.get(),
            remaining_words: words.get(),
        })
    }

    /// Reports that recovery has finished and the device is healthy.
    pub fn complete(&mut self, index: ImageIndex) -> Result<(), TDriver::Error> {
        self.driver
            .set_recovery_status(RecoveryStatus::Successful, index)?;
        self.driver
            .set_device_status(DeviceStatus::Healthy, RecoveryReason::NONE)
    }

    /// Reports that image `index` failed with `status` and `reason`.
    pub fn fail(
        &mut self,
        index: ImageIndex,
        status: RecoveryStatus,
        reason: RecoveryReason,
    ) -> Result<(), TDriver::Error> {
        self.driver.set_recovery_status(status, index)?;
        self.driver
            .set_device_status(DeviceStatus::BootFailure, reason)
    }
}

/// An image that the agent is writing into the FIFO.
///
/// Read the image with [`read`](Self::read), then call
/// [`wait_for_activation`](Self::wait_for_activation). To stop without
/// waiting for activation, drop the transfer and call
/// [`RecoveryDevice::fail`]; the next
/// [`request_image`](RecoveryDevice::request_image) clears the FIFO.
pub struct ImageTransfer<'a, TDriver: RecoveryDriver, TBlocking: Blocking> {
    device: &'a mut RecoveryDevice<TDriver, TBlocking>,
    index: ImageIndex,
    len_words: u32,
    remaining_words: u32,
}

impl<TDriver: RecoveryDriver, TBlocking: Blocking> ImageTransfer<'_, TDriver, TBlocking> {
    /// Returns the index of the image.
    pub fn index(&self) -> ImageIndex {
        self.index
    }

    /// Returns the image size in words.
    pub fn len_words(&self) -> u32 {
        self.len_words
    }

    /// Returns the number of words not yet read.
    pub fn remaining_words(&self) -> u32 {
        self.remaining_words
    }

    /// Reads the next words of the image into `buf` and returns the number
    /// of words read.
    ///
    /// Waits until at least one word is available. Returns 0 if `buf` is
    /// empty or the whole image has been read.
    pub fn read(&mut self, buf: &mut [u32]) -> Result<usize, TDriver::Error> {
        let limit = min(buf.len(), self.remaining_words as usize);
        let Some(buf) = buf.get_mut(..limit) else {
            return Ok(0);
        };
        if buf.is_empty() {
            return Ok(0);
        }
        loop {
            let count = min(self.device.driver.read_fifo(buf)?, limit);
            if count > 0 {
                self.remaining_words = self.remaining_words.saturating_sub(count as u32);
                return Ok(count);
            }
            self.device.blocking.wait_for_notification();
        }
    }

    /// Waits for the agent to activate the image, and reports that the device
    /// is booting it.
    ///
    /// Words that have not been read are read and discarded first, so that
    /// the FIFO is empty for the next image.
    pub fn wait_for_activation(mut self) -> Result<ImageIndex, TDriver::Error> {
        let mut discard = [0_u32; 16];
        while self.read(&mut discard)? > 0 {}
        let device = self.device;
        device
            .driver
            .set_device_status(DeviceStatus::RecoveryPending, RecoveryReason::NONE)?;
        while !device.driver.activation_requested()? {
            device.blocking.wait_for_notification();
        }
        device
            .driver
            .set_recovery_status(RecoveryStatus::BootingImage, self.index)?;
        Ok(self.index)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::cell::Cell;
    use std::collections::VecDeque;

    /// Calls `on_wait` each time the device waits, so a test can play the
    /// agent's part.
    struct FakeBlocking<'a> {
        waits: Cell<usize>,
        on_wait: &'a dyn Fn(usize),
    }

    impl Blocking for FakeBlocking<'_> {
        fn wait_for_notification(&self) {
            let n = self.waits.get();
            self.waits.set(n + 1);
            (self.on_wait)(n);
        }
    }

    #[derive(Default)]
    struct Regs {
        device_status: Option<(DeviceStatus, RecoveryReason)>,
        recovery_status: Option<(RecoveryStatus, ImageIndex)>,
        image_words: u32,
        fifo: VecDeque<u32>,
        activate: bool,
        log: Vec<&'static str>,
    }

    /// A driver over registers that the test (the agent) can also change.
    struct FakeDriver<'a>(&'a std::cell::RefCell<Regs>);

    impl RecoveryDriver for FakeDriver<'_> {
        type Error = ();
        fn set_device_status(
            &mut self,
            status: DeviceStatus,
            reason: RecoveryReason,
        ) -> Result<(), ()> {
            let mut r = self.0.borrow_mut();
            r.device_status = Some((status, reason));
            r.log.push("device_status");
            Ok(())
        }
        fn set_recovery_status(
            &mut self,
            status: RecoveryStatus,
            index: ImageIndex,
        ) -> Result<(), ()> {
            let mut r = self.0.borrow_mut();
            r.recovery_status = Some((status, index));
            r.log.push("recovery_status");
            Ok(())
        }
        fn reset_fifo(&mut self) -> Result<(), ()> {
            let mut r = self.0.borrow_mut();
            r.fifo.clear();
            r.image_words = 0;
            r.log.push("reset_fifo");
            Ok(())
        }
        fn image_words(&mut self) -> Result<Option<NonZero<u32>>, ()> {
            Ok(NonZero::new(self.0.borrow().image_words))
        }
        fn read_fifo(&mut self, buf: &mut [u32]) -> Result<usize, ()> {
            let mut r = self.0.borrow_mut();
            let mut n = 0;
            for word in buf.iter_mut() {
                match r.fifo.pop_front() {
                    Some(w) => *word = w,
                    None => break,
                }
                n += 1;
            }
            Ok(n)
        }
        fn activation_requested(&mut self) -> Result<bool, ()> {
            Ok(self.0.borrow().activate)
        }
        fn clear_activation(&mut self) -> Result<(), ()> {
            let mut r = self.0.borrow_mut();
            r.activate = false;
            r.log.push("clear_activation");
            Ok(())
        }
    }

    fn index(i: u8) -> ImageIndex {
        ImageIndex::new(i).unwrap()
    }

    #[test]
    fn image_index_is_four_bits() {
        assert_eq!(ImageIndex::new(15).map(ImageIndex::get), Some(15));
        assert_eq!(ImageIndex::new(16), None);
    }

    #[test]
    fn receives_image_in_chunks() {
        let regs = std::cell::RefCell::new(Regs::default());
        let image: Vec<u32> = (0..10).collect();
        // The agent writes the size on the first wait, then delivers four
        // words per wait, then activates.
        let agent = |n: usize| {
            let mut r = regs.borrow_mut();
            if n == 0 {
                r.image_words = image.len() as u32;
            } else if r.fifo.is_empty()
                && r.device_status.map(|s| s.0) != Some(DeviceStatus::RecoveryPending)
            {
                let start = (n - 1) * 4;
                r.fifo.extend(image.iter().skip(start).take(4));
            } else {
                r.activate = true;
            }
        };
        let mut device = RecoveryDevice {
            driver: FakeDriver(&regs),
            blocking: FakeBlocking {
                waits: Cell::new(0),
                on_wait: &agent,
            },
        };

        let mut transfer = device
            .request_image(index(3), RecoveryReason::FLASHLESS_BOOT)
            .unwrap();
        assert_eq!(
            regs.borrow().device_status,
            Some((DeviceStatus::RecoveryMode, RecoveryReason::FLASHLESS_BOOT))
        );
        assert_eq!(
            regs.borrow().recovery_status,
            Some((RecoveryStatus::AwaitingImage, index(3)))
        );
        assert_eq!(transfer.len_words(), 10);

        let mut received = Vec::new();
        let mut buf = [0_u32; 3];
        loop {
            let n = transfer.read(&mut buf).unwrap();
            if n == 0 {
                break;
            }
            received.extend_from_slice(&buf[..n]);
        }
        assert_eq!(received, image);
        assert_eq!(transfer.remaining_words(), 0);

        assert_eq!(transfer.wait_for_activation().unwrap(), index(3));
        assert_eq!(
            regs.borrow().recovery_status,
            Some((RecoveryStatus::BootingImage, index(3)))
        );

        device.complete(index(3)).unwrap();
        assert_eq!(
            regs.borrow().device_status,
            Some((DeviceStatus::Healthy, RecoveryReason::NONE))
        );
        assert_eq!(
            regs.borrow().recovery_status,
            Some((RecoveryStatus::Successful, index(3)))
        );
    }

    #[test]
    fn request_clears_state_before_asking() {
        let regs = std::cell::RefCell::new(Regs {
            image_words: 7,
            fifo: [1, 2, 3].into(),
            activate: true,
            ..Default::default()
        });
        let agent = |_: usize| regs.borrow_mut().image_words = 1;
        let mut device = RecoveryDevice {
            driver: FakeDriver(&regs),
            blocking: FakeBlocking {
                waits: Cell::new(0),
                on_wait: &agent,
            },
        };
        let len_words = device
            .request_image(index(1), RecoveryReason::FLASHLESS_BOOT)
            .unwrap()
            .len_words();
        // The stale size of 7 words was not used.
        assert_eq!(len_words, 1);
        let r = regs.borrow();
        assert!(!r.activate);
        assert!(r.fifo.is_empty());
        assert_eq!(
            r.log,
            [
                "clear_activation",
                "reset_fifo",
                "device_status",
                "recovery_status"
            ]
        );
    }

    #[test]
    fn activation_discards_unread_words() {
        let regs = std::cell::RefCell::new(Regs::default());
        let agent = |n: usize| {
            let mut r = regs.borrow_mut();
            match n {
                0 => {
                    r.image_words = 5;
                    r.fifo.extend([1, 2, 3, 4, 5]);
                }
                _ => r.activate = true,
            }
        };
        let mut device = RecoveryDevice {
            driver: FakeDriver(&regs),
            blocking: FakeBlocking {
                waits: Cell::new(0),
                on_wait: &agent,
            },
        };
        let mut transfer = device
            .request_image(index(2), RecoveryReason::FLASHLESS_BOOT)
            .unwrap();
        let mut buf = [0_u32; 2];
        assert_eq!(transfer.read(&mut buf).unwrap(), 2);
        transfer.wait_for_activation().unwrap();
        assert!(regs.borrow().fifo.is_empty());
    }

    #[test]
    fn read_does_not_take_more_than_the_image() {
        let regs = std::cell::RefCell::new(Regs::default());
        // The FIFO holds a word past the end of the image.
        let agent = |_: usize| {
            let mut r = regs.borrow_mut();
            r.image_words = 2;
            r.fifo.extend([1, 2, 99]);
        };
        let mut device = RecoveryDevice {
            driver: FakeDriver(&regs),
            blocking: FakeBlocking {
                waits: Cell::new(0),
                on_wait: &agent,
            },
        };
        let mut transfer = device
            .request_image(index(0), RecoveryReason::FLASHLESS_BOOT)
            .unwrap();
        let mut buf = [0_u32; 8];
        assert_eq!(transfer.read(&mut buf).unwrap(), 2);
        assert_eq!(buf[..2], [1, 2]);
        assert_eq!(transfer.read(&mut buf).unwrap(), 0);
        assert_eq!(regs.borrow().fifo, [99]);
    }

    #[test]
    fn fail_reports_boot_failure() {
        let regs = std::cell::RefCell::new(Regs::default());
        let agent = |_: usize| {};
        let mut device = RecoveryDevice {
            driver: FakeDriver(&regs),
            blocking: FakeBlocking {
                waits: Cell::new(0),
                on_wait: &agent,
            },
        };
        device
            .fail(
                index(4),
                RecoveryStatus::AuthenticationError,
                RecoveryReason::MAIN_FIRMWARE_AUTHENTICATION_FAILURE,
            )
            .unwrap();
        let r = regs.borrow();
        assert_eq!(
            r.recovery_status,
            Some((RecoveryStatus::AuthenticationError, index(4)))
        );
        assert_eq!(
            r.device_status,
            Some((
                DeviceStatus::BootFailure,
                RecoveryReason::MAIN_FIRMWARE_AUTHENTICATION_FAILURE
            ))
        );
    }
}
