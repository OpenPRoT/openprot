// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Shared-flash paired test, reader half: the mock BMC polls the staging flash
//! on SPI1 CS0 until it sees the pattern the RoT wrote.
//!
//! A plain SPI1 master: it drives the RoT's SPIM host port from the outside, so
//! it routes nothing through its own SPIM and leaves the external mux alone —
//! the RoT owns it, and two chips driving one select line would fight.

#![no_std]
#![no_main]

use ast10x0_board::{apply_spim_module_mux, delay_us};
use ast10x0_peripherals::aperture::take_aperture;
use ast10x0_peripherals::scu::{
    pinctrl::{PINCTRL_GPIOB4, PINCTRL_SPI1_QUAD},
    ScuExtMuxSelect, ScuRegisters, SpiMonitorInstance, SpiMonitorSource,
};
use ast10x0_peripherals::smc::{SmcError, SpiNorFlash, SpiNorFlashDevice, SpiUninit};
use target_common::{declare_target, TargetInterface};
use {console_backend as _, entry as _};

mod shared;
use shared::{finish, smc_error_str, Spi1Instance, MAGIC_OFFSET, PATTERN, PATTERN_LEN};

/// The RoT owns the external mux, so the reader polls instead of waiting on a
/// handshake: `set_bmc_resets` does not reach the second board here.
const POLL_INTERVAL_US: u32 = 250_000;
const POLL_ATTEMPTS: u32 = 120;

fn run_test() -> Result<(), SmcError> {
    // SAFETY: kernel main() runs once with exclusive hardware ownership.
    let scu = unsafe { ScuRegisters::new_global_unlocked() };
    scu.apply_pinctrl_group(PINCTRL_SPI1_QUAD);
    // Zero: SPI1 goes straight out this chip's pins instead of detouring
    // through its own SPIM.
    scu.set_spim_internal_mux(SpiMonitorSource::Spi1, 0)
        .map_err(|_| SmcError::HardwareError)?;

    // Clock and data leave on this chip's own SPI1 pins, but chip select still
    // runs through the local mux, so point that at SPI1 or the flash never sees
    // CS assert.
    scu.apply_pinctrl_group(PINCTRL_GPIOB4);
    apply_spim_module_mux(SpiMonitorInstance::Spim0, ScuExtMuxSelect::Mux1);

    let mut spi = unsafe {
        SpiUninit::<Spi1Instance>::new(take_aperture(), take_aperture(), take_aperture())?
    }
    .init()?;

    match SpiNorFlash::new(spi.cs0()?).and_then(|f| f.jedec_id()) {
        Ok(jedec) => pw_log::info!(
            "BMC: SPI1 CS0 JEDEC ID {:02x} {:02x} {:02x}",
            jedec[0] as u32,
            jedec[1] as u32,
            jedec[2] as u32
        ),
        Err(e) => pw_log::info!("BMC: JEDEC read failed: {}", smc_error_str(e) as &str),
    }

    // Errors and mismatches both just retry: while the RoT still holds the mux,
    // these reads are expected to fail or return garbage.
    let mut buf = [0u8; PATTERN_LEN];
    for _ in 0..POLL_ATTEMPTS {
        if let Ok(flash) = SpiNorFlash::new(spi.cs0()?) {
            if flash.read(MAGIC_OFFSET, &mut buf) == Ok(PATTERN_LEN) && buf == PATTERN {
                pw_log::info!("BMC: read the pattern the RoT wrote");
                return Ok(());
            }
        }
        delay_us(POLL_INTERVAL_US);
    }

    pw_log::error!("BMC: timed out waiting for the pattern");
    Err(SmcError::Timeout)
}

pub struct Target {}

impl TargetInterface for Target {
    const NAME: &'static str = "AST10x0 shared flash (mock BMC)";

    fn main() -> ! {
        finish(run_test())
    }
}

declare_target!(Target);
