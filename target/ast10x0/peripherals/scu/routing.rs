// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! SCU routing and mux helpers for SPI monitor integration.

use ast1060_pac as device;

use super::registers::ScuRegisters;
use super::types::{
    Result, ScuExtMuxSelect, SpiMonitorInstance, SpiMonitorPassthrough, SpiMonitorSource,
};

#[derive(Clone, Copy)]
pub struct SpimGpioOriVal {
    clk_gpio_ori_val: [u32; 4],
}

#[derive(Clone, Copy)]
struct SpimGpioInfo {
    scu_register: SpimScuRegister,
    scu_bit_mask: u32,
    gpio_group: GpioGroup,
    gpio_bit_mask: u32,
}

#[derive(Clone, Copy)]
enum SpimScuRegister {
    Scu690,
    Scu694,
}

#[derive(Clone, Copy)]
enum GpioGroup {
    Abcd,
    Efgh,
}

fn gpio_regs() -> &'static device::gpio::RegisterBlock {
    // SAFETY: The PAC supplies the GPIO register block base address.
    unsafe { &*device::Gpio::ptr() }
}

fn gpio_direction(group: GpioGroup) -> u32 {
    match group {
        GpioGroup::Abcd => gpio_regs().gpio004().read().bits(),
        GpioGroup::Efgh => gpio_regs().gpio024().read().bits(),
    }
}

fn gpio_set_direction(group: GpioGroup, mask: u32, output: bool) {
    let update = |bits: u32| {
        if output {
            bits | mask
        } else {
            bits & !mask
        }
    };

    match group {
        GpioGroup::Abcd => gpio_regs()
            .gpio004()
            .modify(|r, w| unsafe { w.bits(update(r.bits())) }),
        GpioGroup::Efgh => gpio_regs()
            .gpio024()
            .modify(|r, w| unsafe { w.bits(update(r.bits())) }),
    };
}

fn gpio_set_data(group: GpioGroup, mask: u32, high: bool) {
    let update = |bits: u32| {
        if high {
            bits | mask
        } else {
            bits & !mask
        }
    };

    match group {
        GpioGroup::Abcd => gpio_regs()
            .gpio000()
            .modify(|r, w| unsafe { w.bits(update(r.bits())) }),
        GpioGroup::Efgh => gpio_regs()
            .gpio020()
            .modify(|r, w| unsafe { w.bits(update(r.bits())) }),
    };
}

fn gpio_set_output(group: GpioGroup, mask: u32, high: bool) {
    gpio_set_data(group, mask, high);
    gpio_set_direction(group, mask, true);
}

// Literal translation of g_ast1060_spim_clk_gpio[] in spi_aspeed.c.
const AST1060_SPIM_CLK_GPIO: [SpimGpioInfo; 4] = [
    SpimGpioInfo {
        scu_register: SpimScuRegister::Scu690,
        scu_bit_mask: 1 << 7,
        gpio_group: GpioGroup::Abcd,
        gpio_bit_mask: 1 << 7,
    },
    SpimGpioInfo {
        scu_register: SpimScuRegister::Scu690,
        scu_bit_mask: 1 << 21,
        gpio_group: GpioGroup::Abcd,
        gpio_bit_mask: 1 << 21,
    },
    SpimGpioInfo {
        scu_register: SpimScuRegister::Scu694,
        scu_bit_mask: 1 << 3,
        gpio_group: GpioGroup::Efgh,
        gpio_bit_mask: 1 << 3,
    },
    SpimGpioInfo {
        scu_register: SpimScuRegister::Scu694,
        scu_bit_mask: 1 << 17,
        gpio_group: GpioGroup::Efgh,
        gpio_bit_mask: 1 << 17,
    },
];

// Literal translation of g_ast1060_spim_cs_gpio[] in spi_aspeed.c.
const AST1060_SPIM_CS_GPIO: [SpimGpioInfo; 4] = [
    SpimGpioInfo {
        scu_register: SpimScuRegister::Scu690,
        scu_bit_mask: 1 << 1,
        gpio_group: GpioGroup::Abcd,
        gpio_bit_mask: 1 << 6,
    },
    SpimGpioInfo {
        scu_register: SpimScuRegister::Scu690,
        scu_bit_mask: 1 << 20,
        gpio_group: GpioGroup::Abcd,
        gpio_bit_mask: 1 << 20,
    },
    SpimGpioInfo {
        scu_register: SpimScuRegister::Scu694,
        scu_bit_mask: 1 << 2,
        gpio_group: GpioGroup::Efgh,
        gpio_bit_mask: 1 << 2,
    },
    SpimGpioInfo {
        scu_register: SpimScuRegister::Scu694,
        scu_bit_mask: 1 << 16,
        gpio_group: GpioGroup::Efgh,
        gpio_bit_mask: 1 << 16,
    },
];

fn ast1060_spim_op_idx(scu0f0: u32) -> Option<usize> {
    if scu0f0 & 0x7 == 0 {
        return None;
    }

    match (scu0f0 & 0x7) - 1 {
        2 => Some(3),
        3 => Some(2),
        _ => None,
    }
}

impl ScuRegisters {
    fn set_spim_pin_function(&self, pin: SpimGpioInfo, enabled: bool) {
        let update = |bits: u32| {
            if enabled {
                bits | pin.scu_bit_mask
            } else {
                bits & !pin.scu_bit_mask
            }
        };

        match pin.scu_register {
            SpimScuRegister::Scu690 => self
                .regs()
                .scu690()
                .modify(|r, w| unsafe { w.bits(update(r.bits())) }),
            SpimScuRegister::Scu694 => self
                .regs()
                .scu694()
                .modify(|r, w| unsafe { w.bits(update(r.bits())) }),
        };
    }

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

    /// Enable or disable the SCU-side SPI monitor filter path. Uses SCU0F0[11:8].
    pub fn set_spim_filter(&self, instance: SpiMonitorInstance, enable: bool) {
        self.unlock_write_protection();
        self.regs().scu0f0().modify(|_, w| match instance {
            SpiMonitorInstance::Spim0 => w.enbl_filter_of_spipf1().bit(enable),
            SpiMonitorInstance::Spim1 => w.enbl_filter_of_spipf2().bit(enable),
            SpiMonitorInstance::Spim2 => w.enbl_filter_of_spipf3().bit(enable),
            SpiMonitorInstance::Spim3 => w.enbl_filter_of_spipf4().bit(enable),
        });
    }

    /// Route the external flash-reset input through one SPIPF.
    ///
    /// Clears the instance's reset signal and source-selection bits in
    /// SCU0F0[19:16] and [23:20], and enables its reset output in [27:24].
    pub fn configure_spim_external_flash_reset(&self, instance: SpiMonitorInstance) {
        self.unlock_write_protection();
        let instance_bit = match instance {
            SpiMonitorInstance::Spim0 => 1 << 0,
            SpiMonitorInstance::Spim1 => 1 << 1,
            SpiMonitorInstance::Spim2 => 1 << 2,
            SpiMonitorInstance::Spim3 => 1 << 3,
        };
        let clear_mask = (instance_bit << 16) | (instance_bit << 20);
        let output_enable = instance_bit << 24;
        self.regs()
            .scu0f0()
            .modify(|r, w| unsafe { w.bits((r.bits() & !clear_mask) | output_enable) });
    }

    /// Check the external flash-reset source and output-enable configuration.
    #[must_use]
    pub fn is_spim_external_flash_reset_configured(&self, instance: SpiMonitorInstance) -> bool {
        let instance_bit = match instance {
            SpiMonitorInstance::Spim0 => 1 << 0,
            SpiMonitorInstance::Spim1 => 1 << 1,
            SpiMonitorInstance::Spim2 => 1 << 2,
            SpiMonitorInstance::Spim3 => 1 << 3,
        };
        let cleared_mask = (instance_bit << 16) | (instance_bit << 20);
        let output_enable = instance_bit << 24;
        let value = self.regs().scu0f0().read().bits();
        value & cleared_mask == 0 && value & output_enable == output_enable
    }

    /// Disable the internal pull-down on the monitor CS output pin.
    pub fn disable_spim_cs_internal_pull_down(&self, instance: SpiMonitorInstance) {
        self.unlock_write_protection();
        match instance {
            SpiMonitorInstance::Spim0 => {
                self.regs()
                    .scu610()
                    .modify(|r, w| unsafe { w.bits(r.bits() | (1 << 6)) });
            }
            SpiMonitorInstance::Spim1 => {
                self.regs()
                    .scu610()
                    .modify(|r, w| unsafe { w.bits(r.bits() | (1 << 20)) });
            }
            // SPIM3's corresponding pin cannot have its pull-down disabled.
            SpiMonitorInstance::Spim2 => {}
            SpiMonitorInstance::Spim3 => {
                self.regs()
                    .scu614()
                    .modify(|r, w| unsafe { w.bits(r.bits() | (1 << 16)) });
            }
        }
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

        let scu0f0 = self.regs().scu0f0().read().bits();
        pw_log::debug!("SPIM pre-config: SCU0F0=0x{:08x}", scu0f0 as u32);
        let op_idx = match ast1060_spim_op_idx(scu0f0) {
            Some(idx) => idx,
            None => return None,
        };
        pw_log::debug!(
            "SPIM pre-config: active SPIM index {}, op index {}",
            ((scu0f0 & 0x7) - 1) as u32,
            op_idx as u32
        );

        let clk = AST1060_SPIM_CLK_GPIO[op_idx];
        let cs = AST1060_SPIM_CS_GPIO[op_idx];
        let mut clk_gpio_ori_val = [0u32; 4];
        // Change the paired SPIM CLKOUT pin to GPIO mode.
        self.set_spim_pin_function(clk, false);

        // Save its GPIO direction bit, then configure it as an input.
        clk_gpio_ori_val[op_idx] = gpio_direction(clk.gpio_group) & clk.gpio_bit_mask;
        gpio_set_direction(clk.gpio_group, clk.gpio_bit_mask, false);

        // Drive the paired SPIM CSOUT GPIO high and configure it as output.
        gpio_set_output(cs.gpio_group, cs.gpio_bit_mask, true);

        // Change the paired SPIM CSOUT pin to GPIO mode.
        self.set_spim_pin_function(cs, false);

        pw_log::debug!(
            "SPIM pre-config: op index {}, saved CLK direction mask 0x{:08x}",
            op_idx as u32,
            clk_gpio_ori_val[op_idx] as u32
        );
        Some(SpimGpioOriVal { clk_gpio_ori_val })
    }

    /// Restore AST1060 SPIM proprietary pin state after a transaction.
    pub fn spim_proprietary_post_config(&self, gpio_ori_val: SpimGpioOriVal) {
        self.unlock_write_protection();

        let scu0f0 = self.regs().scu0f0().read().bits();
        let op_idx = match ast1060_spim_op_idx(scu0f0) {
            Some(idx) => idx,
            None => return,
        };
        let clk = AST1060_SPIM_CLK_GPIO[op_idx];
        let cs = AST1060_SPIM_CS_GPIO[op_idx];
        pw_log::debug!(
            "SPIM post-config: SCU0F0=0x{:08x}, op index {}, saved CLK direction mask 0x{:08x}",
            scu0f0 as u32,
            op_idx as u32,
            gpio_ori_val.clk_gpio_ori_val[op_idx] as u32
        );

        // Restore the paired CLKOUT GPIO direction bit.
        let was_output = gpio_ori_val.clk_gpio_ori_val[op_idx] != 0;
        gpio_set_direction(clk.gpio_group, clk.gpio_bit_mask, was_output);

        // Return paired CLKOUT and CSOUT pins to SPIM mode.
        self.set_spim_pin_function(clk, true);
        self.set_spim_pin_function(cs, true);

        pw_log::debug!("SPIM post-config: restore complete");
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
