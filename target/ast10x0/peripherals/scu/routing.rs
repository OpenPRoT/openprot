// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! SCU routing and mux helpers for SPI monitor integration.

use super::registers::ScuRegisters;
use super::types::{
    Result, ScuExtMuxSelect, SpiMonitorInstance, SpiMonitorPassthrough, SpiMonitorSource,
};

impl ScuRegisters {
    /// Enable or disable passthrough for a SPI monitor instance.
    pub fn set_spim_passthrough(
        &self,
        instance: SpiMonitorInstance,
        passthrough: SpiMonitorPassthrough,
    ) {
        self.unlock_write_protection();
        let enable = passthrough.is_enabled();

        self.regs().scu0f0().modify(|_, w| match instance {
            SpiMonitorInstance::Spim0 => w.enbl_passthrough_of_spipf1().bit(enable),
            SpiMonitorInstance::Spim1 => w.enbl_passthrough_of_spipf2().bit(enable),
            SpiMonitorInstance::Spim2 => w.enbl_passthrough_of_spipf3().bit(enable),
            SpiMonitorInstance::Spim3 => w.enbl_passthrough_of_spipf4().bit(enable),
        });
    }

    /// Route an internal SPI master through the selected SPI monitor path. bit3
    pub fn set_spim_internal_master_route(
        &self,
        instance: SpiMonitorInstance,
        source: SpiMonitorSource,
    ) {
        self.unlock_write_protection();
        self.regs().scu0f0().modify(|_, w| unsafe {
            w.select_int_spimaster_connection()
                .bits(instance as u8 + 1)
        });

        let select_spi2 = matches!(source, SpiMonitorSource::Spi2);
        self.regs()
            .scu0f0()
            .modify(|_, w| w.int_spimaster_sel().bit(select_spi2));
    }

    /// Disable any internal SPI-master detour route.
    pub fn clear_spim_internal_master_route(&self) {
        self.unlock_write_protection();
        let mut bits = self.regs().scu0f0().read().bits();
        bits &= !0xF;
        self.regs().scu0f0().write(|w| unsafe { w.bits(bits) });
    }

    /// Select the external mux signal for a SPI monitor instance.
    pub fn set_spim_ext_mux(&self, instance: SpiMonitorInstance, mux: ScuExtMuxSelect) {
        self.unlock_write_protection();
        let bit = mux.as_bool();

        self.regs().scu0f0().modify(|_, w| match instance {
            SpiMonitorInstance::Spim0 => w.ext_mux_select_sig_of_spipf1().bit(bit),
            SpiMonitorInstance::Spim1 => w.ext_mux_select_sig_of_spipf2().bit(bit),
            SpiMonitorInstance::Spim2 => w.ext_mux_select_sig_of_spipf3().bit(bit),
            SpiMonitorInstance::Spim3 => w.ext_mux_select_sig_of_spipf4().bit(bit),
        });
    }

    /// Query the external mux signal for a SPI monitor instance.
    #[must_use]
    pub fn get_spim_ext_mux(&self, instance: SpiMonitorInstance) -> ScuExtMuxSelect {
        let bit = self.regs().scu0f0().read();
        let is_mux1 = match instance {
            SpiMonitorInstance::Spim0 => bit.ext_mux_select_sig_of_spipf1().bit(),
            SpiMonitorInstance::Spim1 => bit.ext_mux_select_sig_of_spipf2().bit(),
            SpiMonitorInstance::Spim2 => bit.ext_mux_select_sig_of_spipf3().bit(),
            SpiMonitorInstance::Spim3 => bit.ext_mux_select_sig_of_spipf4().bit(),
        };
        if is_mux1 {
            ScuExtMuxSelect::Mux1
        } else {
            ScuExtMuxSelect::Mux0
        }
    }

    /// Enable or disable the SCU-controlled MISO multi-function pin for a SPI
    /// monitor instance.
    pub fn set_spim_miso_multi_func(&self, instance: SpiMonitorInstance, enable: bool) {
        self.unlock_write_protection();
        match instance {
            SpiMonitorInstance::Spim0 => {
                self.regs()
                    .scu690()
                    .modify(|_, w| w.enbl_qspimonitor1misoin_fn_pin().bit(enable));
            }
            SpiMonitorInstance::Spim1 => {
                self.regs()
                    .scu690()
                    .modify(|_, w| w.enbl_qspimonitor2misoin_fn_pin().bit(enable));
            }
            SpiMonitorInstance::Spim2 => {
                self.regs()
                    .scu690()
                    .modify(|_, w| w.enbl_qspimonitor3misoin_fn_pin().bit(enable));
            }
            SpiMonitorInstance::Spim3 => {
                self.regs()
                    .scu694()
                    .modify(|_, w| w.enbl_qspimonitor4misoin_fn_pin().bit(enable));
            }
        }
    }

    /// Read the raw SPI-monitor routing control register image.
    #[must_use]
    pub fn spim_route_ctrl(&self) -> u32 {
        self.regs().scu0f0().read().bits()
    }

    /// Restore the raw SPI-monitor routing control register image.
    pub fn restore_spim_route_ctrl(&self, route_ctrl: u32) {
        self.unlock_write_protection();
        self.regs().scu0f0().write(|w| unsafe { w.bits(route_ctrl) });
    }

    /// Check that a SPI monitor instance can be represented in SCU routing
    /// operations.
    pub fn validate_spim_instance(&self, instance: SpiMonitorInstance) -> Result<()> {
        match instance {
            SpiMonitorInstance::Spim0
            | SpiMonitorInstance::Spim1
            | SpiMonitorInstance::Spim2
            | SpiMonitorInstance::Spim3 => Ok(()),
        }
    }
}