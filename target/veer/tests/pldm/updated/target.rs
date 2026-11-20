// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! The image that the PLDM firmware update test installs. It has no apps.
//!
//! The image passes if the MCU was reset for a hitless update, which is the
//! reset that Caliptra arranges when it activates a new MCU image.

#![no_std]
#![no_main]

use entry::exit;
use kernel_config::MCI_BASE;
use target_common::{declare_target, TargetInterface};
use veer_mci::{Mci, ResetReason};
use {codegen as _, console_backend as _};

pub struct Target {}

impl TargetInterface for Target {
    const NAME: &'static str = "Caliptra PLDM Updated Image";

    fn main() -> ! {
        // SAFETY: this code runs in the kernel, which can access the MCI
        // registers, and nothing else in this image uses the MCI driver.
        let mci = unsafe { Mci::new(MCI_BASE) };
        let reason = mci.reset_reason();
        pw_log::info!(
            "{} (version {}, reset reason: {})",
            pldm_update_ids::UPDATED as &str,
            VERSION as &str,
            reason.name() as &str
        );
        Self::shutdown(match reason {
            ResetReason::FirmwareHitlessUpdate => 0,
            _ => 1,
        });
    }

    fn shutdown(code: u32) -> ! {
        match code {
            0 => pw_log::info!("PASS"),
            _ => pw_log::info!("FAIL: {}", code as u32),
        }
        exit(code);
    }
}

/// The version string of this image in the update package.
const VERSION: &str = "2.0.0";

declare_target!(Target);
