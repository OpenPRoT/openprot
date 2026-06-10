// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! AST10x0 SPI monitor configuration hardware test.

#![no_std]
#![no_main]

use core::cell::UnsafeCell;

use ast10x0_board::{
    apply_spim_external_mux, apply_spim_pinctrl, spim_external_mux_state,
};
use ast10x0_peripherals::scu::{
    ScuError, ScuExtMuxSelect, ScuRegisters, SpiMonitorInstance, SpiMonitorPassthrough,
    SpiMonitorSource,
};
use ast10x0_peripherals::spimonitor::{
    ConfiguredSpiMonitor, LockState, MonitorPolicy, MonitorState, PassthroughMode,
    PrivilegeDirection, PrivilegeOp, SpiMonitor, SpiMonitorController, SpiMonitorError,
    Uninitialized,
};
use console_backend::console_backend_write_all;
use target_common::{declare_target, TargetInterface};
use {console_backend as _, entry as _};

pub struct Target {}

const SPIM: SpiMonitorInstance = SpiMonitorInstance::Spim0;
const PROTECTED_LENGTH: u32 = 0x0010_0000;
const LOG_RAM_BYTES: usize = 0x200;
const LOG_RAM_WORDS: usize = LOG_RAM_BYTES / core::mem::size_of::<u32>();
const COMMAND_VALID_MASK: u32 = (1 << 30) | (1 << 31);
const COMMAND_LOCKED: u32 = 1 << 23;
const LOCK_REQUIRED: u32 = (1 << 0) | (1 << 1) | (1 << 4) | (1 << 5) | (1 << 30) | (1 << 31);

#[repr(align(16))]
struct LogRam(UnsafeCell<[u32; LOG_RAM_WORDS]>);

// SAFETY: This test is single-threaded and gives the buffer exclusively to SPIPF.
unsafe impl Sync for LogRam {}

static LOG_RAM: LogRam = LogRam(UnsafeCell::new([0; LOG_RAM_WORDS]));

#[derive(Clone, Copy)]
enum TestError {
    Monitor,
    Scu,
    Check,
}

impl From<SpiMonitorError> for TestError {
    fn from(_: SpiMonitorError) -> Self {
        Self::Monitor
    }
}

impl From<ScuError> for TestError {
    fn from(_: ScuError) -> Self {
        Self::Scu
    }
}

macro_rules! test_check {
    ($condition:expr, $message:literal) => {
        if !$condition {
            pw_log::info!($message);
            return Err(TestError::Check);
        }
    };
}

fn log_buffer() -> &'static mut [u32] {
    // SAFETY: The test is single-threaded and calls this exactly once.
    unsafe { &mut *LOG_RAM.0.get() }
}

fn configure_wiring(scu: &ScuRegisters) -> Result<(), TestError> {
    pw_log::info!("START: external monitor pinctrl, routing, and mux");
    apply_spim_pinctrl(scu, SPIM);
    scu.disable_spim_cs_internal_pull_down(SPIM);

    // SPIPF observes external BMC/host traffic. Do not detour SPI1 or SPI2
    // internally into the monitor.
    scu.set_spim_internal_mux(SpiMonitorSource::Spi1, 0)?;
    test_check!(
        scu.route_control_raw() & 0x0f == 0,
        "FAIL: internal SPI master detour is enabled"
    );

    // This SCU bit enables the SPIPF signal path. Controller bypass/filtering
    // is controlled independently by SPIPF000[1:0].
    scu.set_spim_passthrough(SPIM, SpiMonitorPassthrough::Enabled);
    scu.set_spim_miso_multi_func(SPIM, true);
    scu.set_spim_filter(SPIM, true);

    apply_spim_external_mux(SPIM, ScuExtMuxSelect::Mux0);
    test_check!(
        spim_external_mux_state(SPIM) == Some(ScuExtMuxSelect::Mux0),
        "FAIL: external mux 0 GPIO readback"
    );
    apply_spim_external_mux(SPIM, ScuExtMuxSelect::Mux1);
    test_check!(
        spim_external_mux_state(SPIM) == Some(ScuExtMuxSelect::Mux1),
        "FAIL: external mux 1 GPIO readback"
    );
    scu.set_spim_ext_mux(SPIM, ScuExtMuxSelect::Mux1);

    test_check!(
        scu.route_control_raw() & 0x0f == 0,
        "FAIL: external monitor setup changed internal SPI routing"
    );
    pw_log::info!("PASS: external monitor pinctrl, routing, and mux");
    Ok(())
}

fn build_policy() -> MonitorPolicy {
    let mut policy = MonitorPolicy::empty();
    policy.allow_commands[..9]
        .copy_from_slice(&[0x9f, 0x05, 0x06, 0x04, 0x02, 0x12, 0x20, 0x21, 0x0c]);
    policy.allow_command_count = 9;
    let _ = policy.add_region(
        0,
        PROTECTED_LENGTH,
        PrivilegeDirection::Write,
        PrivilegeOp::Disable,
    );
    policy
}

fn initialize_monitor() -> Result<ConfiguredSpiMonitor, TestError> {
    pw_log::info!("START: monitor reset, policy, and log RAM");
    let monitor = unsafe { SpiMonitor::<Uninitialized>::new(SpiMonitorController::Spim0) };
    monitor.software_reset();
    test_check!(
        monitor.regs().read_ctrl() & (1 << 15) == 0,
        "FAIL: software reset did not deassert"
    );

    let configured = monitor.apply_policy(&build_policy())?;
    test_check!(
        configured.state() == MonitorState::Configured,
        "FAIL: monitor did not enter configured state"
    );

    configured.configure_log(log_buffer())?;
    test_check!(
        configured.regs().read_log_capacity_entries() as usize == LOG_RAM_WORDS,
        "FAIL: 0x200-byte log RAM configuration"
    );
    test_check!(
        configured.regs().read_log_idx_reg() == 0,
        "FAIL: violation log pointer was not reset"
    );

    configured.set_push_pull(true);
    configured.set_passthrough(PassthroughMode::Disabled);
    configured.enable();
    test_check!(
        configured.regs().read_ctrl() & (1 << 2) != 0,
        "FAIL: monitor filter is not enabled"
    );
    pw_log::info!("PASS: monitor reset, policy, and log RAM");
    Ok(configured)
}

fn test_passthrough_control(configured: &ConfiguredSpiMonitor) -> Result<(), TestError> {
    pw_log::info!("START: passthrough control readback");
    configured.set_passthrough(PassthroughMode::Enabled);
    test_check!(
        configured.regs().read_ctrl() & 0x3 == 0x1,
        "FAIL: single-bit passthrough readback"
    );
    configured.set_passthrough(PassthroughMode::MultiEnabled);
    test_check!(
        configured.regs().read_ctrl() & 0x3 == 0x2,
        "FAIL: multi-bit passthrough readback"
    );
    configured.set_passthrough(PassthroughMode::Disabled);
    test_check!(
        configured.regs().read_ctrl() & 0x3 == 0,
        "FAIL: passthrough disable readback"
    );
    pw_log::info!("PASS: passthrough control readback");
    Ok(())
}

fn test_command_policy(configured: &ConfiguredSpiMonitor) -> Result<(), TestError> {
    pw_log::info!("START: command table add and remove");
    configured.remove_command(0x05)?;
    let status_slot = configured.add_command(0x05, false)?;
    test_check!(
        configured.regs().read_allow_cmd_slot(status_slot) & COMMAND_VALID_MASK != 0,
        "FAIL: command add readback"
    );
    configured.remove_command(0x05)?;
    test_check!(
        configured.regs().read_allow_cmd_slot(status_slot) == 0,
        "FAIL: command remove did not clear slot"
    );
    configured.add_command(0x05, false)?;
    pw_log::info!("PASS: command table add and remove");
    Ok(())
}

fn test_address_policy(configured: &ConfiguredSpiMonitor) -> Result<(), TestError> {
    pw_log::info!("START: protected and unprotected address policy");
    test_check!(
        configured.privilege_word(PrivilegeDirection::Write, 0)? == 0,
        "FAIL: protected write region programming"
    );
    test_check!(
        configured.privilege_word(PrivilegeDirection::Write, 2)? == u32::MAX,
        "FAIL: unprotected write region programming"
    );
    pw_log::info!("PASS: protected and unprotected address policy");
    Ok(())
}

fn test_policy_locking(configured: ConfiguredSpiMonitor) -> Result<(), TestError> {
    pw_log::info!("START: policy locking and readback");
    configured.lock_command(0x9f)?;
    let locked = configured.lock()?;
    test_check!(
        locked.lock_state() == LockState::Locked,
        "FAIL: monitor lock state"
    );
    test_check!(
        locked.regs().read_lock_status() & LOCK_REQUIRED == LOCK_REQUIRED,
        "FAIL: lock register readback"
    );
    let slot = locked.regs().read_allow_cmd_slot(2);
    locked.regs().write_allow_cmd_slot(2, 0);
    test_check!(
        locked.regs().read_allow_cmd_slot(2) == slot && slot & COMMAND_LOCKED != 0,
        "FAIL: command lock did not prevent modification"
    );
    pw_log::info!("PASS: policy locking and readback");
    Ok(())
}

fn run_spimonitor_test() -> Result<(), TestError> {
    pw_log::info!("=== AST10x0 external SPI monitor configuration test ===");

    let scu = unsafe { ScuRegisters::new_global_unlocked() };
    configure_wiring(&scu)?;
    let configured = initialize_monitor()?;
    test_passthrough_control(&configured)?;
    test_command_policy(&configured)?;
    test_address_policy(&configured)?;
    test_policy_locking(configured)?;

    pw_log::info!("External traffic blocking requires a BMC/host stimulus");
    pw_log::info!("=== all SPI monitor configuration tests passed ===");
    Ok(())
}

impl TargetInterface for Target {
    const NAME: &'static str = "AST10x0 External SPI Monitor Configuration Test";

    fn main() -> ! {
        let sentinel = if run_spimonitor_test().is_ok() {
            b"TEST_RESULT:PASS\n"
        } else {
            b"TEST_RESULT:FAIL\n"
        };
        let _ = console_backend_write_all(sentinel);

        #[expect(clippy::empty_loop)]
        loop {}
    }
}

declare_target!(Target);
