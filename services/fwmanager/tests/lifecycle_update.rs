// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Host integration tests for the downstream firmware-update lifecycle,
//! black-box style: every assertion is on externally visible signals —
//! reset transitions, what was flashed while the device was held, which
//! slot booted, what got committed.
//!
//! `stage_update`/`activate_update` are test-local stand-ins for the
//! orchestrator (PR #357): when it lands, it replaces these drivers and
//! the assertions stay.

use std::error::Error;

use fwmanager_api::{await_boot, BootControl, BootMonitor, BootProgress, SlotControl};
use fwmanager_hal_adapters::{GpioBootMonitor, HalBootControl};
use hal_flash::{Flash, FlashAddress};
use openprot_hal_blocking::gpio_port::ActivePolarity;
use openprot_platform_mock::downstream_device::{
    MockDownstreamDevice, MockPinMask, MockRegion, MOCK_DEVICE_FLASH_SIZE,
};

// Board binding: BMC reset on controller line 7, boot-complete on GPIO
// line 4. Normally set in the board's devices.rs.
const BMC_RESET: u8 = 7;
const BMC_READY: MockPinMask = MockPinMask(1 << 4);

// Same stand-in image format as boot_flow.rs: 4 magic bytes, payload, and a
// final byte making the XOR over the whole image zero.
const IMAGE_LEN: usize = 16;
const MAGIC: [u8; 4] = *b"OPRT";

fn image_with_payload(fill: u8) -> [u8; IMAGE_LEN] {
    let mut image = [0u8; IMAGE_LEN];
    image[..4].copy_from_slice(&MAGIC);
    image[4..IMAGE_LEN - 1].fill(fill);
    let checksum = image[..IMAGE_LEN - 1].iter().fold(0, |acc, b| acc ^ b);
    image[IMAGE_LEN - 1] = checksum;
    image
}

fn running_image() -> [u8; IMAGE_LEN] {
    image_with_payload(0xAB)
}

fn update_image() -> [u8; IMAGE_LEN] {
    image_with_payload(0xCD)
}

fn corrupt_update_image() -> [u8; IMAGE_LEN] {
    let mut image = update_image();
    image[7] ^= 0x01;
    image
}

fn image_is_valid(image: &[u8; IMAGE_LEN]) -> bool {
    let checksum = image.iter().fold(0, |acc, b| acc ^ b);
    image[..4] == MAGIC && checksum == 0
}

#[derive(Debug, PartialEq, Eq)]
enum UpdateOutcome {
    /// Trial boot confirmed; the new slot is committed.
    Committed,
    /// The staged image failed authentication; staging was discarded and no
    /// slot or reset line was ever touched.
    RejectedStaged,
    /// The device's firmware write path reported a failure; nothing was
    /// armed or committed and the device was released on its old slot.
    WriteFailed,
    /// The trial boot produced no (or negative) evidence; the trial was
    /// rolled back and the device rebooted on its old slot.
    RolledBack,
}

/// Stage `image` into the device's staging region and authenticate it
/// there. Rejection discards the staged bytes. The device keeps running —
/// staging never touches the reset line or a bootable slot.
fn stage_update<F: Flash>(
    staging: &mut F,
    image: &[u8; IMAGE_LEN],
) -> Result<Option<UpdateOutcome>, Box<dyn Error>>
where
    F::Error: core::fmt::Debug,
{
    if staging.program(FlashAddress::new(0), image).is_err() {
        return Ok(Some(UpdateOutcome::WriteFailed));
    }

    // AuthenticateUpdate: read back what actually landed and check it.
    let mut staged = [0u8; IMAGE_LEN];
    staging
        .read(FlashAddress::new(0), &mut staged)
        .map_err(|e| format!("staging read failed: {e:?}"))?;
    if !image_is_valid(&staged) {
        staging
            .erase(
                FlashAddress::new(0),
                util_types::PowerOf2Usize::new(MOCK_DEVICE_FLASH_SIZE).unwrap(),
            )
            .map_err(|e| format!("staging erase failed: {e:?}"))?;
        return Ok(Some(UpdateOutcome::RejectedStaged));
    }
    Ok(None)
}

/// Activate the staged update on the device's inactive slot as a trial
/// boot: hold the device in reset, flash the slot while nothing runs, arm
/// it as a trial, release, and watch the boot window. Commit only on
/// observed boot completion; anything else rolls back and reboots the old
/// slot.
#[allow(clippy::too_many_arguments)]
fn activate_update<C, M, S, F1, F2>(
    control: &mut C,
    monitor: &M,
    slots: &mut S,
    staging: &mut F1,
    target_flash: &mut F2,
    target_slot: usize,
    poll_budget: usize,
) -> Result<UpdateOutcome, Box<dyn Error>>
where
    C: BootControl,
    M: BootMonitor,
    S: SlotControl<SlotId = usize>,
    F1: Flash,
    F2: Flash,
    C::Error: 'static,
    M::Error: 'static,
    S::Error: 'static,
    F1::Error: core::fmt::Debug,
    F2::Error: core::fmt::Debug,
{
    // The device must be frozen before its firmware is touched.
    control.hold_in_reset()?;

    // Copy staging into the inactive slot.
    let mut staged = [0u8; IMAGE_LEN];
    staging
        .read(FlashAddress::new(0), &mut staged)
        .map_err(|e| format!("staging read failed: {e:?}"))?;
    if target_flash.program(FlashAddress::new(0), &staged).is_err() {
        // The device told us its firmware write failed. Nothing is armed;
        // releasing boots the untouched committed slot.
        control.release()?;
        return Ok(UpdateOutcome::WriteFailed);
    }

    // Arm the trial and boot it.
    slots.set_trial(target_slot)?;
    control.release()?;

    match await_boot(monitor, poll_budget)? {
        BootProgress::Booted => {
            slots.commit()?;
            Ok(UpdateOutcome::Committed)
        }
        BootProgress::Failed | BootProgress::Timeout => {
            control.hold_in_reset()?;
            slots.rollback()?;
            control.release()?; // reboot the still-committed old slot
            Ok(UpdateOutcome::RolledBack)
        }
    }
}

/// Rig: a device running `running_image()` from slot A, plus the eRoT's
/// capability bindings. Returns the device already booted and confirmed.
fn booted_device() -> MockDownstreamDevice {
    let device = MockDownstreamDevice::held_in_reset(Some(1));
    device.load_firmware(&running_image());
    {
        let mut control = HalBootControl::new(device.reset_line(), BMC_RESET);
        let ready = device.ready_line(BMC_READY);
        let monitor = GpioBootMonitor::new(&ready, BMC_READY, ActivePolarity::ActiveHigh);
        control.release().expect("initial release");
        assert_eq!(
            await_boot(&monitor, 10).expect("initial boot"),
            BootProgress::Booted
        );
    }
    device
}

/// A successful downstream update: the eRoT stages the new image, holds the
/// device in reset for the whole time firmware is being flashed, reboots it
/// on the new slot, and commits only after observing the booted state.
#[test]
fn update_succeeds_and_device_is_held_in_reset_while_flashed() {
    let device = booted_device();
    let target = 1 - device.committed_slot();

    let mut control = HalBootControl::new(device.reset_line(), BMC_RESET);
    let ready = device.ready_line(BMC_READY);
    let monitor = GpioBootMonitor::new(&ready, BMC_READY, ActivePolarity::ActiveHigh);
    let mut slots = device.slot_store();
    let mut staging = device.region_flash(MockRegion::Staging);
    let mut target_flash = device.region_flash(MockRegion::Slot(target));

    assert_eq!(stage_update(&mut staging, &update_image()).unwrap(), None);
    let outcome = activate_update(
        &mut control,
        &monitor,
        &mut slots,
        &mut staging,
        &mut target_flash,
        target,
        10,
    )
    .expect("update flow failed");

    assert_eq!(outcome, UpdateOutcome::Committed);
    // Reset discipline: no firmware byte was ever written to a running
    // device — every program op happened while the reset line was asserted.
    assert_eq!(device.programs_while_running(), 0);
    // The device was rebooted into the new slot and reached the booted
    // state before anything was committed.
    assert_eq!(device.last_boot_slot(), Some(target));
    assert_eq!(device.commit_preceded_by_ready(), Some(true));
    assert_eq!(device.committed_slot(), target);
    assert_eq!(device.commit_count(), 1);
    assert_eq!(device.rollback_count(), 0);
    assert!(!device.is_in_reset());
    // The new image really is what the new slot runs.
    assert_eq!(device.slot_content(target)[..IMAGE_LEN], update_image());
}

/// The device's firmware write path fails during activation: the eRoT
/// notices the failed write, arms and commits nothing, and the device comes
/// back up on its old slot with its old image untouched.
#[test]
fn failed_firmware_write_is_noticed_and_nothing_is_committed() {
    let device = booted_device();
    let old_slot = device.committed_slot();
    let target = 1 - old_slot;

    let mut control = HalBootControl::new(device.reset_line(), BMC_RESET);
    let ready = device.ready_line(BMC_READY);
    let monitor = GpioBootMonitor::new(&ready, BMC_READY, ActivePolarity::ActiveHigh);
    let mut slots = device.slot_store();
    let mut staging = device.region_flash(MockRegion::Staging);
    let mut target_flash = device.region_flash(MockRegion::Slot(target));

    assert_eq!(stage_update(&mut staging, &update_image()).unwrap(), None);
    // The device's write path breaks after staging succeeded.
    device.inject_program_fault();

    let outcome = activate_update(
        &mut control,
        &monitor,
        &mut slots,
        &mut staging,
        &mut target_flash,
        target,
        10,
    )
    .expect("update flow failed");

    assert_eq!(outcome, UpdateOutcome::WriteFailed);
    // Nothing was armed or committed; the old slot is still the boot
    // selection and the device was released back onto it.
    assert_eq!(device.committed_slot(), old_slot);
    assert_eq!(device.commit_count(), 0);
    assert_eq!(device.last_boot_slot(), Some(old_slot));
    assert!(!device.is_in_reset());
    // The target slot was never written: still erased.
    assert!(device.slot_content(target).iter().all(|&b| b == 0xFF));
}

/// A corrupt staged image is rejected during staging: the staged bytes are
/// discarded and the running device is never disturbed — no reset, no slot
/// write, no trial.
#[test]
fn corrupt_staged_image_is_rejected_without_touching_the_device() {
    let device = booted_device();
    let target = 1 - device.committed_slot();

    let mut staging = device.region_flash(MockRegion::Staging);
    let outcome = stage_update(&mut staging, &corrupt_update_image()).unwrap();

    assert_eq!(outcome, Some(UpdateOutcome::RejectedStaged));
    assert!(!device.is_in_reset(), "staging must not reset the device");
    assert!(device.slot_content(target).iter().all(|&b| b == 0xFF));
    assert_eq!(device.commit_count(), 0);
    assert_eq!(device.rollback_count(), 0);
    // Discarded: the staging region holds no image anymore.
    let mut staged = [0u8; IMAGE_LEN];
    staging.read(FlashAddress::new(0), &mut staged).unwrap();
    assert!(staged.iter().all(|&b| b == 0xFF));
}

/// The trial image never reaches the booted state: the boot window expires,
/// the trial is rolled back, and the device is rebooted on the old slot —
/// which stays committed.
#[test]
fn trial_boot_timeout_rolls_back_and_reboots_the_old_slot() {
    let device = booted_device();
    let old_slot = device.committed_slot();
    let target = 1 - old_slot;

    let mut control = HalBootControl::new(device.reset_line(), BMC_RESET);
    let ready = device.ready_line(BMC_READY);
    let monitor = GpioBootMonitor::new(&ready, BMC_READY, ActivePolarity::ActiveHigh);
    let mut slots = device.slot_store();
    let mut staging = device.region_flash(MockRegion::Staging);
    let mut target_flash = device.region_flash(MockRegion::Slot(target));

    assert_eq!(stage_update(&mut staging, &update_image()).unwrap(), None);
    // The new image is broken in a way verification can't see: it flashes
    // fine but the device never asserts boot-complete running it.
    device.set_boots_after(None);

    let outcome = activate_update(
        &mut control,
        &monitor,
        &mut slots,
        &mut staging,
        &mut target_flash,
        target,
        10,
    )
    .expect("update flow failed");

    assert_eq!(outcome, UpdateOutcome::RolledBack);
    assert_eq!(device.rollback_count(), 1);
    assert_eq!(device.commit_count(), 0);
    assert_eq!(device.committed_slot(), old_slot);
    // The recovery reboot went back to the old slot.
    assert_eq!(device.last_boot_slot(), Some(old_slot));
    assert!(!device.is_in_reset());
}

/// Negative control for the instrumentation itself: a deliberately careless
/// driver that breaks the update rules must be *caught* by the device's
/// detectors. Until the real orchestrator replaces the test-local drivers,
/// this is what keeps the other tests honest — it proves their assertions
/// would fail for an implementation that flashes a live device or commits
/// without boot confirmation, rather than passing vacuously.
#[test]
fn instrumentation_catches_a_driver_that_violates_the_update_rules() {
    let device = booted_device();
    let target = 1 - device.committed_slot();

    let mut control = HalBootControl::new(device.reset_line(), BMC_RESET);
    let mut slots = device.slot_store();
    let mut target_flash = device.region_flash(MockRegion::Slot(target));

    // Violation 1: write firmware into a bootable slot while the device is
    // running — no hold_in_reset first.
    assert!(!device.is_in_reset());
    target_flash
        .program(FlashAddress::new(0), &update_image())
        .unwrap();

    // Violation 2: arm the trial and commit it immediately after the
    // reboot, without waiting for any boot evidence.
    control.hold_in_reset().unwrap(); // clears the stale ready latch
    slots.set_trial(target).unwrap();
    control.release().unwrap();
    slots.commit().unwrap(); // no await_boot — nothing confirmed this image

    // Both detectors must have recorded the violations. If either of these
    // assertions ever fails, the positive tests above have lost their
    // teeth.
    assert_eq!(device.programs_while_running(), 1);
    assert_eq!(device.commit_preceded_by_ready(), Some(false));
}
