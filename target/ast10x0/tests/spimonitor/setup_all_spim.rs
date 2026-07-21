// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Configure SPIM1 through SPIM4 and keep all monitors active.

#![no_std]
#![no_main]

use core::cell::UnsafeCell;

#[path = "test_common.rs"]
mod test_common;

use ast10x0_board::{
    apply_spim_external_mux, bmc_spim_csin_levels, bmc_spim_path_debug, delay_us,
    enable_flash_power, release_spi_flash_resets, set_bmc_resets, spim_external_mux_state,
};
use ast10x0_peripherals::scu::{
    pinctrl::PINCTRL_SPI1_QUAD, ScuExtMuxSelect, ScuRegisters, SpiMonitorInstance, SpiMonitorSource,
};
use ast10x0_peripherals::smc::{
    ChipSelect, FlashConfig, SmcConfig, SmcController, SmcError, SmcTopology, SpiReady, SpiUninit,
    TransferMode,
};
use ast10x0_peripherals::spimonitor::registers::SpiMonitorRegisters;
use ast10x0_peripherals::spimonitor::{
    ConfiguredSpiMonitor, LockedSpiMonitor, PrivilegeDirection, PrivilegeOp, SpiMonitorController,
    SpiMonitorPolicy, ViolationLogEntry,
};
use target_common::{declare_target, TargetInterface};
use test_common::TestConfig;
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
const MONITOR_POLL_INTERVAL_US: u32 = 10_000;
const BMC_CSIN_RECOVERY_TIMEOUT_US: u32 = 500_000;
const BMC_RECOVERY_RETRY_DELAY_US: u32 = 1_000_000;
const BMC_CSIN_MASK: u32 = (1 << 0) | (1 << 14);
const ENABLE_RUNTIME_DEBUG_LOGS: bool = false;
const BMC_FLASH_CONFIG: FlashConfig = FlashConfig {
    capacity_mb: 128,
    page_size: 256,
    sector_size: 4096,
    block_size: 65536,
    spi_clock_mhz: 25,
};
const ALLOW_COMMANDS: [u8; 32] = [
    0x03, 0x13, 0x0b, 0x0c, 0x6b, 0x6c, 0x01, 0x05, 0x35, 0x06, 0x04, 0x20, 0x21, 0x9f, 0x5a, 0xb7,
    0xe9, 0x32, 0x34, 0xd8, 0xdc, 0x02, 0x12, 0x3b, 0x3c, 0x70, 0xbb, 0xbc, 0x50, 0xeb, 0xec, 0xc2,
];

fn log_buffer(log: &'static LogRam) -> &'static mut [u32] {
    // SAFETY: The caller assigns each static buffer to exactly one monitor.
    unsafe { &mut *log.0.get() }
}

fn raw_log_word(log: &'static LogRam, index: usize) -> u32 {
    // SAFETY: The index comes from the bounded SPIPF log pointer.
    unsafe { core::ptr::read_volatile((*log.0.get()).as_ptr().add(index)) }
}

fn log_violation(id: u32, index: u32, word: u32) {
    match ViolationLogEntry::parse(word) {
        ViolationLogEntry::BlockedCommand(command) => pw_log::info!(
            "SPIM{} BLOCKED COMMAND log[{}]: cmd=0x{:02x}, raw=0x{:08x}",
            id as u32,
            index as u32,
            command as u32,
            word as u32
        ),
        ViolationLogEntry::BlockedWriteAddr(address) => pw_log::info!(
            "SPIM{} BLOCKED WRITE log[{}]: address=0x{:08x}, raw=0x{:08x}",
            id as u32,
            index as u32,
            address as u32,
            word as u32
        ),
        ViolationLogEntry::BlockedReadAddr(address) => pw_log::info!(
            "SPIM{} BLOCKED READ log[{}]: address=0x{:08x}, raw=0x{:08x}",
            id as u32,
            index as u32,
            address as u32,
            word as u32
        ),
        ViolationLogEntry::Invalid(raw) => pw_log::info!(
            "SPIM{} INVALID LOG log[{}]: raw=0x{:08x}",
            id as u32,
            index as u32,
            raw as u32
        ),
    }
}

fn log_violation_status(id: u32, status: u32) {
    pw_log::info!(
        "SPIM{} status=0x{:08x}: command_blocked={}, write_blocked={}, read_blocked={}",
        id as u32,
        status as u32,
        (status & (1 << 0) != 0) as u32,
        (status & (1 << 1) != 0) as u32,
        (status & (1 << 2) != 0) as u32
    );
}

fn acknowledge_violations(regs: &SpiMonitorRegisters) -> u32 {
    let pending = regs.read_ctrl2() & 0x7;
    if pending != 0 {
        regs.modify_ctrl2(|bits| *bits |= pending);
    }
    pending
}

macro_rules! runtime_debug {
    ($($arg:tt)*) => {
        if ENABLE_RUNTIME_DEBUG_LOGS {
            pw_log::info!($($arg)*);
        }
    };
}

fn recover_bmc_path(
    scu: &ScuRegisters,
    spim1: &ConfiguredSpiMonitor,
    spim2: &ConfiguredSpiMonitor,
) {
    runtime_debug!("BMC recovery: asserting BMC reset");
    if !set_bmc_resets(true) {
        runtime_debug!("BMC recovery: reset assertion readback failed");
    }

    runtime_debug!("BMC recovery: routing SPIM1/2 flashes to RoT");
    apply_spim_external_mux(Spim1Config::INSTANCE, ScuExtMuxSelect::Mux1);
    delay_us(1_000);

    loop {
        if !set_bmc_resets(true) {
            runtime_debug!("BMC recovery: reset assertion readback failed");
        }
        if spim_external_mux_state(Spim1Config::INSTANCE) != Some(ScuExtMuxSelect::Mux1) {
            runtime_debug!(
                "BMC recovery: RoT mux readback failed; retrying in {} us",
                BMC_RECOVERY_RETRY_DELAY_US as u32
            );
            apply_spim_external_mux(Spim1Config::INSTANCE, ScuExtMuxSelect::Mux1);
            delay_us(BMC_RECOVERY_RETRY_DELAY_US);
            continue;
        }
        spim1.reset_filter_state();
        spim2.reset_filter_state();

        runtime_debug!("BMC recovery: resetting and probing spi1@0 and spi1@1");
        if reset_bmc_flashes(scu, false).is_err() {
            runtime_debug!(
                "BMC recovery: flash reset/probe failed; retrying in {} us",
                BMC_RECOVERY_RETRY_DELAY_US as u32
            );
            delay_us(BMC_RECOVERY_RETRY_DELAY_US);
            continue;
        }

        spim1.reset_filter_state();
        spim2.reset_filter_state();
        acknowledge_violations(spim1.regs());
        acknowledge_violations(spim2.regs());

        runtime_debug!("BMC recovery: handing SPIM1/2 flashes back to BMC");
        apply_spim_external_mux(Spim1Config::INSTANCE, ScuExtMuxSelect::Mux0);
        delay_us(60_000);
        if spim_external_mux_state(Spim1Config::INSTANCE) != Some(ScuExtMuxSelect::Mux0) {
            runtime_debug!(
                "BMC recovery: BMC mux readback failed; retrying in {} us",
                BMC_RECOVERY_RETRY_DELAY_US as u32
            );
            apply_spim_external_mux(Spim1Config::INSTANCE, ScuExtMuxSelect::Mux1);
            delay_us(BMC_RECOVERY_RETRY_DELAY_US);
            continue;
        }

        if !set_bmc_resets(false) {
            runtime_debug!(
                "BMC recovery: reset release readback failed; retrying in {} us",
                BMC_RECOVERY_RETRY_DELAY_US as u32
            );
            apply_spim_external_mux(Spim1Config::INSTANCE, ScuExtMuxSelect::Mux1);
            delay_us(BMC_RECOVERY_RETRY_DELAY_US);
            continue;
        }

        runtime_debug!("BMC recovery: complete");
        return;
    }
}

fn monitor_spim_violations(
    spim1: &ConfiguredSpiMonitor,
    spim2: &ConfiguredSpiMonitor,
    spim3: &LockedSpiMonitor,
    spim4: &LockedSpiMonitor,
) -> ! {
    let monitors = [
        (1u32, spim1.regs(), &SPIM1_LOG),
        (2u32, spim2.regs(), &SPIM2_LOG),
        (3u32, spim3.regs(), &SPIM3_LOG),
        (4u32, spim4.regs(), &SPIM4_LOG),
    ];
    let mut log_indices = [
        spim1.regs().read_log_idx_reg(),
        spim2.regs().read_log_idx_reg(),
        spim3.regs().read_log_idx_reg(),
        spim4.regs().read_log_idx_reg(),
    ];
    let mut poll_count = 0u32;
    let mut bmc_csin_low_us = 0u32;

    if ENABLE_RUNTIME_DEBUG_LOGS {
        for (id, regs, _) in monitors {
            pw_log::info!(
                "SPIM{} ctrl/status/lock=0x{:08x}/0x{:08x}/0x{:08x}",
                id as u32,
                regs.read_ctrl() as u32,
                regs.read_ctrl2() as u32,
                regs.read_lock_status() as u32
            );
        }
        pw_log::info!("Monitoring BMC and host traffic; SPIPF logs blocked transactions only");
    }

    loop {
        for (slot, (id, regs, log)) in monitors.iter().enumerate() {
            let next_index = regs.read_log_idx_reg();
            if next_index != log_indices[slot] {
                let status = regs.read_ctrl2();
                pw_log::info!(
                    "SPIM{} NEW VIOLATION: log {} -> {}",
                    *id as u32,
                    log_indices[slot] as u32,
                    next_index as u32
                );
                log_violation_status(*id, status);

                let end = next_index.min(test_common::LOG_RAM_WORDS as u32);
                for index in log_indices[slot]..end {
                    log_violation(*id, index, raw_log_word(log, index as usize));
                }

                let acknowledged = acknowledge_violations(regs);
                pw_log::info!(
                    "SPIM{} acknowledged status 0x{:08x}",
                    *id as u32,
                    acknowledged as u32
                );
                log_indices[slot] = next_index;
            }
        }

        let csin_levels = bmc_spim_csin_levels();
        if csin_levels != BMC_CSIN_MASK {
            bmc_csin_low_us = bmc_csin_low_us.saturating_add(MONITOR_POLL_INTERVAL_US);
            if bmc_csin_low_us >= BMC_CSIN_RECOVERY_TIMEOUT_US {
                runtime_debug!(
                    "BMC CSIN timeout: levels=0x{:08x}, low for {} us",
                    csin_levels as u32,
                    bmc_csin_low_us as u32
                );
                let scu = unsafe { ScuRegisters::new_global_unlocked() };
                recover_bmc_path(&scu, spim1, spim2);
                log_indices[0] = spim1.regs().read_log_idx_reg();
                log_indices[1] = spim2.regs().read_log_idx_reg();
                bmc_csin_low_us = 0;
            }
        } else {
            bmc_csin_low_us = 0;
        }

        poll_count = poll_count.wrapping_add(1);
        if ENABLE_RUNTIME_DEBUG_LOGS && poll_count % 100 == 0 {
            let scu = unsafe { ScuRegisters::new_global_unlocked() };
            let mux = match spim_external_mux_state(Spim1Config::INSTANCE) {
                Some(ScuExtMuxSelect::Mux0) => 0u32,
                Some(ScuExtMuxSelect::Mux1) => 1u32,
                None => 0xffff_ffff,
            };
            let host_mux = match spim_external_mux_state(Spim3Config::INSTANCE) {
                Some(ScuExtMuxSelect::Mux0) => 0u32,
                Some(ScuExtMuxSelect::Mux1) => 1u32,
                None => 0xffff_ffff,
            };
            let path = bmc_spim_path_debug();
            pw_log::info!(
                "SPIM health: SCU0F0=0x{:08x}, mux BMC/host={}/{}, CSIN=0x{:08x}",
                scu.route_control_raw() as u32,
                mux as u32,
                host_mux as u32,
                bmc_spim_csin_levels() as u32
            );
            pw_log::info!(
                "SPIM log_idx 1/2/3/4={}/{}/{}/{}",
                monitors[0].1.read_log_idx_reg() as u32,
                monitors[1].1.read_log_idx_reg() as u32,
                monitors[2].1.read_log_idx_reg() as u32,
                monitors[3].1.read_log_idx_reg() as u32
            );
            pw_log::info!(
                "SPIM status 1/2/3/4=0x{:08x}/0x{:08x}/0x{:08x}/0x{:08x}",
                monitors[0].1.read_ctrl2() as u32,
                monitors[1].1.read_ctrl2() as u32,
                monitors[2].1.read_ctrl2() as u32,
                monitors[3].1.read_ctrl2() as u32
            );
            pw_log::info!(
                "BMC path: SCU410/4B0/690=0x{:08x}/0x{:08x}/0x{:08x}, SCU610=0x{:08x}",
                path.scu410 as u32,
                path.scu4b0 as u32,
                path.scu690 as u32,
                path.scu610 as u32
            );
            pw_log::info!(
                "BMC path: GPIO000/004=0x{:08x}/0x{:08x}, SGPIOM570/554=0x{:08x}/0x{:08x}",
                path.gpio_data as u32,
                path.gpio_direction as u32,
                path.sgpio_latch as u32,
                path.sgpio_config as u32
            );
        }
        delay_us(MONITOR_POLL_INTERVAL_US);
    }
}

fn reset_one_bmc_flash(
    scu: &ScuRegisters,
    spi: &SpiReady,
    monitor: SpiMonitorInstance,
    chip_select: ChipSelect,
) -> Result<[u8; 3], SmcError> {
    scu.set_spim_internal_mux(SpiMonitorSource::Spi1, monitor as u8 + 1)
        .map_err(|_| SmcError::HardwareError)?;
    let proprietary_state = scu.spim_proprietary_pre_config();

    let result = (|| {
        spi.transceive_user(chip_select, &[0x66], &[], &mut [], TransferMode::Mode111)?;
        delay_us(10_000);
        spi.transceive_user(chip_select, &[0x99], &[], &mut [], TransferMode::Mode111)?;
        delay_us(50_000);
        spi.transceive_user(chip_select, &[0xe9], &[], &mut [], TransferMode::Mode111)?;

        let mut jedec = [0u8; 3];
        spi.transceive_user(chip_select, &[0x9f], &[], &mut jedec, TransferMode::Mode111)?;
        Ok(jedec)
    })();

    if let Some(state) = proprietary_state {
        scu.spim_proprietary_post_config(state);
    }
    scu.clear_spim_internal_master_route();
    result
}

fn reset_bmc_flashes(scu: &ScuRegisters, log_jedec: bool) -> Result<(), SmcError> {
    scu.apply_pinctrl_group(PINCTRL_SPI1_QUAD);
    let config = SmcConfig {
        controller_id: SmcController::Spi1,
        cs0: Some(BMC_FLASH_CONFIG),
        cs1: Some(BMC_FLASH_CONFIG),
        dma_enabled: false,
        enable_interrupts: false,
        topology: SmcTopology::HostSpi { master_idx: 0 },
    };
    let spi = unsafe { SpiUninit::new(SmcController::Spi1, config)? }.init()?;

    for (index, monitor, chip_select) in [
        (0u32, SpiMonitorInstance::Spim0, ChipSelect::Cs0),
        (1u32, SpiMonitorInstance::Spim1, ChipSelect::Cs1),
    ] {
        let jedec = reset_one_bmc_flash(scu, &spi, monitor, chip_select)?;
        if log_jedec {
            pw_log::info!(
                "spi1@{} reset complete, JEDEC ID: {:02x} {:02x} {:02x}",
                index as u32,
                jedec[0] as u32,
                jedec[1] as u32,
                jedec[2] as u32
            );
        }
        if matches!(jedec, [0x00, 0x00, 0x00] | [0xff, 0xff, 0xff]) {
            return Err(SmcError::HardwareError);
        }
    }
    Ok(())
}

fn production_policy() -> SpiMonitorPolicy {
    let mut policy = SpiMonitorPolicy::empty();
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

fn setup_all_spim() -> Result<(), test_common::TestError> {
    let scu = unsafe { ScuRegisters::new_global_unlocked() };
    let policy = production_policy();
    //let spim4_policy = spim4_test_policy();

    pw_log::info!("=== GPIO flash power ===");
    if !enable_flash_power(&scu) {
        pw_log::info!("FAIL: GPIOL2/GPIOL3 flash power readback");
        return Err(test_common::TestError::Check);
    }
    pw_log::info!("PASS: GPIOL2 and GPIOL3 set high");

    pw_log::info!("=== Release SPI flash reset outputs ===");
    if !release_spi_flash_resets() {
        pw_log::info!("FAIL: SGPIOM CPU/BMC SPI reset outputs did not release");
        return Err(test_common::TestError::Check);
    }
    pw_log::info!("PASS: SGPIOM outputs 6 and 7 released high");

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
    test_common::validate_one_mib_write_protection(&spim1)?;
    test_common::validate_filtering(&spim1)?;

    pw_log::info!("=== SPIM2 ===");
    test_common::configure_wiring::<Spim2Config>(&scu)?;
    let spim2 = test_common::initialize_monitor_with_policy::<Spim2Config>(
        log_buffer(&SPIM2_LOG),
        &policy,
    )?;
    test_common::dump_policy(&spim2, WRITE_PROTECTED_LENGTH)?;
    test_common::validate_one_mib_write_protection(&spim2)?;
    test_common::validate_filtering(&spim2)?;

    pw_log::info!("=== SPIM3 ===");
    test_common::configure_wiring::<Spim3Config>(&scu)?;
    let spim3 = test_common::initialize_monitor_with_policy::<Spim3Config>(
        log_buffer(&SPIM3_LOG),
        &policy,
    )?;
    test_common::dump_policy(&spim3, WRITE_PROTECTED_LENGTH)?;
    test_common::validate_one_mib_write_protection(&spim3)?;
    test_common::validate_filtering(&spim3)?;
    let spim3 = test_common::lock_monitor(spim3)?;

    pw_log::info!("=== SPIM4 ===");
    test_common::configure_wiring::<Spim4Config>(&scu)?;
    let spim4 = test_common::initialize_monitor_with_policy::<Spim4Config>(
        log_buffer(&SPIM4_LOG),
        &policy,
    )?;
    test_common::dump_policy(&spim4, WRITE_PROTECTED_LENGTH)?;
    test_common::validate_one_mib_write_protection(&spim4)?;
    test_common::validate_filtering(&spim4)?;
    let spim4 = test_common::lock_monitor(spim4)?;

    if scu.route_control_raw() & 0x0fff_0000 != 0 {
        pw_log::info!(
            "FAIL: unexpected SCU flash-reset routing: 0x{:08x}",
            scu.route_control_raw() as u32
        );
        return Err(test_common::TestError::Check);
    }
    pw_log::info!("All SPIM1-4 monitors are filtering traffic");
    pw_log::info!("SPIM3/4 policies are locked; SPIM1/2 match unlocked Zephyr runtime");
    pw_log::info!(
        "SCU0F0 after SPIM setup: 0x{:08x}",
        scu.route_control_raw() as u32
    );

    pw_log::info!("=== Reset BMC flashes through internal SPI1 ===");
    if reset_bmc_flashes(&scu, true).is_err() {
        pw_log::info!("FAIL: BMC flash reset/readback through SPI1");
        return Err(test_common::TestError::Check);
    }
    pw_log::info!("PASS: spi1@0 and spi1@1 are in a known SPI state");

    pw_log::info!("=== Hand BMC flash mux to external BMC master ===");
    spim1.reset_filter_state();
    spim2.reset_filter_state();
    test_common::validate_filtering(&spim1)?;
    test_common::validate_filtering(&spim2)?;
    pw_log::info!(
        "SPIM1/2 pre-release ctrl=0x{:08x}/0x{:08x}",
        spim1.regs().read_ctrl() as u32,
        spim2.regs().read_ctrl() as u32
    );

    apply_spim_external_mux(Spim1Config::INSTANCE, ScuExtMuxSelect::Mux0);
    if spim_external_mux_state(Spim1Config::INSTANCE) != Some(ScuExtMuxSelect::Mux0) {
        pw_log::info!("FAIL: SPIM1/2 external BMC mux handoff");
        return Err(test_common::TestError::Check);
    }
    pw_log::info!("PASS: SPIM1/2 mux switched from RoT=1 to BMC=0");

    pw_log::info!("Waiting 60 ms for flash routing to settle");
    delay_us(60_000);

    pw_log::info!("=== Release BMC reset ===");
    if !set_bmc_resets(false) {
        pw_log::info!("FAIL: SGPIOM BMC reset outputs did not release");
        return Err(test_common::TestError::Check);
    }
    pw_log::info!("PASS: SGPIOM outputs 8 and 9 released high");

    pw_log::info!("Firmware will remain active until the user stops or resets it");
    monitor_spim_violations(&spim1, &spim2, &spim3, &spim4)
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
