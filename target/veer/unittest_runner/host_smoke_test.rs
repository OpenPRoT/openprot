// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Host-driven test exercising `caliptra_mcu_hw_model` (`ModelEmulated`
//! on x86_64, `ModelFpgaRealtime` on VCK190 aarch64) against the shared
//! `//target/veer/unittest_runner` MCU firmware image.

use caliptra_mcu_hw_model::{McuHwModel, McuManager};
use test_boot_files::BootFiles;

#[test]
fn probe_mci_and_boot() {
    let files = BootFiles::from_env();
    let mut hw = caliptra_mcu_hw_model::new_unbooted(files.init_params())
        .expect("hardware model failed to initialize");
    let (hw_rev_id, wires0, wires1, security_state) = hw.mcu_manager().with_mci(|mci| {
        (
            u32::from(mci.hw_rev_id().read()),
            mci.generic_input_wires().at(0).read(),
            mci.generic_input_wires().at(1).read(),
            u32::from(mci.security_state().read()),
        )
    });
    eprintln!(
        "MCI hw_rev_id={hw_rev_id:#010x} generic_input_wires=[{wires0:#010x}, {wires1:#010x}] security_state={security_state:#010x}"
    );
    assert_ne!(hw_rev_id, 0, "expected non-zero MCI hw_rev_id");

    hw.boot().expect("hardware model failed to boot");
    hw.step_until_exit_success()
        .expect("MCU firmware did not exit with PASS");
}
