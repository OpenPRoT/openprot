// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

use anyhow::{bail, Result};
use clap::Parser;
use std::time::Duration;

use opentitanlib::app::TransportWrapper;
use opentitanlib::io::gpio::{PinMode, PullMode};
use opentitanlib::test_utils::init::InitializeTest;
use opentitanlib::uart::console::UartConsole;

#[derive(Debug, Parser)]
struct Opts {
    #[command(flatten)]
    init: InitializeTest,

    /// Console receive timeout.
    #[arg(long, value_parser = humantime::parse_duration, default_value = "180s")]
    timeout: Duration,
}

fn sw_strap_set_teacup(transport: &TransportWrapper, value: u8) -> Result<()> {
    // The teacup board has HyperDebug GPIOs dedicated to driving the weak values.
    let pin_pairs = [
        (
            transport.gpio_pin("SW_STRAP0")?,
            transport.gpio_pin("SW_STRAP0_WEAK")?,
        ),
        (
            transport.gpio_pin("SW_STRAP1")?,
            transport.gpio_pin("SW_STRAP1_WEAK")?,
        ),
        (
            transport.gpio_pin("SW_STRAP2")?,
            transport.gpio_pin("SW_STRAP2_WEAK")?,
        ),
    ];
    for (i, (strong, weak)) in pin_pairs.iter().enumerate() {
        let shift = 2usize.checked_mul(i).unwrap_or(0);
        let pinval = ((value >> shift) & 3) as usize;
        if pinval == 0 || pinval == 3 {
            strong.set(Some(PinMode::PushPull), Some(pinval == 3), None, None)?;
            weak.set(Some(PinMode::Input), None, None, None)?;
        } else {
            weak.set(Some(PinMode::PushPull), Some(pinval == 2), None, None)?;
            strong.set(Some(PinMode::Input), None, None, None)?;
        }
    }
    Ok(())
}

fn sw_strap_set_verilator(transport: &TransportWrapper, value: u8) -> Result<()> {
    let dont_care = false;
    let settings = [
        (PinMode::PushPull, false, PullMode::None),
        (PinMode::Input, dont_care, PullMode::PullDown),
        (PinMode::Input, dont_care, PullMode::PullUp),
        (PinMode::PushPull, true, PullMode::None),
    ];
    let pins = [
        transport.gpio_pin("IOC0")?,
        transport.gpio_pin("IOC1")?,
        transport.gpio_pin("IOC2")?,
    ];
    for (i, pin) in pins.iter().enumerate() {
        let shift = 2usize.checked_mul(i).unwrap_or(0);
        let pinval = ((value >> shift) & 3) as usize;
        let Some(&(mode, val, pull)) = settings.get(pinval) else {
            bail!("Invalid pinval index: {}", pinval);
        };
        pin.set(Some(mode), Some(val), Some(pull), None)?;
    }
    Ok(())
}

fn set_strap(transport: &TransportWrapper, interface: &str, value: u8) -> Result<()> {
    match interface {
        "teacup" => sw_strap_set_teacup(transport, value),
        "verilator" | "hyper310" | "hyper340" => sw_strap_set_verilator(transport, value),
        intf => bail!("Unsupported interface for SWStraps test: {}", intf),
    }
}

fn is_strong_only(value: u8) -> bool {
    for i in 0..3 {
        let shift = 2usize.checked_mul(i).unwrap_or(0);
        let v = (value >> shift) & 3;
        if v == 1 || v == 2 {
            return false;
        }
    }
    true
}

fn strap_pattern(value: u8) -> String {
    let bits = [
        b's', // Strong zero.
        b'w', // Weak zero.
        b'W', // Weak one.
        b'S', // Strong one.
    ];
    let mut buf = [b'X'; 3];
    for i in 0..3 {
        let shift = 2usize.checked_mul(i).unwrap_or(0);
        let v = ((value >> shift) & 3) as usize;
        if let (Some(idx), Some(&ch)) = (2usize.checked_sub(i), bits.get(v)) {
            if let Some(slot) = buf.get_mut(idx) {
                *slot = ch;
            }
        }
    }
    std::str::from_utf8(&buf).unwrap_or("???").into()
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

    log::info!("Waiting for initial strap value 0x00 after boot...");
    UartConsole::wait_for(&*uart, r"SW_STRAP = 0x00", opts.timeout)?;
    log::info!("Verified initial strap value 0x00");

    // Test remaining strap patterns 1..64, then test 0x00 again at the end.
    let test_values = (1..64).chain(std::iter::once(0));

    for value in test_values {
        if (interface == "hyper310" || interface == "hyper340") && !is_strong_only(value) {
            log::info!(
                "Skipping weak strapping value {:#04x} (pattern: {}) on FPGA",
                value,
                strap_pattern(value)
            );
            continue;
        }

        log::info!(
            "Testing strap value {:#04x} (pattern: {})",
            value,
            strap_pattern(value)
        );
        set_strap(&transport, interface, value)?;

        let expected_pattern = format!(r"SW_STRAP = {:#04x}", value);
        log::info!("Waiting for console output: '{}'...", expected_pattern);
        UartConsole::wait_for(&*uart, &expected_pattern, opts.timeout)?;
        log::info!("Verified strap value {:#04x}", value);
    }

    // Reset straps to all zeros at the end of the test.
    set_strap(&transport, interface, 0)?;

    log::info!("✅ All SWStraps tests passed!");
    Ok(())
}
