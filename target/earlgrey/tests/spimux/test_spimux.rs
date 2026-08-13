// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#![no_std]
#![no_main]

use earlgrey_gpio::EarlGreyGpio;
use earlgrey_pinout::dualsbs::DualSideBySide;
use earlgrey_pinout::Pinout;
use earlgrey_platform::spimux::{SpiMuxEvent, SpiMuxHandler, SpiMuxRoute};
use openprot_hal_blocking::gpio_port::PinMask;
use pw_status::Result;
use userspace::entry;
use userspace::time::{sleep_until, Clock, Duration, SystemClock};
use util_panic as _;

const DELAY_10MS: Duration = Duration::from_millis(10);

fn sleep_10ms() {
    let _ = sleep_until(SystemClock::now() + DELAY_10MS);
}

fn verify_mux_state(gpio: &EarlGreyGpio, expect_ctrl_high: bool) -> Result<()> {
    let out = gpio.read_output().map_err(|_| pw_status::Error::Internal)?;
    let ctrl_mask = earlgrey_gpio::GpioMask::from(DualSideBySide::SPI_MUX_CTRL);
    let en_mask = earlgrey_gpio::GpioMask::from(DualSideBySide::SPI_MUX_EN_N);
    let reset_mask = earlgrey_gpio::GpioMask::from(DualSideBySide::SPI_RESET_N);

    let is_ctrl_high = out.contains(ctrl_mask);
    let is_en_high = out.contains(en_mask);
    let is_reset_high = out.contains(reset_mask);

    if is_ctrl_high != expect_ctrl_high {
        pw_log::error!("SPI_MUX_CTRL mismatch: expected high={}", expect_ctrl_high);
        return Err(pw_status::Error::Internal);
    }
    if is_en_high {
        pw_log::error!("SPI_MUX_EN_N should be LOW (enabled)");
        return Err(pw_status::Error::Internal);
    }
    if !is_reset_high {
        pw_log::error!("SPI_RESET_N should be HIGH (released)");
        return Err(pw_status::Error::Internal);
    }
    Ok(())
}

fn run_spimux_test() -> Result<()> {
    // SAFETY: EarlGreyGpio::new() initializes MMIO access to the GPIO and Pinmux peripherals;
    // safe in this single-threaded test environment.
    let mut gpio = unsafe { EarlGreyGpio::new() };
    DualSideBySide::configure(&mut gpio).map_err(|_| pw_status::Error::Internal)?;

    let mut spimux = SpiMuxHandler::new(
        DualSideBySide::SPI_MUX_EN_N,
        DualSideBySide::SPI_MUX_CTRL,
        DualSideBySide::SPI_RESET_N,
        DualSideBySide::SPI_HOST0_WP_N,
        DualSideBySide::SPI_HOST1_WP_N,
    );

    pw_log::info!("🔄 RUNNING SPIMUX SWITCHING TEST");

    // 1. Initialize to default ColdBoot state (HostCpu0Earlgrey1: Host CPU -> Flash 0, Earlgrey -> Flash 1)
    spimux
        .handle_event(SpiMuxEvent::ColdBoot, &mut gpio)
        .map_err(|_| pw_status::Error::Internal)?;
    verify_mux_state(&gpio, false)?;
    pw_log::info!("SPIMUX_ROUTE = HostCpu0Earlgrey1");

    sleep_10ms();

    // 2. Switch Mux to HostCpu1Earlgrey0 (Host CPU -> Flash 1, Earlgrey -> Flash 0)
    spimux
        .handle_event(
            SpiMuxEvent::Route(SpiMuxRoute::HostCpu1Earlgrey0),
            &mut gpio,
        )
        .map_err(|_| pw_status::Error::Internal)?;
    verify_mux_state(&gpio, true)?;
    pw_log::info!("SPIMUX_ROUTE = HostCpu1Earlgrey0");

    sleep_10ms();

    // 3. Switch Mux back to HostCpu0Earlgrey1 (Host CPU -> Flash 0, Earlgrey -> Flash 1)
    spimux
        .handle_event(
            SpiMuxEvent::Route(SpiMuxRoute::HostCpu0Earlgrey1),
            &mut gpio,
        )
        .map_err(|_| pw_status::Error::Internal)?;
    verify_mux_state(&gpio, false)?;
    pw_log::info!("SPIMUX_ROUTE = HostCpu0Earlgrey1_AGAIN");

    pw_log::info!("✅ PASS");

    Ok(())
}

#[entry]
fn entry() -> Result<()> {
    run_spimux_test()
}
