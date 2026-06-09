// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! SCU routing and mux helpers for SPI monitor integration.

use core::ptr::{read_volatile, write_volatile};

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
    scu_reg_addr: usize,
    scu_bit_mask: u32,
    gpio_addr: usize,
    gpio_bit_mask: u32,
}

// Literal translation of g_ast1060_spim_clk_gpio[] in spi_aspeed.c.
const AST1060_SPIM_CLK_GPIO: [SpimGpioInfo; 4] = [
    SpimGpioInfo {
        scu_reg_addr: 0x7e6e_2690,
        scu_bit_mask: 1 << 7,
        gpio_addr: 0x7e78_0000,
        gpio_bit_mask: 1 << 7,
    },
    SpimGpioInfo {
        scu_reg_addr: 0x7e6e_2690,
        scu_bit_mask: 1 << 21,
        gpio_addr: 0x7e78_0000,
        gpio_bit_mask: 1 << 21,
    },
    SpimGpioInfo {
        scu_reg_addr: 0x7e6e_2694,
        scu_bit_mask: 1 << 3,
        gpio_addr: 0x7e78_0020,
        gpio_bit_mask: 1 << 3,
    },
    SpimGpioInfo {
        scu_reg_addr: 0x7e6e_2694,
        scu_bit_mask: 1 << 17,
        gpio_addr: 0x7e78_0020,
        gpio_bit_mask: 1 << 17,
    },
];

// Literal translation of g_ast1060_spim_cs_gpio[] in spi_aspeed.c.
const AST1060_SPIM_CS_GPIO: [SpimGpioInfo; 4] = [
    SpimGpioInfo {
        scu_reg_addr: 0x7e6e_2690,
        scu_bit_mask: 1 << 1,
        gpio_addr: 0x7e78_0000,
        gpio_bit_mask: 1 << 6,
    },
    SpimGpioInfo {
        scu_reg_addr: 0x7e6e_2690,
        scu_bit_mask: 1 << 20,
        gpio_addr: 0x7e78_0000,
        gpio_bit_mask: 1 << 20,
    },
    SpimGpioInfo {
        scu_reg_addr: 0x7e6e_2694,
        scu_bit_mask: 1 << 2,
        gpio_addr: 0x7e78_0020,
        gpio_bit_mask: 1 << 2,
    },
    SpimGpioInfo {
        scu_reg_addr: 0x7e6e_2694,
        scu_bit_mask: 1 << 16,
        gpio_addr: 0x7e78_0020,
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

        let scu0f0 = self.regs().scu0f0().read().bits();
        let op_idx = match ast1060_spim_op_idx(scu0f0) {
            Some(idx) => idx,
            None => return None,
        };

        let clk = AST1060_SPIM_CLK_GPIO[op_idx];
        let cs = AST1060_SPIM_CS_GPIO[op_idx];
        let mut clk_gpio_ori_val = [0u32; 4];

        unsafe {
            // Change the paired SPIM CLKOUT pin to GPIO mode.
            let mut reg_val = read_volatile(clk.scu_reg_addr as *const u32);
            reg_val &= !clk.scu_bit_mask;
            write_volatile(clk.scu_reg_addr as *mut u32, reg_val);

            // Save its GPIO direction bit, then configure it as an input.
            let clk_dir_addr = (clk.gpio_addr + 0x4) as *mut u32;
            reg_val = read_volatile(clk_dir_addr);
            clk_gpio_ori_val[op_idx] = reg_val & clk.gpio_bit_mask;
            reg_val &= !clk.gpio_bit_mask;
            write_volatile(clk_dir_addr, reg_val);

            // Drive the paired SPIM CSOUT GPIO high and configure it as output.
            let cs_data_addr = cs.gpio_addr as *mut u32;
            reg_val = read_volatile(cs_data_addr);
            reg_val |= cs.gpio_bit_mask;
            write_volatile(cs_data_addr, reg_val);

            let cs_dir_addr = (cs.gpio_addr + 0x4) as *mut u32;
            reg_val = read_volatile(cs_dir_addr);
            reg_val |= cs.gpio_bit_mask;
            write_volatile(cs_dir_addr, reg_val);

            // Change the paired SPIM CSOUT pin to GPIO mode.
            let cs_scu_addr = cs.scu_reg_addr as *mut u32;
            reg_val = read_volatile(cs_scu_addr);
            reg_val &= !cs.scu_bit_mask;
            write_volatile(cs_scu_addr, reg_val);
        }

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

        unsafe {
            // Restore the paired CLKOUT GPIO direction bit.
            let clk_dir_addr = (clk.gpio_addr + 0x4) as *mut u32;
            let mut reg_val = read_volatile(clk_dir_addr);
            reg_val &= !clk.gpio_bit_mask;
            reg_val |= gpio_ori_val.clk_gpio_ori_val[op_idx];
            write_volatile(clk_dir_addr, reg_val);

            // Return paired CLKOUT and CSOUT pins to SPIM mode.
            let clk_scu_addr = clk.scu_reg_addr as *mut u32;
            reg_val = read_volatile(clk_scu_addr);
            reg_val |= clk.scu_bit_mask;
            write_volatile(clk_scu_addr, reg_val);

            let cs_scu_addr = cs.scu_reg_addr as *mut u32;
            reg_val = read_volatile(cs_scu_addr);
            reg_val |= cs.scu_bit_mask;
            write_volatile(cs_scu_addr, reg_val);
        }

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
