// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Recovery agent for the FPGA: caliptra-sw's BMC, which sends the image over
//! I3C as it does for the boot images.

use caliptra_mcu_hw_model::{DefaultHwModel, McuHwModel};

const AWAITING_IMAGE: u32 = 1;

/// Queues `image` with the BMC and restarts it once the firmware asks for it.
///
/// The BMC still holds the three boot images, so the test image is at index
/// 3. After boot the BMC stops polling, and its state machine moves back to
/// sending an image when the device reports recovery mode again.
///
/// The BMC stops for good when it reads `RECOVERY_STATUS` 3 (recovery
/// successful). The device still reports 3 after boot, so the BMC is only
/// restarted after the firmware has set "awaiting image".
pub fn serve(hw: &mut DefaultHwModel, image: &[u32]) {
    let bytes = image.iter().flat_map(|w| w.to_le_bytes()).collect();
    hw.base.bmc.push_recovery_image(bytes);
    while recovery_status(hw) != AWAITING_IMAGE {
        if let Some(status) = hw.output().exit_status() {
            panic!("firmware exited with {status:?} before asking for an image");
        }
        hw.step();
    }
    hw.start_recovery_bmc();
}

/// Returns `RECOVERY_STATUS.DEV_REC_STATUS` as the BMC would read it.
fn recovery_status(hw: &mut DefaultHwModel) -> u32 {
    hw.base
        .i3c_core()
        .expect("FPGA model has no i3c-core")
        .sec_fw_recovery_if()
        .recovery_status()
        .read()
        .dev_rec_status()
}
