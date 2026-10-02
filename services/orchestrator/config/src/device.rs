// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! One managed device: its checkpoints, its images, and its retry budget.

use crate::checkpoint::BootCheckpoint;
use crate::layout::ImageLayout;

/// Fails the build if `max_retry` is too small for a device to boot every
/// image. Recovery restores one image per attempt, and the orchestrator
/// counts the restore before it decides whether to boot, so the last
/// restore it allows is never booted. A device with two slots and a golden
/// image therefore needs four attempts for the golden image to run, not
/// three.
///
/// This is a floor, not the exact number. A board whose driver retries the
/// same image before stepping to the next one needs more attempts than
/// this. Devices with no layout are skipped, because the eRoT has no images
/// to step through for them.
///
/// Boards call it from a const fence, so a budget that is too small fails
/// the build:
///
/// ```ignore
/// const _: () = assert_retry_reaches_every_image(MAX_RETRY, MANAGED_DEVICES);
/// ```
///
/// # Panics
///
/// Panics if `max_retry` is not greater than a device's image count.
pub const fn assert_retry_reaches_every_image<R, P>(max_retry: u8, devices: &[DeviceConfig<R, P>]) {
    let mut i = 0;
    while i < devices.len() {
        if let Some(layout) = devices[i].layout() {
            assert!(
                max_retry as usize > layout.image_count(),
                "max_retry is too small to boot every image of a device"
            );
        }
        i += 1;
    }
}

/// One managed downstream device, as declared by the board config.
///
/// Generic over the board's reset signal type `R` (which must match the
/// `ResetId` of the reset controller behind the board's `BootControl`
/// implementation) and its boot-probe vocabulary `P`, for the same
/// reason: probes are board-specific.
///
/// Deliberately says nothing about attestation or commit requirements:
/// those follow from what kind of device this is (iRoT-backed or
/// symbiont, the orchestrator's `ComponentKind`), not from a table
/// setting — a second knob would only let the two disagree.
///
/// Fields are private so a device entry that violates the schema is
/// unrepresentable: [`new`](Self::new) is the only way in, and it checks.
#[derive(Debug, Clone, Copy)]
pub struct DeviceConfig<R, P: 'static> {
    name: &'static str,
    reset_signal: R,
    checkpoints: &'static [BootCheckpoint<P>],
    layout: Option<ImageLayout>,
}

impl<R, P> DeviceConfig<R, P> {
    /// Declares a managed device. `const`, so board tables run the checks
    /// at build time.
    ///
    /// # Panics
    ///
    /// Panics — a build error in const context — if `name` is empty, if
    /// `checkpoints` is empty, if two checkpoints share a name (failure
    /// reports identify a checkpoint by name; a duplicate would make them
    /// ambiguous). Layout rules are checked by [`ImageLayout::new`].
    #[must_use]
    pub const fn new(
        name: &'static str,
        reset_signal: R,
        checkpoints: &'static [BootCheckpoint<P>],
        layout: Option<ImageLayout>,
    ) -> Self {
        assert!(!name.is_empty(), "device name must not be empty");
        assert!(
            !checkpoints.is_empty(),
            "device must declare at least one boot checkpoint"
        );
        let mut c = 0;
        while c < checkpoints.len() {
            let mut d = c + 1;
            while d < checkpoints.len() {
                assert!(
                    !str_eq(checkpoints[c].name(), checkpoints[d].name()),
                    "checkpoint names must be unique per device"
                );
                d += 1;
            }
            c += 1;
        }
        Self {
            name,
            reset_signal,
            checkpoints,
            layout,
        }
    }

    /// The device's name in reports and logs.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        self.name
    }

    /// Reset signal id, passed to HalBootControl::new.
    #[must_use]
    pub const fn reset_signal(&self) -> &R {
        &self.reset_signal
    }

    /// Boot checkpoints, in the order the device passes them. The device
    /// counts as booted when the last one is reached; a checkpoint whose
    /// window expires fails the attempt — whether to retry or recover is
    /// the orchestrator's decision, not table data.
    #[must_use]
    pub const fn checkpoints(&self) -> &'static [BootCheckpoint<P>] {
        self.checkpoints
    }

    /// This device's images, when the eRoT owns its flash. `None` when the
    /// device takes its own updates, where the eRoT never addresses a byte
    /// range.
    #[must_use]
    pub const fn layout(&self) -> Option<ImageLayout> {
        self.layout
    }
}

// `==` on `&str` is not const; compare bytes by hand.
const fn str_eq(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    if a.len() != b.len() {
        return false;
    }
    let mut i = 0;
    while i < a.len() {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{Golden, ImageLayout, Region, Slot, SlotId};
    use core::time::Duration;

    // Board tables run the constructors at compile time, where a
    // rejection is a build error nobody can assert on. These tests call
    // them at runtime to prove the reject paths actually fire.

    const BOOT_COMPLETE: BootCheckpoint<u8> =
        BootCheckpoint::new("boot-complete", 0, Duration::from_secs(1));

    // Same name, different signal: each checkpoint is individually valid,
    // so the pair only trips the device-level duplicate check.
    const BOOT_COMPLETE_DUPLICATE_NAME: BootCheckpoint<u8> =
        BootCheckpoint::new("boot-complete", 1, Duration::from_secs(1));

    /// Slots are one megabyte each; the values only have to be distinct
    /// and non-overlapping.
    const SLOT_LEN: u32 = 0x10_0000;

    /// An ordinary slot, placed by id so two of them never overlap by
    /// accident.
    const fn slot(id: u8) -> Slot {
        Slot::new(SlotId(id), Region::new(id as u32 * SLOT_LEN, SLOT_LEN))
    }

    /// The golden image, above every slot `slot` can place.
    const GOLDEN: Golden = Golden::new(Region::new(0xF000_0000, SLOT_LEN));

    const LAYOUT: ImageLayout = ImageLayout::new(const { &[slot(0), slot(1)] }, Some(GOLDEN));

    #[test]
    #[should_panic(expected = "checkpoint names must be unique")]
    fn rejects_duplicate_checkpoint_names() {
        let _ = DeviceConfig::new(
            "dev",
            0u8,
            &[BOOT_COMPLETE, BOOT_COMPLETE_DUPLICATE_NAME],
            None,
        );
    }

    #[test]
    fn accepts_a_valid_table() {
        let device = DeviceConfig::new("dev", 0u8, &[BOOT_COMPLETE], Some(LAYOUT));
        assert_eq!(device.name(), "dev");
        assert_eq!(*device.reset_signal(), 0);
        assert_eq!(device.checkpoints().len(), 1);
        assert_eq!(device.checkpoints()[0].name(), "boot-complete");
        assert_eq!(*device.checkpoints()[0].probe(), 0);
        assert_eq!(device.checkpoints()[0].timeout(), Duration::from_secs(1));

        let layout = device.layout().expect("declared above");
        assert_eq!(layout.slots().len(), 2);
        assert_eq!(layout.slots()[1].region().base(), SLOT_LEN);
        assert_eq!(layout.slots()[1].region().end(), 2 * SLOT_LEN);
        assert_eq!(
            layout.golden().expect("declared above").region().base(),
            0xF000_0000
        );
    }

    /// A device that owns its own images declares no layout at all, golden
    /// included: the eRoT never addresses a byte range for it.
    #[test]
    fn accepts_a_device_without_a_layout() {
        let device = DeviceConfig::new("dev", 0u8, &[BOOT_COMPLETE], None);
        assert!(device.layout().is_none());
    }

    #[test]
    #[should_panic(expected = "device name must not be empty")]
    fn rejects_an_empty_device_name() {
        let _ = DeviceConfig::new("", 0u8, &[BOOT_COMPLETE], None);
    }

    #[test]
    #[should_panic(expected = "at least one boot checkpoint")]
    fn rejects_an_empty_checkpoint_list() {
        let _ = DeviceConfig::new("dev", 0u8, &[] as &[BootCheckpoint<u8>], None);
    }

    /// Two slots and a golden image need four attempts: three restores
    /// plus the one the last restore would otherwise never get.
    #[test]
    fn accepts_a_retry_budget_that_boots_the_golden_image() {
        const DEVICES: &[DeviceConfig<u8, u8>] =
            &[DeviceConfig::new("dev", 0, &[BOOT_COMPLETE], Some(LAYOUT))];
        assert_retry_reaches_every_image(4, DEVICES);
    }

    /// A device with no layout has no images to step through, so any
    /// budget covers it.
    #[test]
    fn accepts_any_retry_budget_for_a_device_without_a_layout() {
        const DEVICES: &[DeviceConfig<u8, u8>] =
            &[DeviceConfig::new("dev", 0, &[BOOT_COMPLETE], None)];
        assert_retry_reaches_every_image(0, DEVICES);
    }

    #[test]
    /// Three attempts restore the golden image and stop before booting it.
    #[should_panic(expected = "max_retry is too small")]
    fn rejects_a_retry_budget_that_never_boots_the_golden_image() {
        const DEVICES: &[DeviceConfig<u8, u8>] =
            &[DeviceConfig::new("dev", 0, &[BOOT_COMPLETE], Some(LAYOUT))];
        assert_retry_reaches_every_image(3, DEVICES);
    }
}
