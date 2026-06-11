// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Configure SPIM1 through SPIM4 and keep all monitors active.

#![no_std]
#![no_main]

use core::cell::UnsafeCell;

#[path = "test_common.rs"]
mod test_common;

use ast10x0_board::{delay_us, enable_flash_power, set_bmc_resets};
use ast10x0_peripherals::scu::{ScuRegisters, SpiMonitorInstance};
use ast10x0_peripherals::spimonitor::{
    MonitorPolicy, PrivilegeDirection, PrivilegeOp, SpiMonitorController,
};
use target_common::{declare_target, TargetInterface};
use {console_backend as _, entry as _};

struct Spim1Config;
struct Spim2Config;
struct Spim3Config;
struct Spim4Config;

impl test_common::TestConfig for Spim1Config {
    const INSTANCE: SpiMonitorInstance = SpiMonitorInstance::Spim0;
    const CONTROLLER: SpiMonitorController = SpiMonitorController::Spim0;
}

impl test_common::TestConfig for Spim2Config {
    const INSTANCE: SpiMonitorInstance = SpiMonitorInstance::Spim1;
    const CONTROLLER: SpiMonitorController = SpiMonitorController::Spim1;
}

impl test_common::TestConfig for Spim3Config {
    const INSTANCE: SpiMonitorInstance = SpiMonitorInstance::Spim2;
    const CONTROLLER: SpiMonitorController = SpiMonitorController::Spim2;
}

impl test_common::TestConfig for Spim4Config {
    const INSTANCE: SpiMonitorInstance = SpiMonitorInstance::Spim3;
    const CONTROLLER: SpiMonitorController = SpiMonitorController::Spim3;
}

#[repr(align(16))]
struct LogRam(UnsafeCell<[u32; test_common::LOG_RAM_WORDS]>);

// SAFETY: Each buffer is exclusively assigned to one SPIPF instance.
unsafe impl Sync for LogRam {}

static SPIM1_LOG: LogRam = LogRam(UnsafeCell::new([0; test_common::LOG_RAM_WORDS]));
static SPIM2_LOG: LogRam = LogRam(UnsafeCell::new([0; test_common::LOG_RAM_WORDS]));
static SPIM3_LOG: LogRam = LogRam(UnsafeCell::new([0; test_common::LOG_RAM_WORDS]));
static SPIM4_LOG: LogRam = LogRam(UnsafeCell::new([0; test_common::LOG_RAM_WORDS]));

const WRITE_PROTECTED_LENGTH: u32 = 0x0010_0000;
const ALLOW_COMMANDS: [u8; 32] = [
    0x03, 0x13, 0x0b, 0x0c, 0x6b, 0x6c, 0x01, 0x05, 0x35, 0x06, 0x04, 0x20, 0x21, 0x9f, 0x5a,
    0xb7, 0xe9, 0x32, 0x34, 0xd8, 0xdc, 0x02, 0x12, 0x3b, 0x3c, 0x70, 0xbb, 0xbc, 0x50, 0xeb,
    0xec, 0xc2,
];
const SPIM4_ALLOW_COMMANDS: [u8; 15] = [
    0x01, 0x06, 0x04, 0x20, 0x21, 0xb7, 0xe9, 0x32, 0x34, 0xd8, 0xdc, 0x02, 0x12, 0x50, 0xc2,
];

fn log_buffer(log: &'static LogRam) -> &'static mut [u32] {
    // SAFETY: The caller assigns each static buffer to exactly one monitor.
    unsafe { &mut *log.0.get() }
}

fn production_policy() -> MonitorPolicy {
    let mut policy = MonitorPolicy::empty();
    policy.allow_commands.copy_from_slice(&ALLOW_COMMANDS);
    policy.allow_command_count = ALLOW_COMMANDS.len();
    let _ = policy.add_region(
        0,
        WRITE_PROTECTED_LENGTH,
        PrivilegeDirection::Write,
        PrivilegeOp::Disable,
    );
    policy
}

fn spim4_test_policy() -> MonitorPolicy {
    let mut policy = MonitorPolicy::empty();
    policy.allow_commands[..SPIM4_ALLOW_COMMANDS.len()]
        .copy_from_slice(&SPIM4_ALLOW_COMMANDS);
    policy.allow_command_count = SPIM4_ALLOW_COMMANDS.len();
    let _ = policy.add_region(
        0,
        WRITE_PROTECTED_LENGTH,
        PrivilegeDirection::Write,
        PrivilegeOp::Disable,
    );
    policy
}

fn setup_all_spim() -> Result<(), test_common::TestError> {
    let scu = unsafe { ScuRegisters::new_global_unlocked() };
    let policy = production_policy();
    let spim4_policy = spim4_test_policy();

    pw_log::info!("=== GPIO flash power ===");
    if !enable_flash_power(&scu) {
        pw_log::info!("FAIL: GPIOL2/GPIOL3 flash power readback");
        return Err(test_common::TestError::Check);
    }
    pw_log::info!("PASS: GPIOL2 and GPIOL3 set high");

    pw_log::info!("=== Hold BMC in reset ===");
    if !set_bmc_resets(true) {
        pw_log::info!("FAIL: SGPIOM BMC reset outputs did not assert");
        return Err(test_common::TestError::Check);
    }
    pw_log::info!("PASS: SGPIOM outputs 8 and 9 asserted low");

    pw_log::info!("=== SPIM1 ===");
    test_common::configure_wiring::<Spim1Config>(&scu)?;
    let spim1 = test_common::initialize_monitor_with_policy::<Spim1Config>(
        log_buffer(&SPIM1_LOG),
        &policy,
    )?;
    test_common::dump_policy(&spim1, WRITE_PROTECTED_LENGTH)?;
    let _spim1 = test_common::lock_monitor(spim1)?;

    pw_log::info!("=== SPIM2 ===");
    test_common::configure_wiring::<Spim2Config>(&scu)?;
    let spim2 = test_common::initialize_monitor_with_policy::<Spim2Config>(
        log_buffer(&SPIM2_LOG),
        &policy,
    )?;
    test_common::dump_policy(&spim2, WRITE_PROTECTED_LENGTH)?;
    let _spim2 = test_common::lock_monitor(spim2)?;

    pw_log::info!("=== SPIM3 ===");
    test_common::configure_wiring::<Spim3Config>(&scu)?;
    let spim3 = test_common::initialize_monitor_with_policy::<Spim3Config>(
        log_buffer(&SPIM3_LOG),
        &policy,
    )?;
    test_common::dump_policy(&spim3, WRITE_PROTECTED_LENGTH)?;
    let _spim3 = test_common::lock_monitor(spim3)?;

    pw_log::info!("=== SPIM4 ===");
    pw_log::info!("SPIM4 test policy blocks all read commands");
    test_common::configure_wiring::<Spim4Config>(&scu)?;
    let spim4 = test_common::initialize_monitor_with_policy::<Spim4Config>(
        log_buffer(&SPIM4_LOG),
        &spim4_policy,
    )?;
    test_common::dump_policy(&spim4, WRITE_PROTECTED_LENGTH)?;
    let _spim4 = test_common::lock_monitor(spim4)?;

    pw_log::info!("All SPIM1-4 monitors are configured and locked");
    pw_log::info!(
        "SCU0F0 after SPIM setup: 0x{:08x}",
        scu.route_control_raw() as u32
    );

    pw_log::info!("Waiting 60 ms for flash routing to settle");
    delay_us(60_000);

    pw_log::info!("=== Release BMC reset ===");
    if !set_bmc_resets(false) {
        pw_log::info!("FAIL: SGPIOM BMC reset outputs did not release");
        return Err(test_common::TestError::Check);
    }
    pw_log::info!("PASS: SGPIOM outputs 8 and 9 released high");

    pw_log::info!("Firmware will remain active until the user stops or resets it");
    Ok(())
}

struct Target;

impl TargetInterface for Target {
    const NAME: &'static str = "AST10x0 Setup All External SPI Monitors";

    fn main() -> ! {
        if setup_all_spim().is_err() {
            pw_log::info!("FAIL: setup all SPIM monitors");
        }

        #[expect(clippy::empty_loop)]
        loop {}
    }
}

declare_target!(Target);
