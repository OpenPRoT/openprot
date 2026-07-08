// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! SCU routing and mux helpers for SPI monitor integration.

use super::registers::ScuRegisters;
use super::types::{
    Result, ScuExtMuxSelect, SpiMonitorInstance, SpiMonitorPassthrough, SpiMonitorSource,
};
const PIN_SPIM0_CLK_OUT_BIT: u32 = 7;
const PIN_SPIM1_CLK_OUT_BIT: u32 = 21;
const PIN_SPIM2_CLK_OUT_BIT: u32 = 3;
const PIN_SPIM3_CLK_OUT_BIT: u32 = 17;

pub type SpimGpioOriVal = [u32; 4];

macro_rules! modify_reg {
    ($reg:expr, $bit:expr, $clear:expr) => {{
        let mut val: u32 = $reg.read().bits();
        if $clear {
            val &= !(1 << $bit);
        } else {
            val |= 1 << $bit;
        }
        $reg.write(|w| unsafe { w.bits(val) });
    }};
}

impl ScuRegisters {
    /// Enable or disable passthrough for a SPI monitor instance. Uses SCU0F0[7:4].
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

    /// Route an internal SPI master through the selected SPI monitor path.
    pub fn set_spim_internal_master_route(
        &self,
        instance: SpiMonitorInstance,
        source: SpiMonitorSource,
    ) {
        self.unlock_write_protection();
        self.regs()
            .scu0f0()
            .modify(|_, w| unsafe { w.select_int_spimaster_connection().bits(instance as u8 + 1) });

        let select_spi2 = matches!(source, SpiMonitorSource::Spi2);
        self.regs()
            .scu0f0()
            .modify(|_, w| w.int_spimaster_sel().bit(select_spi2));
    }

    /// Select the internal SPI master source and SPIM output path.
    ///
    /// `output_select` is the raw SCU0F0[2:0] value. Zero disables the
    /// internal SPI master path; values 1 through 4 select SPIM0 through SPIM3.
    pub fn set_spim_internal_mux(&self, source: SpiMonitorSource, output_select: u8) -> Result<()> {
        if output_select > 4 {
            return Err(super::types::ScuError::InvalidMuxSelection);
        }

        self.unlock_write_protection();
        let source_bit = if matches!(source, SpiMonitorSource::Spi2) {
            0x8
        } else {
            0
        };
        let mut bits = self.regs().scu0f0().read().bits();
        bits = (bits & !0xF) | source_bit | u32::from(output_select);
        self.regs().scu0f0().write(|w| unsafe { w.bits(bits) });
        Ok(())
    }

    /// Disable any internal SPI-master detour route.
    pub fn clear_spim_internal_master_route(&self) {
        self.unlock_write_protection();
        let mut bits = self.regs().scu0f0().read().bits();
        bits &= !0xF;
        self.regs().scu0f0().write(|w| unsafe { w.bits(bits) });
    }

    /// Apply AST1060 SPIM proprietary pin setup before a transaction.
    pub fn spim_proprietary_pre_config(&self) -> Option<SpimGpioOriVal> {
        self.unlock_write_protection();

        let scu = self.regs();
        let gpio = unsafe { &*ast1060_pac::Gpio::ptr() };

        let scu0f0 = scu.scu0f0().read().bits();
        if scu0f0 & 0x7 == 0 {
            return None;
        }

        let spim_idx = (scu0f0 & 0x7) - 1;
        if spim_idx > 3 {
            return None;
        }

        let mut gpio_ori_val = [0; 4];

        for idx in 0..4 {
            if idx as u32 == spim_idx {
                continue;
            }

            match idx {
                0 => {
                    modify_reg!(scu.scu690(), PIN_SPIM0_CLK_OUT_BIT, true);
                    gpio_ori_val[0] = gpio.gpio004().read().bits();
                    modify_reg!(gpio.gpio004(), PIN_SPIM0_CLK_OUT_BIT, true);
                }
                1 => {
                    modify_reg!(scu.scu690(), PIN_SPIM1_CLK_OUT_BIT, true);
                    gpio_ori_val[1] = gpio.gpio004().read().bits();
                    modify_reg!(gpio.gpio004(), PIN_SPIM1_CLK_OUT_BIT, true);
                }
                2 => {
                    modify_reg!(scu.scu694(), PIN_SPIM2_CLK_OUT_BIT, true);
                    gpio_ori_val[2] = gpio.gpio024().read().bits();
                    modify_reg!(gpio.gpio024(), PIN_SPIM2_CLK_OUT_BIT, true);
                }
                3 => {
                    modify_reg!(scu.scu694(), PIN_SPIM3_CLK_OUT_BIT, true);
                    gpio_ori_val[3] = gpio.gpio024().read().bits();
                    modify_reg!(gpio.gpio024(), PIN_SPIM3_CLK_OUT_BIT, true);
                }
                _ => {}
            }
        }

        Some(gpio_ori_val)
    }

    /// Restore AST1060 SPIM proprietary pin state after a transaction.
    pub fn spim_proprietary_post_config(&self, gpio_ori_val: SpimGpioOriVal) {
        self.unlock_write_protection();

        let scu = self.regs();
        let gpio = unsafe { &*ast1060_pac::Gpio::ptr() };

        let bits = scu.scu0f0().read().bits();
        if bits.trailing_zeros() >= 3 {
            return;
        }

        let spim_idx = (bits & 0x7) - 1;
        if spim_idx > 3 {
            return;
        }

        for idx in 0..4 {
            if idx as u32 == spim_idx {
                continue;
            }

            match idx {
                0 => {
                    let ori_val = gpio_ori_val[0];
                    gpio.gpio004().modify(|r, w| unsafe {
                        let mut current = r.bits();
                        current &= !(1 << PIN_SPIM0_CLK_OUT_BIT);
                        current |= ori_val;
                        w.bits(current)
                    });
                    modify_reg!(scu.scu690(), PIN_SPIM0_CLK_OUT_BIT, false);
                }
                1 => {
                    let ori_val = gpio_ori_val[1];
                    gpio.gpio004().modify(|r, w| unsafe {
                        let mut current = r.bits();
                        current &= !(1 << PIN_SPIM1_CLK_OUT_BIT);
                        current |= ori_val;
                        w.bits(current)
                    });
                    modify_reg!(gpio.gpio004(), PIN_SPIM1_CLK_OUT_BIT, false);
                }
                2 => {
                    let ori_val = gpio_ori_val[2];
                    gpio.gpio024().modify(|r, w| unsafe {
                        let mut current = r.bits();
                        current &= !(1 << PIN_SPIM2_CLK_OUT_BIT);
                        current |= ori_val;
                        w.bits(current)
                    });
                    modify_reg!(scu.scu694(), PIN_SPIM2_CLK_OUT_BIT, false);
                }
                3 => {
                    let ori_val = gpio_ori_val[3];
                    gpio.gpio024().modify(|r, w| unsafe {
                        let mut current = r.bits();
                        current &= !(1 << PIN_SPIM3_CLK_OUT_BIT);
                        current |= ori_val;
                        w.bits(current)
                    });
                    modify_reg!(scu.scu694(), PIN_SPIM3_CLK_OUT_BIT, false);
                }
                _ => {}
            }
        }
    }

    /// Select the external mux signal for a SPI monitor instance. Uses SCU0F0[15:12].
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

    /// Enable or disable the SCU-side SPI monitor block for an instance.
    ///
    /// Uses SCU0F0[11:8] (`enbl_filter_of_spipfN`). Required together with
    /// SPIPF000 single passthrough for push-pull IO.
    pub fn set_spim_scu_monitor_enable(&self, instance: SpiMonitorInstance, enable: bool) {
        self.unlock_write_protection();
        self.regs().scu0f0().modify(|_, w| match instance {
            SpiMonitorInstance::Spim0 => w.enbl_filter_of_spipf1().bit(enable),
            SpiMonitorInstance::Spim1 => w.enbl_filter_of_spipf2().bit(enable),
            SpiMonitorInstance::Spim2 => w.enbl_filter_of_spipf3().bit(enable),
            SpiMonitorInstance::Spim3 => w.enbl_filter_of_spipf4().bit(enable),
        });
    }

    /// Disable the SPI master CS internal pull-down for a monitor instance.
    ///
    /// SPI_M1 (SPIM0): GPIOA6 / SCU610[6]; SPI_M2 (SPIM1): GPIOC0 / SCU610[20];
    /// SPI_M4 (SPIM3): GPIOG0 / SCU614[16]. SPIM2 has no disable bit.
    pub fn spim_disable_cs_internal_pd(&self, instance: SpiMonitorInstance) {
        self.unlock_write_protection();
        match instance {
            SpiMonitorInstance::Spim0 => {
                self.regs()
                    .scu610()
                    .modify(|_, w| w.dis_gpioa6int_pull_down().bit(true));
            }
            SpiMonitorInstance::Spim1 => {
                self.regs()
                    .scu610()
                    .modify(|_, w| w.dis_gpioc0int_pull_down().bit(true));
            }
            SpiMonitorInstance::Spim2 => {}
            SpiMonitorInstance::Spim3 => {
                self.regs()
                    .scu614()
                    .modify(|_, w| w.dis_gpiog0int_pull_down().bit(true));
            }
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
