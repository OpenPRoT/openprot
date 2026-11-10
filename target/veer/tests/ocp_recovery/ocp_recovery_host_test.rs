// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Host side of the OCP recovery test. Boots the firmware in `target.rs` and
//! plays the recovery agent (the BMC) for one image. `agent::serve` is
//! different for each platform.

use caliptra_mcu_hw_model::McuHwModel;
use test_boot_files::BootFiles;

#[cfg(target_arch = "aarch64")]
#[path = "fpga_agent.rs"]
mod agent;

#[cfg(not(target_arch = "aarch64"))]
#[path = "emulator_agent.rs"]
mod agent;

#[test]
fn ocp_recovery() {
    let files = BootFiles::from_env();
    let mut hw = caliptra_mcu_hw_model::new(files.init_params()).expect("failed to boot");
    let image: Vec<u32> = (0..image::WORDS).map(image::word).collect();
    agent::serve(&mut hw, &image);
    hw.step_until_exit_success()
        .expect("firmware did not exit with PASS");
}
