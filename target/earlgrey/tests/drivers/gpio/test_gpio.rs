// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#![no_std]
#![no_main]

use earlgrey_gpio::{EarlGreyGpio, EarlGreyPinConfig, GpioMask, GpioPin};
use earlgrey_pinmux::{Pad, Pull};
use openprot_hal_blocking::gpio_port::{GpioPort, PinMask};
use pw_status::Result;
use userspace::entry;
use util_panic as _;

fn test_gpio_basic() -> Result<()> {
    let mut gpio = unsafe { EarlGreyGpio::new() };

    // Test 1: Configure Pin 0 as output
    pw_log::info!("Configuring Pin 0 as output");
    gpio.configure(
        GpioPin::Pin0.into(),
        EarlGreyPinConfig {
            is_input: false,
            is_output: true,
            input_filter: false,
            pad: None, // Don't care about physical pad in Verilator for this basic test
            pull: Pull::None,
        },
    )
    .map_err(|_| pw_status::Error::Internal)?;

    // Test 2: Set Pin 0 high
    pw_log::info!("Setting Pin 0 high");
    gpio.set_reset(GpioPin::Pin0.into(), GpioMask::empty())
        .map_err(|_| pw_status::Error::Internal)?;

    // In Verilator, data_in usually reflects data_out if OE is set (loopback behavior depends on testbench)
    // For now, let's just check if we can read back our own output state using our target-specific method
    let output = gpio.read_output().map_err(|_| pw_status::Error::Internal)?;
    if !output.contains(GpioPin::Pin0.into()) {
        pw_log::error!("Pin 0 output readback failed (expected High)");
        return Err(pw_status::Error::Internal.into());
    }

    // Test 3: Toggle Pin 0
    pw_log::info!("Toggling Pin 0");
    gpio.toggle(GpioPin::Pin0.into())
        .map_err(|_| pw_status::Error::Internal)?;

    let output = gpio.read_output().map_err(|_| pw_status::Error::Internal)?;
    if output.contains(GpioPin::Pin0.into()) {
        pw_log::error!("Pin 0 toggle failed (expected Low)");
        return Err(pw_status::Error::Internal.into());
    }

    // Test 4: Configure a Dedicated I/O (DIO) pin
    // Note: Toggling DIOs via the GPIO block isn't possible in standard EarlGrey,
    // but we can verify the configuration logic (attributes/pull-ups) works.
    pw_log::info!("Configuring DIO 0 (Dedicated IO) with Pull-up");
    gpio.configure(
        GpioPin::Pin0.into(),
        EarlGreyPinConfig {
            is_input: true,
            is_output: false,
            input_filter: false,
            pad: Some(Pad::DIO0),
            pull: Pull::Up,
        },
    )
    .map_err(|_| pw_status::Error::Internal)?;

    Ok(())
}

// TODO: remove after fixing QEMU to handle this test properly.
#[allow(dead_code)]
fn test_gpio_constants() -> Result<()> {
    let mut gpio = unsafe { EarlGreyGpio::new() };

    // Configure Pin 1 as input, connected to ConstantZero
    pw_log::info!("Configuring Pin 1 as input connected to ConstantZero");
    gpio.configure(
        GpioPin::Pin1.into(),
        EarlGreyPinConfig {
            is_input: true,
            is_output: false,
            input_filter: false,
            pad: Some(Pad::ConstantZero),
            pull: Pull::None,
        },
    )
    .map_err(|_| pw_status::Error::Internal)?;

    // Read Pin 1, should be low (0)
    let input = gpio.read_input().map_err(|_| pw_status::Error::Internal)?;
    if input.contains(GpioPin::Pin1.into()) {
        pw_log::error!("Pin 1 (ConstantZero) readback failed (expected Low)");
        return Err(pw_status::Error::Internal.into());
    }

    // Configure Pin 1 as input, connected to ConstantOne
    pw_log::info!("Configuring Pin 1 as input connected to ConstantOne");
    gpio.configure(
        GpioPin::Pin1.into(),
        EarlGreyPinConfig {
            is_input: true,
            is_output: false,
            input_filter: false,
            pad: Some(Pad::ConstantOne),
            pull: Pull::None,
        },
    )
    .map_err(|_| pw_status::Error::Internal)?;

    // Read Pin 1, should be high (1)
    let input = gpio.read_input().map_err(|_| pw_status::Error::Internal)?;
    if !input.contains(GpioPin::Pin1.into()) {
        pw_log::error!("Pin 1 (ConstantOne) readback failed (expected High)");
        return Err(pw_status::Error::Internal.into());
    }

    Ok(())
}

// TODO: remove after determining if this is a valid test.
#[allow(dead_code)]
fn test_gpio_mio_loopback() -> Result<()> {
    let mut gpio = unsafe { EarlGreyGpio::new() };

    // Configure Pin 2 as output, connected to Pad::IOA0
    pw_log::info!("Configuring Pin 2 as output connected to IOA0");
    gpio.configure(
        GpioPin::Pin2.into(),
        EarlGreyPinConfig {
            is_input: false,
            is_output: true,
            input_filter: false,
            pad: Some(Pad::IOA0),
            pull: Pull::None,
        },
    )
    .map_err(|_| pw_status::Error::Internal)?;

    // Configure Pin 3 as input, connected to Pad::IOA0
    pw_log::info!("Configuring Pin 3 as input connected to Pad::IOA0");
    gpio.configure(
        GpioPin::Pin3.into(),
        EarlGreyPinConfig {
            is_input: true,
            is_output: false,
            input_filter: false,
            pad: Some(Pad::IOA0),
            pull: Pull::None,
        },
    )
    .map_err(|_| pw_status::Error::Internal)?;

    // Set Pin 2 high
    pw_log::info!("Setting Pin 2 high");
    gpio.set_reset(GpioPin::Pin2.into(), GpioMask::empty())
        .map_err(|_| pw_status::Error::Internal)?;

    // Read Pin 3, should be high
    let input = gpio.read_input().map_err(|_| pw_status::Error::Internal)?;
    if !input.contains(GpioPin::Pin3.into()) {
        pw_log::error!("MIO Loopback failed (expected High on Pin 3)");
        return Err(pw_status::Error::Internal.into());
    }

    // Set Pin 2 low
    pw_log::info!("Setting Pin 2 low");
    gpio.set_reset(GpioMask::empty(), GpioPin::Pin2.into())
        .map_err(|_| pw_status::Error::Internal)?;

    // Read Pin 3, should be low
    let input = gpio.read_input().map_err(|_| pw_status::Error::Internal)?;
    if input.contains(GpioPin::Pin3.into()) {
        pw_log::error!("MIO Loopback failed (expected Low on Pin 3)");
        return Err(pw_status::Error::Internal.into());
    }

    Ok(())
}

#[entry]
fn entry() -> Result<()> {
    pw_log::info!("🔄 RUNNING GPIO SMOKE TEST");
    let ret = test_gpio_basic();

    if ret.is_ok() {
        // TODO: it seems that qemu doesn't implement ConstantOne for GPIOs correctly.
        // ret = test_gpio_constants();
    }
    if ret.is_ok() {
        // TODO: it is unclear whether you can really configure the pinmux for loopback.
        // This test fails on both qemu and fpga.
        // ret = test_gpio_mio_loopback();
    }

    match ret {
        Ok(()) => pw_log::info!("✅ PASS"),
        Err(e) => pw_log::error!("❌ FAIL: {}", e as u32),
    }
    ret
}
