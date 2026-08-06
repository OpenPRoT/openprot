// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Host integration test for the eRoT boot flow, black-box style: verify
//! the device's firmware, release it from reset only on a pass, poll for
//! boot completion — asserting only on externally visible outcomes. The
//! flow mirrors the orchestrator's effect order (VerifyFirmware ->
//! ReleaseReset -> await ready). No kernel, no QEMU.

use std::error::Error;

use fwmanager_api::{BootControl, BootMonitor, BootStatus};
use fwmanager_hal_adapters::{GpioBootMonitor, HalBootControl};
use hal_flash::{Flash, FlashAddress};
use openprot_hal_blocking::gpio_port::{ActivePolarity, GpioErrorKind};
use openprot_platform_mock::downstream_device::{MockDownstreamDevice, MockPinMask};
use openprot_platform_mock::system_control::MockSystemControlError;

// Board binding: BMC reset on controller line 7, boot-complete on GPIO
// line 4. Normally set in the board's devices.rs.
const BMC_RESET: u8 = 7;
const BMC_READY: MockPinMask = MockPinMask(1 << 4);

// Firmware image format for this test: 4 magic bytes, payload, and a final
// byte that makes the XOR over the whole image zero. Stands in for real
// signature verification.
const IMAGE_LEN: usize = 16;
const MAGIC: [u8; 4] = *b"OPRT";

fn valid_image() -> [u8; IMAGE_LEN] {
    let mut image = [0u8; IMAGE_LEN];
    image[..4].copy_from_slice(&MAGIC);
    image[4..IMAGE_LEN - 1].fill(0xAB);
    let checksum = image[..IMAGE_LEN - 1].iter().fold(0, |acc, b| acc ^ b);
    image[IMAGE_LEN - 1] = checksum;
    image
}

fn corrupt_image() -> [u8; IMAGE_LEN] {
    let mut image = valid_image();
    image[7] ^= 0x01;
    image
}

/// The eRoT's verification step: read the device's firmware over the flash
/// seam and check it.
fn firmware_is_valid<F: Flash>(flash: &mut F) -> Result<bool, F::Error> {
    let mut image = [0u8; IMAGE_LEN];
    flash.read(FlashAddress::new(0), &mut image)?;
    let checksum = image.iter().fold(0, |acc, b| acc ^ b);
    Ok(image[..4] == MAGIC && checksum == 0)
}

#[derive(Debug, PartialEq, Eq)]
enum BootOutcome {
    Booted,
    TimedOut,
    /// Firmware failed verification; the device was never released.
    FirmwareRejected,
}

/// The eRoT's boot flow for one managed device: hold it in reset, verify
/// its firmware, and only on a pass release it and poll for boot completion
/// within the budget. `TimedOut` is a policy outcome, not an error.
fn supervise_boot<C, M, F>(
    control: &mut C,
    monitor: &M,
    flash: &mut F,
    poll_budget: usize,
) -> Result<BootOutcome, Box<dyn Error>>
where
    C: BootControl,
    M: BootMonitor,
    F: Flash,
    C::Error: 'static,
    M::Error: 'static,
    F::Error: core::fmt::Debug,
{
    control.hold_in_reset()?;
    if !firmware_is_valid(flash).map_err(|e| format!("flash read failed: {e:?}"))? {
        return Ok(BootOutcome::FirmwareRejected);
    }
    control.release()?;
    for _ in 0..poll_budget {
        if monitor.boot_status()? == BootStatus::Booted {
            return Ok(BootOutcome::Booted);
        }
    }
    Ok(BootOutcome::TimedOut)
}

/// The eRoT's capability bindings for the device, as a board's devices.rs
/// would make them.
fn erot_for(
    device: &MockDownstreamDevice,
) -> (
    HalBootControl<openprot_platform_mock::downstream_device::MockDeviceResetLine<'_>>,
    openprot_platform_mock::downstream_device::MockDeviceReadyLine<'_>,
) {
    let control = HalBootControl::new(device.reset_line(), BMC_RESET);
    (control, device.ready_line(BMC_READY))
}

#[test]
fn valid_firmware_boots_within_budget() {
    let device = MockDownstreamDevice::held_in_reset(Some(3));
    device.load_firmware(&valid_image());
    let (mut control, ready) = erot_for(&device);
    let monitor = GpioBootMonitor::new(&ready, BMC_READY, ActivePolarity::ActiveHigh);

    let outcome =
        supervise_boot(&mut control, &monitor, &mut device.flash(), 10).expect("boot flow failed");

    assert_eq!(outcome, BootOutcome::Booted);
    assert!(!device.is_in_reset());
}

#[test]
fn corrupt_firmware_is_rejected_and_the_device_never_released() {
    let device = MockDownstreamDevice::held_in_reset(Some(0));
    device.load_firmware(&corrupt_image());
    let (mut control, ready) = erot_for(&device);
    let monitor = GpioBootMonitor::new(&ready, BMC_READY, ActivePolarity::ActiveHigh);

    let outcome =
        supervise_boot(&mut control, &monitor, &mut device.flash(), 10).expect("boot flow failed");

    assert_eq!(outcome, BootOutcome::FirmwareRejected);
    // Verification happened, release did not.
    assert!(device.flash_reads() > 0);
    assert!(device.is_in_reset());
    assert_eq!(device.polls_since_release(), 0);
}

#[test]
fn firmware_is_verified_before_the_device_is_released() {
    let device = MockDownstreamDevice::held_in_reset(Some(0));
    device.load_firmware(&valid_image());
    let (mut control, ready) = erot_for(&device);
    let monitor = GpioBootMonitor::new(&ready, BMC_READY, ActivePolarity::ActiveHigh);

    supervise_boot(&mut control, &monitor, &mut device.flash(), 10).expect("boot flow failed");

    assert_eq!(device.release_preceded_by_flash_read(), Some(true));
}

#[test]
fn a_hung_device_is_a_timeout_not_an_error() {
    let device = MockDownstreamDevice::held_in_reset(None);
    device.load_firmware(&valid_image());
    let (mut control, ready) = erot_for(&device);
    let monitor = GpioBootMonitor::new(&ready, BMC_READY, ActivePolarity::ActiveHigh);

    let outcome =
        supervise_boot(&mut control, &monitor, &mut device.flash(), 5).expect("boot flow failed");

    assert_eq!(outcome, BootOutcome::TimedOut);
}

// Evidence from a previous boot cycle must never read as Booted: the flow
// re-asserts reset first, and the reset line clears the latch.
#[test]
fn stale_boot_evidence_is_cleared_by_the_reset_cycle() {
    let device = MockDownstreamDevice::held_in_reset(None);
    device.load_firmware(&valid_image());
    device.latch_ready();
    let (mut control, ready) = erot_for(&device);
    let monitor = GpioBootMonitor::new(&ready, BMC_READY, ActivePolarity::ActiveHigh);

    let outcome =
        supervise_boot(&mut control, &monitor, &mut device.flash(), 5).expect("boot flow failed");

    assert_eq!(outcome, BootOutcome::TimedOut);
}

#[test]
fn a_failing_reset_controller_aborts_before_release() {
    let device = MockDownstreamDevice::held_in_reset(Some(0));
    device.load_firmware(&valid_image());
    device.inject_reset_fault(MockSystemControlError::HardwareFailure);
    let (mut control, ready) = erot_for(&device);
    let monitor = GpioBootMonitor::new(&ready, BMC_READY, ActivePolarity::ActiveHigh);

    let err = supervise_boot(&mut control, &monitor, &mut device.flash(), 5)
        .expect_err("expected the reset fault");

    assert_eq!(err.to_string(), "boot control error: HardwareFailure");
    assert!(device.is_in_reset());
    assert_eq!(device.polls_since_release(), 0);
}

#[test]
fn a_monitor_fault_surfaces_with_kind_and_cause() {
    let device = MockDownstreamDevice::held_in_reset(Some(0));
    device.load_firmware(&valid_image());
    device.inject_gpio_fault(GpioErrorKind::HardwareFailure);
    let (mut control, ready) = erot_for(&device);
    let monitor = GpioBootMonitor::new(&ready, BMC_READY, ActivePolarity::ActiveHigh);

    let err = supervise_boot(&mut control, &monitor, &mut device.flash(), 5)
        .expect_err("expected the monitor fault");

    assert_eq!(err.to_string(), "boot monitor error: HardwareFailure");
    let cause = err
        .source()
        .expect("the concrete HAL error must be the cause");
    assert_eq!(cause.to_string(), "MockGpioFault(HardwareFailure)");
}
