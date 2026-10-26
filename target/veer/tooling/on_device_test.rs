// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Shared host-side test entry point for on-device VeeR tests
//! (`caliptra_on_device_test`).
//!
//! Boots the configured MCU firmware image through `caliptra_mcu_hw_model`
//! (`ModelEmulated` on x86_64, `ModelFpgaRealtime` on VCK190 aarch64) and
//! steps it until the image exits with PASS or FAIL. MCU UART output is logged
//! by the hardware model's `log_writer` (stdout).

use caliptra_mcu_hw_model::McuHwModel;
use test_boot_files::BootFiles;

#[test]
fn on_device_test() {
    let files = BootFiles::from_env();
    let mut hw =
        caliptra_mcu_hw_model::new(files.init_params()).expect("hardware model failed to boot");
    hw.step_until_exit_success()
        .expect("on-device test firmware did not exit with PASS");
}
