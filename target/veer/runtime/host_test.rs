// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Boots the runtime image and checks that it has read the handoff table of
//! the MCU ROM, read the reset reason, recorded its milestone in the MCI and
//! stopped the watchdog that the ROM starts.
//!
//! The ROM's watchdog period is 800 million clock cycles, and the test does
//! not wait for it to pass. `//target/veer/tests/wdt` checks that a stopped
//! watchdog does not time out.
//!
//! The test then makes Caliptra's request for an MCU reset pending through
//! the MCI's trigger register and checks that the image resets the MCU: the
//! ROM runs again.

use std::time::{Duration, Instant};

use caliptra_mcu_hw_model::{McuHwModel, McuManager};
use test_boot_files::BootFiles;

/// Logged by `target.rs` when the kernel has started and the image has set up
/// the hardware.
const KERNEL_STARTED: &str = "kernel started";

/// Start of the line that `target.rs` logs when it has read a valid handoff
/// table.
const HANDOFF_READ: &str = "ROM handoff table 1.";

/// The line that `target.rs` logs for the reset reason. The ROM starts the
/// first image after a power-on with this reason.
const RESET_REASON: &str = "MCU reset reason: firmware boot";

/// `FIRMWARE_OS_INITIALIZED` in the milestone half of the MCI's firmware flow
/// status.
const OS_INITIALIZED: u32 = 1 << 24;

/// Start of every line that the ROM logs.
const ROM_LOG: &str = "[mcu-rom]";

const TIMEOUT: Duration = Duration::from_secs(120);

#[test]
fn sets_up_the_hardware_and_answers_a_reset_request() {
    let files = BootFiles::from_env();
    let mut hw =
        caliptra_mcu_hw_model::new(files.init_params()).expect("hardware model failed to boot");

    let start = Instant::now();
    let mut log = String::new();
    hw.step_until(|m| {
        let out = m.output().take(usize::MAX);
        print!("{out}");
        log.push_str(&out);
        log.contains(KERNEL_STARTED) || start.elapsed() > TIMEOUT
    });
    assert!(log.contains(KERNEL_STARTED), "the kernel did not start");
    assert!(
        log.contains(HANDOFF_READ),
        "the image did not read the ROM's handoff table"
    );
    assert!(
        log.contains(RESET_REASON),
        "the image did not log a firmware boot as the reset reason"
    );
    assert_ne!(
        hw.mci_flow_status() & OS_INITIALIZED,
        0,
        "the image did not record that the kernel started"
    );

    let (enabled, period, t1_timeout, t2_timeout, nmi) = hw.mcu_manager().with_mci(|mci| {
        let period = mci.wdt_timer1_timeout_period();
        let status = mci.wdt_status().read();
        (
            mci.wdt_timer1_en().read().timer1_en(),
            (u64::from(period.at(1).read()) << 32) | u64::from(period.at(0).read()),
            status.t1_timeout(),
            status.t2_timeout(),
            mci.hw_error_fatal().read().nmi_pin(),
        )
    });
    // The period is all ones after a reset, and the image does not change it.
    // Any other value is the period that the ROM set when it started the
    // watchdog.
    assert_ne!(period, u64::MAX, "the ROM did not start the watchdog");
    assert!(!enabled, "the watchdog is enabled");
    assert!(!t1_timeout, "watchdog timer 1 timed out");
    assert!(!t2_timeout, "watchdog timer 2 timed out");
    assert!(!nmi, "the watchdog raised an NMI");
    assert_eq!(hw.mci_fw_fatal_error(), None, "firmware hit a fatal error");

    hw.mcu_manager().with_mci(|mci| {
        mci.intr_block_rf()
            .notif0_intr_trig_r()
            .write(|w| w.notif_cptra_mcu_reset_req_trig(true));
    });
    let start = Instant::now();
    let mut log = String::new();
    hw.step_until(|m| {
        let out = m.output().take(usize::MAX);
        print!("{out}");
        log.push_str(&out);
        log.contains(ROM_LOG) || start.elapsed() > TIMEOUT
    });
    assert!(
        log.contains(ROM_LOG),
        "the image did not reset the MCU at Caliptra's request"
    );
    let pending = hw.mcu_manager().with_mci(|mci| {
        mci.intr_block_rf()
            .notif0_internal_intr_r()
            .read()
            .notif_cptra_mcu_reset_req_sts()
    });
    assert!(!pending, "the image did not acknowledge Caliptra's request");
}
