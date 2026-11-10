// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! On-device test for the OCP recovery HAL and the i3c-core driver.
//!
//! Asks the recovery agent for image [`image::INDEX`], checks every word
//! against [`image::word`], waits for activation and reports success. The
//! host test plays the agent.

#![no_std]
#![no_main]

use entry::exit;
use hal_ocp_recovery::{ImageIndex, RecoveryDevice, RecoveryReason};
use target_common::{declare_target, TargetInterface};
use util_types::Blocking;
use veer_ocp_recovery::{I3cRecovery, MmioBlock};
use {codegen as _, console_backend as _};

const INDEX: ImageIndex = match ImageIndex::new(image::INDEX) {
    Some(index) => index,
    None => panic!("image index must be below 16"),
};

/// Waits for the agent by spinning.
struct Spin;

impl Blocking for Spin {
    fn wait_for_notification(&self) {
        core::hint::spin_loop();
    }
}

enum Failure {
    Bypass,
    Driver(util_error::ErrorCode),
    Length(u32),
    Mismatch(u32),
    Short(u32),
}

impl From<util_error::ErrorCode> for Failure {
    fn from(e: util_error::ErrorCode) -> Self {
        Failure::Driver(e)
    }
}

fn run() -> Result<(), Failure> {
    // SAFETY: I3C_BASE is the i3c-core register block in this target's
    // memory map, and nothing else in this image uses it.
    let driver = I3cRecovery::new(unsafe { MmioBlock::new(kernel_config::I3C_BASE) });
    if !driver.i3c_enabled() {
        return Err(Failure::Bypass);
    }
    let mut device = RecoveryDevice {
        driver,
        blocking: Spin,
    };
    let index = INDEX;

    pw_log::info!("Requesting recovery image {}", image::INDEX as u32);
    let mut transfer = device.request_image(index, RecoveryReason::FORCED_RECOVERY)?;
    if transfer.len_words() != image::WORDS {
        return Err(Failure::Length(transfer.len_words()));
    }

    let mut offset = 0u32;
    let mut buf = [0u32; 32];
    while transfer.remaining_words() > 0 {
        let n = transfer.read(&mut buf)?;
        for word in buf.iter().take(n) {
            if *word != image::word(offset) {
                return Err(Failure::Mismatch(offset));
            }
            offset += 1;
        }
    }
    if offset != image::WORDS {
        return Err(Failure::Short(offset));
    }

    pw_log::info!("Received {} words; waiting for activation", offset as u32);
    transfer.wait_for_activation()?;
    device.complete(index)?;
    Ok(())
}

pub struct Target {}

impl TargetInterface for Target {
    const NAME: &'static str = "Caliptra-MCU OCP Recovery";

    fn main() -> ! {
        match run() {
            Ok(()) => {
                pw_log::info!("PASS");
                exit(0);
            }
            Err(Failure::Bypass) => pw_log::error!("FAIL: recovery interface is in AXI bypass"),
            Err(Failure::Driver(e)) => pw_log::error!("FAIL: driver error {}", u32::from(e) as u32),
            Err(Failure::Length(n)) => pw_log::error!("FAIL: image is {} words", n as u32),
            Err(Failure::Mismatch(i)) => pw_log::error!("FAIL: word {} does not match", i as u32),
            Err(Failure::Short(n)) => pw_log::error!("FAIL: only {} words read", n as u32),
        }
        exit(1);
    }
}

declare_target!(Target);
