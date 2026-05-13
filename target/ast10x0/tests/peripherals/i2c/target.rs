// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#![no_std]
#![no_main]

use ast10x0_board::{Ast10x0Board, Ast10x0BoardDescriptor};
use ast10x0_peripherals::i2c::{
    Ast1060I2c, ClockConfig, I2cConfig, I2cError, I2cSpeed, I2cXferMode,
};
use ast10x0_peripherals::scu::pinctrl;
use codegen as _;
use console_backend as _;
use cortex_m_semihosting::debug::{EXIT_FAILURE, EXIT_SUCCESS, exit};
use entry as _;
use target_common::{TargetInterface, declare_target};

pub struct Target {}

const ERR_INIT_FAILED: &str = "i2c controller init failed";
const ERR_VERIFY_FAILED: &str = "i2c init register verification failed";

struct ExpectedTiming {
    base_clk_div: u8,
    scl_low: u8,
    scl_high: u8,
    scl_high_min: u8,
}

fn i2c_error_str(error: I2cError) -> &'static str {
    match error {
        I2cError::Overrun => "Overrun",
        I2cError::NoAcknowledge => "NoAcknowledge",
        I2cError::Timeout => "Timeout",
        I2cError::BusRecoveryFailed => "BusRecoveryFailed",
        I2cError::Bus => "Bus",
        I2cError::Busy => "Busy",
        I2cError::Invalid => "Invalid",
        I2cError::Abnormal => "Abnormal",
        I2cError::ArbitrationLoss => "ArbitrationLoss",
        I2cError::SlaveError => "SlaveError",
        I2cError::InvalidAddress => "InvalidAddress",
    }
}

fn expected_timing(speed: I2cSpeed) -> ExpectedTiming {
    match speed {
        // ClockConfig::ast1060_default() => base_clk3=2.77MHz, divider ratio=28
        I2cSpeed::Standard => ExpectedTiming {
            base_clk_div: 3,
            scl_low: 14,
            scl_high: 12,
            scl_high_min: 11,
        },
        // base_clk2=10MHz, divider ratio=25
        I2cSpeed::Fast => ExpectedTiming {
            base_clk_div: 2,
            scl_low: 13,
            scl_high: 10,
            scl_high_min: 9,
        },
        // base_clk1=20MHz, divider ratio=20
        I2cSpeed::FastPlus => ExpectedTiming {
            base_clk_div: 1,
            scl_low: 10,
            scl_high: 8,
            scl_high_min: 7,
        },
    }
}

fn dump_i2c1_registers(name: &str, config: &I2cConfig) {
    // SAFETY: The test has exclusive ownership of I2C1 during execution.
    let regs = unsafe { &*ast1060_pac::I2c1::ptr() };
    let c00 = regs.i2cc00().read();
    let c04 = regs.i2cc04().read();
    let m10 = regs.i2cm10().read();
    let expected = expected_timing(config.speed);
    let expected_timeout_div: u32 = if config.smbus_timeout { 2 } else { 0 };
    let expected_timeout_timer: u32 = if config.smbus_timeout { 8 } else { 0 };

    pw_log::error!("--- {} mode I2C1 register dump ---", name as &str);
    pw_log::error!("i2cc00(raw)=0x{:08x}", c00.bits() as u32);
    pw_log::error!("i2cc04(raw)=0x{:08x}", c04.bits() as u32);
    pw_log::error!("i2cm10(raw)=0x{:08x}", m10.bits() as u32);

    pw_log::error!(
        "decoded: master_en={} auto_release={} dis_multimaster={} (expected={})",
        c00.enbl_master_fn().bit() as u8,
        c00.enbl_bus_autorelease_when_scllow_sdalow_or_slave_mode_inactive_timeout()
            .bit() as u8,
        c00.dis_multimaster_capability_for_master_fn_only().bit() as u8,
        (!config.multi_master) as u8
    );

    pw_log::error!(
        "decoded: base_clk_div={} (expected={}) scl_low={} (expected={})",
        c04.base_clk_divisor_tbase_clk().bits() as u32,
        expected.base_clk_div as u32,
        c04.cycles_of_master_sclclklow_pulse_width_tcklow().bits() as u32,
        expected.scl_low as u32
    );

    pw_log::error!(
        "decoded: scl_high={} (expected={}) scl_high_min={} (expected={})",
        c04.cycles_of_master_sclclkhigh_pulse_width_tckhigh().bits() as u32,
        expected.scl_high as u32,
        c04.cycles_of_master_sclclkhigh_minimum_pulse_width_tckhigh_min()
            .bits() as u32,
        expected.scl_high_min as u32
    );

    pw_log::error!(
        "decoded: timeout_div={} (expected={}) timeout_timer={} (expected={})",
        c04.timeout_base_clk_divisor_tout_base_clk().bits() as u32,
        expected_timeout_div as u32,
        c04.timeout_timer().bits() as u32,
        expected_timeout_timer as u32
    );

    pw_log::error!(
        "decoded: pkt_done_int={} bus_recover_int={} smbus_alert_int={} (expected={})",
        m10.enbl_pkt_cmd_done_int().bit() as u8,
        m10.enbl_bus_recover_done_int().bit() as u8,
        m10.enbl_smbus_dev_alert_int().bit() as u8,
        config.smbus_alert as u8
    );
}

fn verify_init_registers(name: &str, config: &I2cConfig) -> Result<(), &'static str> {
    // SAFETY: The test has exclusive ownership of I2C1 during execution.
    let regs = unsafe { &*ast1060_pac::I2c1::ptr() };

    let c00 = regs.i2cc00().read();
    let c04 = regs.i2cc04().read();
    let m10 = regs.i2cm10().read();
    let expected = expected_timing(config.speed);

    let mut ok = true;

    let master_en = c00.enbl_master_fn().bit();
    if !master_en {
        pw_log::error!("{} verify: enbl_master_fn not set", name as &str);
        ok = false;
    }

    let auto_release = c00
        .enbl_bus_autorelease_when_scllow_sdalow_or_slave_mode_inactive_timeout()
        .bit();
    if !auto_release {
        pw_log::error!("{} verify: bus auto-release not set", name as &str);
        ok = false;
    }

    let dis_multimaster = c00.dis_multimaster_capability_for_master_fn_only().bit();
    let expected_dis_multimaster = !config.multi_master;
    if dis_multimaster != expected_dis_multimaster {
        pw_log::error!(
            "{} verify: dis_multimaster mismatch actual={} expected={}",
            name as &str,
            dis_multimaster as u8,
            expected_dis_multimaster as u8
        );
        ok = false;
    }

    let base_clk_div = c04.base_clk_divisor_tbase_clk().bits();
    if base_clk_div != expected.base_clk_div {
        pw_log::error!(
            "{} verify: base_clk_div mismatch actual={} expected={}",
            name as &str,
            base_clk_div as u32,
            expected.base_clk_div as u32
        );
        ok = false;
    }

    let scl_low = c04.cycles_of_master_sclclklow_pulse_width_tcklow().bits();
    if scl_low != expected.scl_low {
        pw_log::error!(
            "{} verify: scl_low mismatch actual={} expected={}",
            name as &str,
            scl_low as u32,
            expected.scl_low as u32
        );
        ok = false;
    }

    let scl_high = c04.cycles_of_master_sclclkhigh_pulse_width_tckhigh().bits();
    if scl_high != expected.scl_high {
        pw_log::error!(
            "{} verify: scl_high mismatch actual={} expected={}",
            name as &str,
            scl_high as u32,
            expected.scl_high as u32
        );
        ok = false;
    }

    let scl_high_min = c04
        .cycles_of_master_sclclkhigh_minimum_pulse_width_tckhigh_min()
        .bits();
    if scl_high_min != expected.scl_high_min {
        pw_log::error!(
            "{} verify: scl_high_min mismatch actual={} expected={}",
            name as &str,
            scl_high_min as u32,
            expected.scl_high_min as u32
        );
        ok = false;
    }

    let timeout_div = c04.timeout_base_clk_divisor_tout_base_clk().bits();
    let expected_timeout_div = if config.smbus_timeout { 2 } else { 0 };
    if timeout_div != expected_timeout_div {
        pw_log::error!(
            "{} verify: timeout_div mismatch actual={} expected={}",
            name as &str,
            timeout_div as u32,
            expected_timeout_div as u32
        );
        ok = false;
    }

    let timeout_timer = c04.timeout_timer().bits();
    let expected_timeout_timer = if config.smbus_timeout { 8 } else { 0 };
    if timeout_timer != expected_timeout_timer {
        pw_log::error!(
            "{} verify: timeout_timer mismatch actual={} expected={}",
            name as &str,
            timeout_timer as u32,
            expected_timeout_timer as u32
        );
        ok = false;
    }

    let pkt_done_int = m10.enbl_pkt_cmd_done_int().bit();
    if !pkt_done_int {
        pw_log::error!("{} verify: pkt_cmd_done_int not enabled", name as &str);
        ok = false;
    }

    let bus_recover_int = m10.enbl_bus_recover_done_int().bit();
    if !bus_recover_int {
        pw_log::error!("{} verify: bus_recover_done_int not enabled", name as &str);
        ok = false;
    }

    let smbus_alert_int = m10.enbl_smbus_dev_alert_int().bit();
    if smbus_alert_int != config.smbus_alert {
        pw_log::error!(
            "{} verify: smbus_alert_int mismatch actual={} expected={}",
            name as &str,
            smbus_alert_int as u8,
            config.smbus_alert as u8
        );
        ok = false;
    }

    if ok {
        Ok(())
    } else {
        pw_log::error!("--- {} mode I2C1 register dump ---", name as &str);
        pw_log::error!("i2cc00(raw)=0x{:08x}", c00.bits() as u32);
        pw_log::error!("i2cc04(raw)=0x{:08x}", c04.bits() as u32);
        pw_log::error!("i2cm10(raw)=0x{:08x}", m10.bits() as u32);

        pw_log::error!(
            "decoded: master_en={} auto_release={} dis_multimaster={} (expected={})",
            c00.enbl_master_fn().bit() as u8,
            c00.enbl_bus_autorelease_when_scllow_sdalow_or_slave_mode_inactive_timeout()
                .bit() as u8,
            c00.dis_multimaster_capability_for_master_fn_only().bit() as u8,
            (!config.multi_master) as u8
        );

        pw_log::error!(
            "decoded: base_clk_div={} (expected={}) scl_low={} (expected={})",
            c04.base_clk_divisor_tbase_clk().bits() as u32,
            expected.base_clk_div as u32,
            c04.cycles_of_master_sclclklow_pulse_width_tcklow().bits() as u32,
            expected.scl_low as u32
        );

        pw_log::error!(
            "decoded: scl_high={} (expected={}) scl_high_min={} (expected={})",
            c04.cycles_of_master_sclclkhigh_pulse_width_tckhigh().bits() as u32,
            expected.scl_high as u32,
            c04.cycles_of_master_sclclkhigh_minimum_pulse_width_tckhigh_min()
                .bits() as u32,
            expected.scl_high_min as u32
        );

        pw_log::error!(
            "decoded: timeout_div={} (expected={}) timeout_timer={} (expected={})",
            c04.timeout_base_clk_divisor_tout_base_clk().bits() as u32,
            expected_timeout_div as u32,
            c04.timeout_timer().bits() as u32,
            expected_timeout_timer as u32
        );

        pw_log::error!(
            "decoded: pkt_done_int={} bus_recover_int={} smbus_alert_int={} (expected={})",
            m10.enbl_pkt_cmd_done_int().bit() as u8,
            m10.enbl_bus_recover_done_int().bit() as u8,
            m10.enbl_smbus_dev_alert_int().bit() as u8,
            config.smbus_alert as u8
        );
        Err(ERR_VERIFY_FAILED)
    }
}

fn run_i2c_init_smoke_test() -> Result<(), &'static str> {
    pw_log::info!("=== AST10x0 I2C init smoke test ===");

    let board = Ast10x0Board::new(Ast10x0BoardDescriptor {
        pinctrl_groups: &[pinctrl::PINCTRL_I2C1],
    });

    // SAFETY: Test target runs once at boot with exclusive access to the board.
    unsafe { board.init() };
    pw_log::info!("Board-level I2C global init complete");

    run_init_case(
        "standard",
        I2cConfig {
            speed: I2cSpeed::Standard,
            xfer_mode: I2cXferMode::BufferMode,
            multi_master: true,
            smbus_timeout: true,
            smbus_alert: false,
            clock_config: ClockConfig::ast1060_default(),
        },
    )?;
    run_init_case(
        "fast",
        I2cConfig {
            speed: I2cSpeed::Fast,
            xfer_mode: I2cXferMode::BufferMode,
            multi_master: true,
            smbus_timeout: true,
            smbus_alert: false,
            clock_config: ClockConfig::ast1060_default(),
        },
    )?;
    run_init_case(
        "fast-plus",
        I2cConfig {
            speed: I2cSpeed::FastPlus,
            xfer_mode: I2cXferMode::BufferMode,
            multi_master: false,
            smbus_timeout: true,
            smbus_alert: false,
            clock_config: ClockConfig::ast1060_default(),
        },
    )?;
    run_init_case_dma(
        "dma-fast",
        I2cConfig {
            speed: I2cSpeed::Fast,
            xfer_mode: I2cXferMode::DmaMode,
            multi_master: true,
            smbus_timeout: true,
            smbus_alert: false,
            clock_config: ClockConfig::ast1060_default(),
        },
    )?;

    pw_log::info!("=== AST10x0 I2C init smoke test complete ===");
    Ok(())
}

fn run_init_case(name: &str, config: I2cConfig) -> Result<(), &'static str> {
    pw_log::info!("Instantiating controller 1 in {} mode", name as &str);

    // SAFETY: The test owns the controller for the process lifetime and uses
    // the matching I2C/I2CBUFF register pair for controller 1.
    let result = unsafe {
        Ast1060I2c::new(
            ast1060_pac::I2c1::ptr(),
            ast1060_pac::I2cbuff1::ptr(),
            &config,
            |_| core::hint::spin_loop(),
        )
    };

    match result {
        Ok(_i2c) => {
            verify_init_registers(name, &config)?;
            pw_log::info!("{} mode init+verify passed", name as &str);
            Ok(())
        }
        Err(error) => {
            let error_name = i2c_error_str(error);
            pw_log::error!(
                "{} mode init failed: {}",
                name as &str,
                error_name as &str
            );
            dump_i2c1_registers(name, &config);
            Err(ERR_INIT_FAILED)
        }
    }
}

fn run_init_case_dma(name: &str, config: I2cConfig) -> Result<(), &'static str> {
    pw_log::info!(
        "Instantiating controller 1 in {} mode (new_with_dma)",
        name as &str
    );

    let mut dma_buf = [0u8; 64];

    // SAFETY: The test owns the controller for the process lifetime and uses
    // the matching I2C/I2CBUFF register pair for controller 1.
    let result = unsafe {
        Ast1060I2c::new_with_dma(
            ast1060_pac::I2c1::ptr(),
            ast1060_pac::I2cbuff1::ptr(),
            &config,
            &mut dma_buf,
            |_| core::hint::spin_loop(),
        )
    };

    match result {
        Ok(_i2c) => {
            verify_init_registers(name, &config)?;
            pw_log::info!("{} mode init+verify passed", name as &str);
            Ok(())
        }
        Err(error) => {
            let error_name = i2c_error_str(error);
            pw_log::error!(
                "{} mode init failed: {}",
                name as &str,
                error_name as &str
            );
            dump_i2c1_registers(name, &config);
            Err(ERR_INIT_FAILED)
        }
    }
}

impl TargetInterface for Target {
    const NAME: &'static str = "AST10x0 Kernel I2C";

    fn main() -> ! {
        let exit_status = match run_i2c_init_smoke_test() {
            Ok(()) => EXIT_SUCCESS,
            Err(error) => {
                pw_log::error!("I2C init smoke test failed: {}", error as &str);
                EXIT_FAILURE
            }
        };

        exit(exit_status);
        #[expect(clippy::empty_loop)]
        loop {}
    }

    fn shutdown(code: u32) -> ! {
        pw_log::info!("Shutting down with code {}", code as u32);
        #[expect(clippy::empty_loop)]
        loop {}
    }
}

declare_target!(Target);
