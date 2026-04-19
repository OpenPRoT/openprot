// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#![no_std]
#![no_main]

use earlgrey_gpio::{EarlGreyGpio, EarlGreyPinConfig, GpioMask, GpioPin};
use earlgrey_pinmux::{Pad, Pull};
use openprot_hal_blocking::gpio_port::{GpioPort, PinMask};
use pw_status::Result;
use userspace::{entry, syscall};

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

#[entry]
fn entry() -> ! {
    pw_log::info!("🔄 RUNNING GPIO SMOKE TEST");
    let ret = test_gpio_basic();

    if ret.is_err() {
        pw_log::error!("❌ FAIL");
    } else {
        pw_log::info!("✅ PASS");
    }

    let _ = syscall::debug_shutdown(ret);
    loop {}
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    pw_log::error!("FAIL: panic in gpio test");
    loop {}
}
