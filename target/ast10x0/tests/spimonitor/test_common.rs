// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Shared AST10x0 external SPI monitor configuration tests.

#![allow(dead_code)]

use core::cell::UnsafeCell;

use ast10x0_board::{apply_spim_external_mux, apply_spim_pinctrl, spim_external_mux_state};
use ast10x0_peripherals::scu::{
    ScuError, ScuExtMuxSelect, ScuRegisters, SpiMonitorInstance, SpiMonitorPassthrough,
    SpiMonitorSource,
};
use ast10x0_peripherals::spimonitor::{
    ConfiguredSpiMonitor, LockState, MonitorPolicy, MonitorState, PassthroughMode,
    PrivilegeDirection, PrivilegeOp, SpiMonitor, SpiMonitorController, SpiMonitorError,
    Uninitialized,
};

pub const PROTECTED_LENGTH: u32 = 0x0010_0000;
const LOG_RAM_BYTES: usize = 0x200;
pub const LOG_RAM_WORDS: usize = LOG_RAM_BYTES / core::mem::size_of::<u32>();
const COMMAND_VALID_MASK: u32 = (1 << 30) | (1 << 31);
const COMMAND_LOCKED: u32 = 1 << 23;
const LOCK_REQUIRED: u32 = (1 << 0) | (1 << 1) | (1 << 4) | (1 << 5) | (1 << 30) | (1 << 31);

pub trait TestConfig {
    const INSTANCE: SpiMonitorInstance;
    const CONTROLLER: SpiMonitorController;
}

#[repr(align(16))]
struct LogRam(UnsafeCell<[u32; LOG_RAM_WORDS]>);

// SAFETY: Each test image is single-threaded and owns one SPIPF log buffer.
unsafe impl Sync for LogRam {}

static LOG_RAM: LogRam = LogRam(UnsafeCell::new([0; LOG_RAM_WORDS]));

#[derive(Clone, Copy)]
pub enum TestError {
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

pub fn configure_wiring<C: TestConfig>(scu: &ScuRegisters) -> Result<(), TestError> {
    pw_log::info!("START: external monitor pinctrl, routing, and mux");
    apply_spim_pinctrl(scu, C::INSTANCE);
    scu.disable_spim_cs_internal_pull_down(C::INSTANCE);

    // SPIPF observes external BMC/host traffic. Do not detour SPI1 or SPI2.
    scu.set_spim_internal_mux(SpiMonitorSource::Spi1, 0)?;
    test_check!(
        scu.route_control_raw() & 0x0f == 0,
        "FAIL: internal SPI master detour is enabled"
    );

    // SCU passthrough enables the external signal path through SPIPF.
    scu.set_spim_passthrough(C::INSTANCE, SpiMonitorPassthrough::Enabled);
    scu.set_spim_miso_multi_func(C::INSTANCE, true);
    scu.set_spim_filter(C::INSTANCE, true);

    // This board provides ext-mux-sel-gpios. Match the Zephyr driver by
    // driving those GPIOs only; SCU0F0[15:12] is used only when no external
    // mux GPIOs are described.
    apply_spim_external_mux(C::INSTANCE, ScuExtMuxSelect::Mux1);
    test_check!(
        spim_external_mux_state(C::INSTANCE) == Some(ScuExtMuxSelect::Mux1),
        "FAIL: external mux 1 GPIO readback"
    );

    test_check!(
        scu.route_control_raw() & 0x0f == 0,
        "FAIL: external monitor setup changed internal SPI routing"
    );
    pw_log::info!("PASS: external monitor pinctrl, routing, and mux");
    Ok(())
}

pub fn build_policy() -> MonitorPolicy {
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

pub fn initialize_monitor<C: TestConfig>(
    buffer: &'static mut [u32],
) -> Result<ConfiguredSpiMonitor, TestError> {
    initialize_monitor_with_policy::<C>(buffer, &build_policy())
}

pub fn initialize_monitor_with_policy<C: TestConfig>(
    buffer: &'static mut [u32],
    policy: &MonitorPolicy,
) -> Result<ConfiguredSpiMonitor, TestError> {
    pw_log::info!("START: monitor reset, policy, and log RAM");
    let monitor = unsafe { SpiMonitor::<Uninitialized>::new(C::CONTROLLER) };
    monitor.software_reset();
    test_check!(
        monitor.regs().read_ctrl() & (1 << 15) == 0,
        "FAIL: software reset did not deassert"
    );

    let configured = monitor.apply_policy(policy)?;
    test_check!(
        configured.state() == MonitorState::Configured,
        "FAIL: monitor did not enter configured state"
    );

    configured.configure_log(buffer)?;
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

#[allow(dead_code)]
pub fn dump_policy(
    configured: &ConfiguredSpiMonitor,
    write_protected_length: u32,
) -> Result<(), TestError> {
    pw_log::info!("allow-command table:");
    for slot in 0..32 {
        let value = configured.regs().read_allow_cmd_slot(slot);
        if value != 0 {
            pw_log::info!(
                "  slot {:02}: cmd=0x{:02x}, raw=0x{:08x}",
                slot as u32,
                (value & 0xff) as u32,
                value as u32
            );
        }
    }

    pw_log::info!(
        "write-protected region: start=0x00000000, length=0x{:08x}",
        write_protected_length as u32
    );
    Ok(())
}

#[allow(dead_code)]
pub fn validate_filtering(configured: &ConfiguredSpiMonitor) -> Result<(), TestError> {
    test_check!(
        configured.regs().read_ctrl() & 0x7 == 0x4,
        "FAIL: SPIPF filtering mode readback"
    );
    pw_log::info!("PASS: SPIPF filtering mode");
    Ok(())
}

#[allow(dead_code)]
pub fn validate_one_mib_write_protection(
    configured: &ConfiguredSpiMonitor,
) -> Result<(), TestError> {
    test_check!(
        configured.privilege_word(PrivilegeDirection::Read, 0)? == u32::MAX
            && configured.privilege_word(PrivilegeDirection::Read, 1)? == u32::MAX
            && configured.privilege_word(PrivilegeDirection::Read, 2)? == u32::MAX,
        "FAIL: read privilege table is not unrestricted"
    );
    test_check!(
        configured.privilege_word(PrivilegeDirection::Write, 0)? == 0
            && configured.privilege_word(PrivilegeDirection::Write, 1)? == 0,
        "FAIL: first 1 MiB is not write protected"
    );
    test_check!(
        configured.privilege_word(PrivilegeDirection::Write, 2)? == u32::MAX,
        "FAIL: writes at and above 0x00100000 are not enabled"
    );
    pw_log::info!("PASS: reads allowed; writes below 0x00100000 blocked");
    pw_log::info!("PASS: writes at 0x00110000 allowed");
    Ok(())
}

#[allow(dead_code)]
pub fn lock_monitor(
    configured: ConfiguredSpiMonitor,
) -> Result<ast10x0_peripherals::spimonitor::LockedSpiMonitor, TestError> {
    Ok(configured.lock()?)
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

pub fn run<C: TestConfig>() -> Result<(), TestError> {
    pw_log::info!("=== AST10x0 external SPI monitor configuration test ===");

    let scu = unsafe { ScuRegisters::new_global_unlocked() };
    configure_wiring::<C>(&scu)?;
    let configured = initialize_monitor::<C>(log_buffer())?;
    test_passthrough_control(&configured)?;
    test_command_policy(&configured)?;
    test_address_policy(&configured)?;
    test_policy_locking(configured)?;

    pw_log::info!("External traffic blocking requires a BMC/host stimulus");
    pw_log::info!("=== all SPI monitor configuration tests passed ===");
    Ok(())
}
