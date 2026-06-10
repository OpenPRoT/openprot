// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Static SPI-monitor wiring for AST10x0 boards.
//!
//! Composes the `scu::routing` mux helpers with the `spimonitor::controller`
//! typestate to apply once-per-process external-monitor routing and SPIPF
//! policy at backend init time. Per-transaction reroutes are explicitly out of scope:
//! the SPIPF lock is one-way, and the design doc
//! (`peripherals/spimonitor/planning/overview-and-usage-model.md`) calls for
//! "configure early, validate, lock, and operate under that locked policy."

use ast10x0_peripherals::scu::{
    pinctrl::{
        PINCTRL_SPIM1_DEFAULT, PINCTRL_SPIM2_DEFAULT, PINCTRL_SPIM3_DEFAULT,
        PINCTRL_SPIM4_DEFAULT,
    },
    ScuError, ScuExtMuxSelect, ScuRegisters, SpiMonitorInstance, SpiMonitorPassthrough,
    SpiMonitorSource,
};
use ast10x0_peripherals::smc::SmcController;
use ast10x0_peripherals::spimonitor::{
    LockedSpiMonitor, MonitorPolicy, PassthroughMode, SpiMonitor, SpiMonitorController,
    SpiMonitorError, Uninitialized,
};
use ast1060_pac as device;

/// Static wiring for one external SPI monitor path.
///
/// The `source` identifies the external flash bus associated with this policy.
/// It does not enable the AST1060 internal SPI-master detour.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SpimWiring {
    /// Monitor instance connected to the external bus.
    pub instance: SpiMonitorInstance,
    /// External SPI bus associated with the monitor policy.
    pub source: SpiMonitorSource,
    /// Passthrough enable for the chosen instance.
    pub passthrough: SpiMonitorPassthrough,
    /// External mux selection (board-specific).
    pub ext_mux: ScuExtMuxSelect,
    /// Whether to enable the SCU-controlled MISO multi-function pin.
    pub miso_multi_func: bool,
}

impl SpimWiring {
    /// Default wiring for `SmcController::Spi1` (aspeed-rust SPI0,
    /// `master_idx = 0`) routed through `Spim0` (SPIPF1 @ `0x7E79_1000`).
    #[must_use]
    pub const fn default_spi1_via_spim0() -> Self {
        Self {
            instance: SpiMonitorInstance::Spim0,
            source: SpiMonitorSource::Spi1,
            passthrough: SpiMonitorPassthrough::Enabled,
            ext_mux: ScuExtMuxSelect::Mux0,
            miso_multi_func: true,
        }
    }

    /// Default wiring for `SmcController::Spi2` (aspeed-rust SPI1,
    /// `master_idx = 2`) routed through `Spim2` (SPIPF3 @ `0x7E79_3000`).
    #[must_use]
    pub const fn default_spi2_via_spim2() -> Self {
        Self {
            instance: SpiMonitorInstance::Spim2,
            source: SpiMonitorSource::Spi2,
            passthrough: SpiMonitorPassthrough::Enabled,
            ext_mux: ScuExtMuxSelect::Mux0,
            miso_multi_func: true,
        }
    }
}

/// Errors raised while applying SPIM wiring.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SpimWiringError {
    /// Caller asked for SPIM wiring on FMC, which has no SPIM path.
    InvalidController,
    /// `wiring.source` disagrees with the SPI controller being initialized.
    RouteMismatch,
    /// SCU-side validation rejected the requested instance.
    Scu(ScuError),
    /// SPIPF policy programming or lock failed.
    Monitor(SpiMonitorError),
}

impl From<ScuError> for SpimWiringError {
    fn from(value: ScuError) -> Self {
        Self::Scu(value)
    }
}

impl From<SpiMonitorError> for SpimWiringError {
    fn from(value: SpiMonitorError) -> Self {
        Self::Monitor(value)
    }
}

/// Apply the default SCU pinctrl group for a SPI monitor instance.
///
/// The instance numbering follows the hardware SPIPF blocks: `Spim0` uses
/// the device-tree `spim1` pins, through `Spim3` using the `spim4` pins.
pub fn apply_spim_pinctrl(scu: &ScuRegisters, instance: SpiMonitorInstance) {
    let group = match instance {
        SpiMonitorInstance::Spim0 => PINCTRL_SPIM1_DEFAULT,
        SpiMonitorInstance::Spim1 => PINCTRL_SPIM2_DEFAULT,
        SpiMonitorInstance::Spim2 => PINCTRL_SPIM3_DEFAULT,
        SpiMonitorInstance::Spim3 => PINCTRL_SPIM4_DEFAULT,
    };
    scu.apply_pinctrl_group(group);
}

/// Drive the external mux-select GPIO pair described by the AST1060 DTS.
///
/// SPIM1/2 use GPIO A-D pin 12 and SGPIOM pin 0. SPIM3/4 use GPIO E-H
/// pin 8 and SGPIOM pin 2. Both signals carry the same mux selection.
pub fn apply_spim_external_mux(instance: SpiMonitorInstance, mux: ScuExtMuxSelect) {
    let high = matches!(mux, ScuExtMuxSelect::Mux1);
    let (gpio_group, gpio_mask, sgpio_mask) = match instance {
        SpiMonitorInstance::Spim0 | SpiMonitorInstance::Spim1 => {
            (ExternalMuxGpioGroup::Abcd, 1 << 12, 1 << 0)
        }
        SpiMonitorInstance::Spim2 | SpiMonitorInstance::Spim3 => {
            (ExternalMuxGpioGroup::Efgh, 1 << 8, 1 << 2)
        }
    };

    let gpio = unsafe { &*device::Gpio::ptr() };
    let scu = unsafe { &*device::Scu::ptr() };
    scu.scu41c().modify(|_, w| {
        w.enbl_sgpiomaster_ckfn_pin()
            .set_bit()
            .enbl_sgpiomaster_ldfn_pin()
            .set_bit()
            .enbl_sgpiomaster_dofn_pin()
            .set_bit()
            .enbl_sgpiomaster_difn_pin()
            .set_bit()
    });
    match gpio_group {
        ExternalMuxGpioGroup::Abcd => {
            gpio.gpio000().modify(|r, w| unsafe {
                w.bits(update_bit(r.bits(), gpio_mask, high))
            });
            gpio.gpio004().modify(|r, w| unsafe {
                w.bits(r.bits() | gpio_mask)
            });
        }
        ExternalMuxGpioGroup::Efgh => {
            gpio.gpio020().modify(|r, w| unsafe {
                w.bits(update_bit(r.bits(), gpio_mask, high))
            });
            gpio.gpio024().modify(|r, w| unsafe {
                w.bits(r.bits() | gpio_mask)
            });
        }
    }

    let sgpio = unsafe { &*device::Sgpiom::ptr() };
    sgpio.gpio554().modify(|_, w| unsafe {
        w.enbl_of_serial_gpio()
            .set_bit()
            .numbers_of_serial_gpiopins()
            .bits(16)
            .serial_gpioclk_division()
            .bits(24)
    });
    let latch = sgpio.gpio570().read().bits();
    sgpio
        .gpio500()
        .write(|w| unsafe { w.bits(update_bit(latch, sgpio_mask, high)) });

    crate::delay_us(1_000);
}

/// Read back the two board-level external mux-select outputs.
#[must_use]
pub fn spim_external_mux_state(
    instance: SpiMonitorInstance,
) -> Option<ScuExtMuxSelect> {
    let (gpio_group, gpio_mask, sgpio_mask) = match instance {
        SpiMonitorInstance::Spim0 | SpiMonitorInstance::Spim1 => {
            (ExternalMuxGpioGroup::Abcd, 1 << 12, 1 << 0)
        }
        SpiMonitorInstance::Spim2 | SpiMonitorInstance::Spim3 => {
            (ExternalMuxGpioGroup::Efgh, 1 << 8, 1 << 2)
        }
    };
    let gpio = unsafe { &*device::Gpio::ptr() };
    let gpio_high = match gpio_group {
        ExternalMuxGpioGroup::Abcd => gpio.gpio000().read().bits() & gpio_mask != 0,
        ExternalMuxGpioGroup::Efgh => gpio.gpio020().read().bits() & gpio_mask != 0,
    };
    let sgpio = unsafe { &*device::Sgpiom::ptr() };
    let sgpio_high = sgpio.gpio570().read().bits() & sgpio_mask != 0;
    if gpio_high != sgpio_high {
        None
    } else if gpio_high {
        Some(ScuExtMuxSelect::Mux1)
    } else {
        Some(ScuExtMuxSelect::Mux0)
    }
}

#[derive(Clone, Copy)]
enum ExternalMuxGpioGroup {
    Abcd,
    Efgh,
}

const fn update_bit(value: u32, mask: u32, set: bool) -> u32 {
    if set {
        value | mask
    } else {
        value & !mask
    }
}

/// Apply static SPIM wiring at controller-init time.
///
/// Order: validate → pinctrl → SCU route → passthrough → ext-mux →
/// MISO multi-func → SPIPF policy → SPIPF lock. The lock is one-way; an empty
/// `MonitorPolicy::empty()` combined with lock will brick the SPI bus until
/// reset, so callers should pass a vetted preset (see [`presets`]).
///
/// # Safety
/// Caller must hold exclusive access to the SCU register block and to the
/// target SPIPF block for the lifetime of the returned `LockedSpiMonitor`.
pub unsafe fn apply_spim_wiring(
    scu: &ScuRegisters,
    controller_id: SmcController,
    wiring: SpimWiring,
    policy: &MonitorPolicy,
) -> Result<LockedSpiMonitor, SpimWiringError> {
    unsafe { apply_spim_wiring_with_log(scu, controller_id, wiring, policy, None) }
}

/// Apply static SPIM wiring and optionally configure violation-log DMA before
/// the policy registers are locked.
///
/// # Safety
/// The caller must satisfy [`apply_spim_wiring`] ownership requirements. A
/// supplied log buffer must remain exclusively owned by the monitor forever.
pub unsafe fn apply_spim_wiring_with_log(
    scu: &ScuRegisters,
    controller_id: SmcController,
    wiring: SpimWiring,
    policy: &MonitorPolicy,
    log_buffer: Option<&'static mut [u32]>,
) -> Result<LockedSpiMonitor, SpimWiringError> {
    validate_controller_for_source(controller_id, wiring.source)?;
    scu.validate_spim_instance(wiring.instance)?;

    apply_spim_pinctrl(scu, wiring.instance);
    scu.disable_spim_cs_internal_pull_down(wiring.instance);
    // SPIPF monitors the external BMC/host pins. Keep SCU0F0[3:0] clear so
    // neither AST1060 internal SPI controller is detoured into the monitor.
    scu.set_spim_internal_mux(wiring.source, 0)?;
    scu.set_spim_passthrough(wiring.instance, wiring.passthrough);
    scu.set_spim_ext_mux(wiring.instance, wiring.ext_mux);
    apply_spim_external_mux(wiring.instance, wiring.ext_mux);
    scu.set_spim_miso_multi_func(wiring.instance, wiring.miso_multi_func);
    scu.set_spim_filter(wiring.instance, true);

    let monitor_controller = match wiring.instance {
        SpiMonitorInstance::Spim0 => SpiMonitorController::Spim0,
        SpiMonitorInstance::Spim1 => SpiMonitorController::Spim1,
        SpiMonitorInstance::Spim2 => SpiMonitorController::Spim2,
        SpiMonitorInstance::Spim3 => SpiMonitorController::Spim3,
    };

    // SAFETY: Caller upholds exclusive SPIPF block access for the chosen
    // instance, mirroring the SCU exclusivity required above.
    let monitor = unsafe { SpiMonitor::<Uninitialized>::new(monitor_controller) };
    let configured = monitor.apply_policy(policy)?;
    if let Some(buffer) = log_buffer {
        configured.configure_log(buffer)?;
    }
    configured.set_push_pull(true);
    configured.set_passthrough(PassthroughMode::Disabled);
    configured.enable();
    let locked = configured.lock()?;
    Ok(locked)
}

fn validate_controller_for_source(
    controller_id: SmcController,
    source: SpiMonitorSource,
) -> Result<(), SpimWiringError> {
    match (controller_id, source) {
        (SmcController::Fmc, _) => Err(SpimWiringError::InvalidController),
        (SmcController::Spi1, SpiMonitorSource::Spi1) => Ok(()),
        (SmcController::Spi2, SpiMonitorSource::Spi2) => Ok(()),
        (SmcController::Spi1, SpiMonitorSource::Spi2)
        | (SmcController::Spi2, SpiMonitorSource::Spi1) => Err(SpimWiringError::RouteMismatch),
    }
}

/// Built-in `MonitorPolicy` presets vetted against the BMC's flash opcode set.
pub mod presets {
    use ast10x0_peripherals::spimonitor::{
        profile, MonitorPolicy, PrivilegeDirection, PrivilegeOp,
    };

    /// Allow-list for the BMC's normal flash opcodes covering both 3-byte and
    /// 4-byte addressing variants. Empty `regions` (no address-privilege
    /// filter applied).
    ///
    /// Opcodes:
    /// `READ` (`0x03`), `FAST_READ` (`0x0B`), `FAST_READ_4B` (`0x0C`),
    /// `PP` (`0x02`), `PP_4B` (`0x12`),
    /// `SE_4K` (`0x20`), `SE_4K_4B` (`0x21`),
    /// `RDSR` (`0x05`), `WREN` (`0x06`), `WRDI` (`0x04`),
    /// `RDID` (`0x9F`), `RSTEN` (`0x66`), `RST` (`0x99`).
    #[must_use]
    pub const fn bmc_default_policy() -> MonitorPolicy {
        let mut p = MonitorPolicy::empty();
        p.allow_commands[0] = 0x03; // READ
        p.allow_commands[1] = 0x0B; // FAST_READ
        p.allow_commands[2] = 0x0C; // FAST_READ_4B
        p.allow_commands[3] = 0x02; // PP
        p.allow_commands[4] = 0x12; // PP_4B
        p.allow_commands[5] = 0x20; // SE_4K
        p.allow_commands[6] = 0x21; // SE_4K_4B
        p.allow_commands[7] = 0x05; // RDSR
        p.allow_commands[8] = 0x06; // WREN
        p.allow_commands[9] = 0x04; // WRDI
        p.allow_commands[10] = 0x9F; // RDID
        p.allow_commands[11] = 0x66; // RSTEN
        p.allow_commands[12] = 0x99; // RST
        p.allow_command_count = 13;
        p
    }

    /// Policy matching the supplied Zephyr SPIM nodes: full command list and
    /// write protection over flash addresses `0x0000_0000..0x0800_0000`.
    #[must_use]
    pub fn zephyr_spim_policy() -> MonitorPolicy {
        let mut policy = profile::zephyr_default();
        let _ = policy.add_region(
            0,
            0x0800_0000,
            PrivilegeDirection::Write,
            PrivilegeOp::Disable,
        );
        policy
    }
}
