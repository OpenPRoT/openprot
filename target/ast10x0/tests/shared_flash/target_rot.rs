// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Shared-flash paired test, writer half: the RoT writes a pattern into the
//! staging flash on SPI1 CS0, then hands the flash to the mock BMC.
//!
//! Kernel-only, so a failure can only be about flash and routing.

#![no_std]
#![no_main]

use ast10x0_board::{
    apply_spim_external_mux, apply_spim_pinctrl, enable_flash_power, release_spi_flash_resets,
};
use ast10x0_peripherals::aperture::take_aperture;
use ast10x0_peripherals::scu::{
    pinctrl::PINCTRL_SPI1_QUAD, ScuExtMuxSelect, ScuRegisters, SpiMonitorInstance,
    SpiMonitorPassthrough, SpiMonitorSource,
};
use ast10x0_peripherals::smc::{SmcError, SpiNorFlash, SpiNorFlashDevice, SpiUninit};
use target_common::{declare_target, TargetInterface};
use {console_backend as _, entry as _};

mod shared;
use shared::{finish, Spi1Instance, MAGIC_OFFSET, PATTERN, PATTERN_LEN};

const SPIM: SpiMonitorInstance = SpiMonitorInstance::Spim0;

/// A JEDEC ID of all-00 means the bus is held low, all-ff means it is floating.
/// A byte count from `read()` proves nothing: it is controller-local and passes
/// on a dead bus.
fn jedec_is_present(id: [u8; 3]) -> bool {
    id != [0x00, 0x00, 0x00] && id != [0xff, 0xff, 0xff]
}

fn run_test() -> Result<(), SmcError> {
    // SAFETY: kernel main() runs once with exclusive hardware ownership.
    let scu = unsafe { ScuRegisters::new_global_unlocked() };

    if !enable_flash_power(&scu) || !release_spi_flash_resets() {
        return Err(SmcError::HardwareError);
    }

    apply_spim_pinctrl(&scu, SPIM);
    scu.apply_pinctrl_group(PINCTRL_SPI1_QUAD);
    scu.disable_spim_cs_internal_pull_down(SPIM);
    scu.set_spim_passthrough(SPIM, SpiMonitorPassthrough::Enabled);
    apply_spim_external_mux(SPIM, ScuExtMuxSelect::Mux1);
    scu.set_spim_internal_mux(SpiMonitorSource::Spi1, SPIM as u8 + 1)
        .map_err(|_| SmcError::HardwareError)?;

    let mut spi = unsafe {
        SpiUninit::<Spi1Instance>::new(take_aperture(), take_aperture(), take_aperture())?
    }
    .init()?;

    let jedec = SpiNorFlash::new(spi.cs0()?)?.jedec_id()?;
    pw_log::info!(
        "RoT: SPI1 CS0 JEDEC ID {:02x} {:02x} {:02x}",
        jedec[0] as u32,
        jedec[1] as u32,
        jedec[2] as u32
    );
    if !jedec_is_present(jedec) {
        return Err(SmcError::DeviceNotSupported);
    }

    {
        let mut flash = SpiNorFlash::new(spi.cs0()?)?;
        flash.erase_sector(MAGIC_OFFSET)?;
        flash.program_page(MAGIC_OFFSET, &PATTERN)?;
    }

    let mut readback = [0u8; PATTERN_LEN];
    let n = SpiNorFlash::new(spi.cs0()?)?.read(MAGIC_OFFSET, &mut readback)?;
    if n != PATTERN_LEN || readback != PATTERN {
        pw_log::error!("RoT: local readback mismatch");
        return Err(SmcError::HardwareError);
    }
    pw_log::info!("RoT: wrote and verified the pattern; handing the flash over");

    // Hand the flash to the mock BMC: release SPIM0's input from our own SPI1
    // master so the external host drives it, then flip the mux. Passthrough
    // stays on, since that is the path the BMC's reads take.
    scu.clear_spim_internal_master_route();
    apply_spim_external_mux(SPIM, ScuExtMuxSelect::Mux0);
    Ok(())
}

pub struct Target {}

impl TargetInterface for Target {
    const NAME: &'static str = "AST10x0 shared flash (RoT)";

    fn main() -> ! {
        finish(run_test())
    }
}

declare_target!(Target);
