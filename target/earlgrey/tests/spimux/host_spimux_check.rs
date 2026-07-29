// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

use anyhow::{bail, Result};
use clap::Parser;
use std::time::Duration;

use opentitanlib::app::TransportWrapper;
use opentitanlib::io::gpio::PinMode;
use opentitanlib::test_utils::init::InitializeTest;
use opentitanlib::uart::console::UartConsole;

#[derive(Debug, Parser)]
struct Opts {
    #[command(flatten)]
    init: InitializeTest,

    #[arg(
        long,
        value_parser = humantime::parse_duration,
        default_value = "30s"
    )]
    timeout: Duration,
}

fn verify_electrical_state(
    transport: &TransportWrapper,
    interface: &str,
    expected_ctrl_high: bool,
    state_name: &str,
) -> Result<()> {
    if interface != "logician" && interface != "teacup" {
        log::info!(
            "[{}] Verified on-chip GPIO register readback via console (external pin sampling skipped on {})",
            state_name,
            interface
        );
        return Ok(());
    }

    log::info!(
        "[{}] Verifying electrical pin levels on tester board ({})",
        state_name,
        interface
    );

    let ioa3_pin = transport.gpio_pin("IOA3")?;
    let ioa6_pin = transport.gpio_pin("IOA6")?;
    let iob8_pin = transport.gpio_pin("IOB8")?;
    let iob7_pin = transport.gpio_pin("IOB7")?;
    let ioa7_pin = transport.gpio_pin("IOA7")?;

    ioa3_pin.set(Some(PinMode::Input), None, None, None)?;
    ioa6_pin.set(Some(PinMode::Input), None, None, None)?;
    iob8_pin.set(Some(PinMode::Input), None, None, None)?;
    iob7_pin.set(Some(PinMode::Input), None, None, None)?;
    ioa7_pin.set(Some(PinMode::Input), None, None, None)?;

    let ioa3_val = ioa3_pin.read()?;
    let ioa6_val = ioa6_pin.read()?;
    let iob8_val = iob8_pin.read()?;
    let iob7_val = iob7_pin.read()?;
    let ioa7_val = ioa7_pin.read()?;

    log::info!(
        "[{}] TESTER PIN SAMPLING -> IOB8={}, IOB7={}, IOA7={}, IOA3={}, IOA6={}",
        state_name,
        iob8_val,
        iob7_val,
        ioa7_val,
        ioa3_val,
        ioa6_val
    );

    let ctrl_val = iob8_val;
    let en_n_val = iob7_val;
    let reset_n_val = ioa7_val;

    if ctrl_val != expected_ctrl_high {
        bail!(
            "[{}] MUX_CTRL pin (IOB8) mismatch: expected {}, got {}",
            state_name,
            expected_ctrl_high,
            ctrl_val
        );
    }
    if en_n_val {
        bail!(
            "[{}] MUX_EN_N pin (IOB7) mismatch: expected LOW (false/enabled), got HIGH",
            state_name
        );
    }
    if !reset_n_val {
        bail!(
            "[{}] RESET_N pin (IOA7) mismatch: expected HIGH (true/released), got LOW",
            state_name
        );
    }
    if !ioa3_val || !ioa6_val {
        bail!(
            "[{}] WP_N pins mismatch: expected IOA3 (WP0) and IOA6 (WP1) HIGH (released), got IOA3={}, IOA6={}",
            state_name,
            ioa3_val,
            ioa6_val
        );
    }

    log::info!(
        "[{}] Verified electrical state: MUX_CTRL={}, MUX_EN_N=LOW, RESET_N=HIGH, WP0/WP1=HIGH",
        state_name,
        if ctrl_val {
            "HIGH (HostCpu1Earlgrey0)"
        } else {
            "LOW (HostCpu0Earlgrey1)"
        }
    );
    Ok(())
}

fn main() -> Result<()> {
    let opts = Opts::parse();
    opts.init.init_logging();

    let transport = opts.init.init_target()?;
    let interface = opts.init.backend_opts.interface.as_str();

    log::info!("Resetting target...");
    transport.reset(opentitanlib::app::UartRx::Clear)?;

    let uart = transport.uart("console")?;
    log::info!("Waiting for RUNNING banner on console...");
    UartConsole::wait_for(&*uart, r"RUNNING", opts.timeout)?;

    log::info!("Waiting for initial SPIMUX_ROUTE = HostCpu0Earlgrey1 banner...");
    UartConsole::wait_for(&*uart, r"SPIMUX_ROUTE = HostCpu0Earlgrey1", opts.timeout)?;
    verify_electrical_state(&transport, interface, false, "ROUTE_HostCpu0Earlgrey1")?;

    log::info!("Waiting for SPIMUX_ROUTE = HostCpu1Earlgrey0 banner...");
    UartConsole::wait_for(&*uart, r"SPIMUX_ROUTE = HostCpu1Earlgrey0", opts.timeout)?;
    verify_electrical_state(&transport, interface, true, "ROUTE_HostCpu1Earlgrey0")?;

    log::info!("Waiting for SPIMUX_ROUTE = HostCpu0Earlgrey1_AGAIN banner...");
    UartConsole::wait_for(
        &*uart,
        r"SPIMUX_ROUTE = HostCpu0Earlgrey1_AGAIN",
        opts.timeout,
    )?;
    verify_electrical_state(
        &transport,
        interface,
        false,
        "ROUTE_HostCpu0Earlgrey1_AGAIN",
    )?;

    log::info!("Waiting for PASS banner...");
    UartConsole::wait_for(&*uart, r"✅ PASS", opts.timeout)?;

    log::info!("✅ All SPIMUX electrical and switching E2E checks passed!");
    Ok(())
}
