// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#![no_std]
#![no_main]

use ast10x0_board::{Ast10x0Board, Ast10x0BoardDescriptor};
use ast10x0_peripherals::aperture::take_aperture;
use ast10x0_peripherals::gpio::{GpioBlock, IntoGpio, OutputPin};
use ast10x0_peripherals::scu::{self, create_pins, pinctrl};
use console_backend::console_backend_write_all;
use entry as _;
use target_common::{declare_target, TargetInterface};

pub struct Target;

static PINCTRL_GROUPS: [&[ast10x0_peripherals::scu::PinctrlPin]; 3] = [
    pinctrl::PINCTRL_I2C2,
    pinctrl::PINCTRL_FMC_QUAD,
    pinctrl::PINCTRL_SPI1_QUAD,
];

/// Give the SPI1 master a standing path to the staging flash through SPIM0.
///
/// Parking the route here is what lets the staging app drive the flash with
/// plain SPI-NOR commands: `SpiTransaction` only touches the SCU when a caller
/// names a `SpiMonitorInstance`, which `SpiNorFlash` never does. The mock BMC
/// is held in reset for the duration so only one master is on the bus.
#[cfg(feature = "rot")]
fn park_spi1_to_staging_flash() -> bool {
    use ast10x0_board::{
        apply_spim_external_mux, apply_spim_pinctrl, enable_flash_power, release_spi_flash_resets,
        set_bmc_resets,
    };
    use ast10x0_peripherals::scu::{
        ScuExtMuxSelect, ScuRegisters, SpiMonitorInstance, SpiMonitorPassthrough, SpiMonitorSource,
    };

    const SPIM: SpiMonitorInstance = SpiMonitorInstance::Spim0;

    // SAFETY: kernel main() runs once with exclusive hardware ownership.
    let scu = unsafe { ScuRegisters::new_global_unlocked() };

    if !enable_flash_power(&scu) || !release_spi_flash_resets() || !set_bmc_resets(true) {
        return false;
    }

    apply_spim_pinctrl(&scu, SPIM);
    scu.disable_spim_cs_internal_pull_down(SPIM);
    scu.set_spim_passthrough(SPIM, SpiMonitorPassthrough::Enabled);
    apply_spim_external_mux(SPIM, ScuExtMuxSelect::Mux1);
    scu.set_spim_internal_mux(SpiMonitorSource::Spi1, SPIM as u8 + 1)
        .is_ok()
}

/// Point the mock BMC's SPI1 master at the staging flash so the UA can read
/// back what the RoT staged.
///
/// A plain SPI1 master: clock and data leave on this chip's own pins, but chip
/// select still runs through the module-local mux, so that has to point at SPI1
/// or the flash never sees CS assert. The fixture-level select stays untouched —
/// the RoT owns it, and two chips driving one line would fight.
#[cfg(not(feature = "rot"))]
fn park_spi1_to_staging_flash() -> bool {
    use ast10x0_board::apply_spim_module_mux;
    use ast10x0_peripherals::scu::{
        ScuExtMuxSelect, ScuRegisters, SpiMonitorInstance, SpiMonitorSource,
    };

    // SAFETY: kernel main() runs once with exclusive hardware ownership.
    let scu = unsafe { ScuRegisters::new_global_unlocked() };

    if scu
        .set_spim_internal_mux(SpiMonitorSource::Spi1, 0)
        .is_err()
    {
        return false;
    }

    scu.apply_pinctrl_group(pinctrl::PINCTRL_GPIOB4);
    apply_spim_module_mux(SpiMonitorInstance::Spim0, ScuExtMuxSelect::Mux1);
    true
}

impl TargetInterface for Target {
    const NAME: &'static str = "AST10x0 PLDM firmware update";

    fn main() -> ! {
        // SAFETY: kernel main() runs once with exclusive hardware ownership.
        if unsafe {
            Ast10x0Board::new(Ast10x0BoardDescriptor {
                pinctrl_groups: &PINCTRL_GROUPS,
            })
            .init()
        }
        .is_err()
        {
            loop {}
        }

        // GPIOJ0: the fd image's reset-passthrough request line to the Pi harness.
        // SAFETY: sole pin creation site in this binary, at boot; the pins! table is this chip's true pin map.
        let pins = unsafe { create_pins() };
        // SAFETY: kernel-only binary, minted once; no process holds a conflicting grant.
        let gpio = GpioBlock::new(unsafe { take_aperture() });
        let gpio_j0 = pins.scu418_8.into_gpio(&gpio);
        scu::route(&gpio_j0);
        // Driven low (deasserted) immediately so the line has a defined level for
        // the whole test instead of floating until fd_main.rs binds it as output.
        // Holding it low is also how the Pi's mirror arms itself.
        let mut gpio_j0 = gpio_j0.into_output();
        let _ = gpio_j0.set_low();

        // GPIOH4/H5: the boot-confirmation line, wired GPIOH4 (RoT, input) to
        // GPIOH5 (mock BMC, output). Routed on both images since this kernel
        // binary is shared; each userspace app binds only the half its board needs.
        scu::route(&pins.scu414_28.into_gpio(&gpio));
        scu::route(&pins.scu414_29.into_gpio(&gpio));

        if !park_spi1_to_staging_flash() {
            let _ = console_backend_write_all(b"SPI1 staging path setup failed\r\n");
            loop {}
        }

        codegen::start();
        loop {}
    }

    fn shutdown(code: u32) -> ! {
        let sentinel: &[u8] = if code == 0 {
            b"TEST_RESULT:PASS\n"
        } else {
            b"TEST_RESULT:FAIL\n"
        };
        let _ = console_backend_write_all(sentinel);
        #[expect(clippy::empty_loop)]
        loop {}
    }
}

declare_target!(Target);
