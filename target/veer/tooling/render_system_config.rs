// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Renders a VeeR pw_kernel system config template for one platform, filling
//! in addresses from upstream caliptra-mcu-sw's memory maps.
//!
//! Usage: render_system_config <emulator|fpga|ast1080> <template> <output>

use ast1080_memory_map::{AST1080_FMC_CTRL_ADDR, AST1080_MCU_STRAPS, AST1080_MEMORY_MAP};
use caliptra_mcu_config::{McuMemoryMap, McuStraps};
use caliptra_mcu_config_emulator::{EMULATOR_MCU_STRAPS, EMULATOR_MEMORY_MAP};
use caliptra_mcu_config_fpga::{FPGA_MCU_STRAPS, FPGA_MEMORY_MAP};
use caliptra_ss_registers::primary_flash_ctrl::PRIMARY_FLASH_CTRL_ADDR;
use veer_flash::FPGA_PRIMARY_FLASH_CTRL_ADDR;
use veer_mcu_mbox::{CSR_OFFSET, MAX_SRAM_SIZE, MBOX0_OFFSET, MBOX1_OFFSET};

/// Sizes in bytes of the SRAMs of MCU mailbox 0 and 1 of the emulator
/// (`MCU_MAILBOX0_SRAM_SIZE` and `MCU_MAILBOX1_SRAM_SIZE` in caliptra-mcu-sw
/// `emulator/consts`).
const EMULATOR_MCU_MBOX_SRAM_SIZES: [u32; 2] = [2 * 1024 * 1024; 2];

/// Sizes in bytes of the SRAMs of MCU mailbox 0 and 1 of the VCK190
/// bitstream, which `HW_CONFIG0` of its MCI reports.
const FPGA_MCU_MBOX_SRAM_SIZES: [u32; 2] = [16 * 1024; 2];

/// Sizes in bytes of the SRAMs of MCU mailbox 0 and 1 of the AST1080, from
/// ASPEED's caliptra-mcu-sw fork (platforms/ast1040). Mailbox 0 uses all the
/// SRAM address space the MCI reserves for it (its driver clears
/// `mcu_mbox0_csr_mbox_sram.len() * 4` bytes), and mailbox 1 is the 1 MiB
/// staging SRAM that `runtime/src/board.rs` maps.
const AST1080_MCU_MBOX_SRAM_SIZES: [u32; 2] = [MAX_SRAM_SIZE as u32, 1024 * 1024];

/// Offset of the kernel's RAM within MCU SRAM. This is OpenPRoT's own layout
/// choice (code first, data after), not part of the upstream memory map.
const KERNEL_RAM_OFFSET: u32 = 0x4_0000;

/// Register block base and PIC interrupt number of the I3C core that the straps
/// select for MCTP: core 0 (IRQ 2), or core 1 (IRQ 3) when `active_i3c` is 1. Any
/// other value is core 0, as the MCU ROM treats it.
fn active_i3c(map: &McuMemoryMap, straps: &McuStraps) -> (u32, u32) {
    if straps.active_i3c == 1 {
        (map.i3c1_offset, 3)
    } else {
        (map.i3c_offset, 2)
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let [_, platform, template, output] = args.as_slice() else {
        panic!("usage: render_system_config <emulator|fpga|ast1080> <template> <output>");
    };

    // The memory maps do not include the flash controllers, so their
    // addresses come from the register definitions and the flash driver. The
    // maps do not include the sizes of the MCU mailboxes either.
    let (map, straps, primary_flash_ctrl, mcu_mbox_sram_sizes): (
        &McuMemoryMap,
        &McuStraps,
        u32,
        [u32; 2],
    ) = match platform.as_str() {
        "emulator" => (
            &EMULATOR_MEMORY_MAP,
            &EMULATOR_MCU_STRAPS,
            PRIMARY_FLASH_CTRL_ADDR,
            EMULATOR_MCU_MBOX_SRAM_SIZES,
        ),
        "fpga" => (
            &FPGA_MEMORY_MAP,
            &FPGA_MCU_STRAPS,
            FPGA_PRIMARY_FLASH_CTRL_ADDR as u32,
            FPGA_MCU_MBOX_SRAM_SIZES,
        ),
        "ast1080" => (
            &AST1080_MEMORY_MAP,
            &AST1080_MCU_STRAPS,
            AST1080_FMC_CTRL_ADDR,
            AST1080_MCU_MBOX_SRAM_SIZES,
        ),
        other => panic!("unknown platform {other:?}; expected emulator, fpga or ast1080"),
    };
    assert!(
        KERNEL_RAM_OFFSET < map.sram_size,
        "KERNEL_RAM_OFFSET is outside MCU SRAM"
    );

    // Placeholder -> value. Add new template variables here.
    //
    // Placeholders
    //   @KERNEL_FLASH_START@  Start of MCU SRAM. MCU ROM jumps here, so the
    //                         kernel image starts here.
    //   @KERNEL_RAM_START@    Where the system generator starts placing app
    //                         data: KERNEL_RAM_OFFSET into MCU SRAM.
    //   @MCI_BASE@            MCI register block (timer, device mappings).
    //   @MBOX_BASE@           Caliptra mailbox registers.
    //   @MCU_MBOX0_BASE@      SRAM of MCU mailbox 0.
    //   @MCU_MBOX0_CSR_BASE@  Registers of MCU mailbox 0.
    //   @MCU_MBOX0_SRAM_SIZE@ Size in bytes of the SRAM of MCU mailbox 0.
    //   @MCU_MBOX1_BASE@, @MCU_MBOX1_CSR_BASE@, @MCU_MBOX1_SRAM_SIZE@
    //                         The same for MCU mailbox 1.
    //   @SOC_IFC_BASE@        Caliptra SoC interface registers.
    //   @PRIMARY_FLASH_CTRL@  Registers of the primary flash controller.
    //   @I3C_BASE@            Register block of the active Caliptra-SS I3C core.
    //   @I3C_IRQ@             PIC interrupt number of that core.
    //   @HANDOFF_BASE@        Handoff table that the MCU ROM leaves in DCCM.
    let (i3c_base, i3c_irq) = active_i3c(map, straps);
    let substitutions = [
        // Start of MCU SRAM. MCU ROM will jump here.
        ("@KERNEL_FLASH_START@", map.sram_offset),
        // Where the system generator starts placing app data
        ("@KERNEL_RAM_START@", map.sram_offset + KERNEL_RAM_OFFSET),
        // MCI register block base offset
        ("@MCI_BASE@", map.mci_offset),
        // Caliptra mailbox registers
        ("@MBOX_BASE@", map.mbox_offset),
        // The MCU mailboxes, which are inside the MCI block
        ("@MCU_MBOX0_BASE@", map.mci_offset + MBOX0_OFFSET as u32),
        (
            "@MCU_MBOX0_CSR_BASE@",
            map.mci_offset + (MBOX0_OFFSET + CSR_OFFSET) as u32,
        ),
        ("@MCU_MBOX0_SRAM_SIZE@", mcu_mbox_sram_sizes[0]),
        ("@MCU_MBOX1_BASE@", map.mci_offset + MBOX1_OFFSET as u32),
        (
            "@MCU_MBOX1_CSR_BASE@",
            map.mci_offset + (MBOX1_OFFSET + CSR_OFFSET) as u32,
        ),
        ("@MCU_MBOX1_SRAM_SIZE@", mcu_mbox_sram_sizes[1]),
        // Caliptra SoC interface registers
        ("@SOC_IFC_BASE@", map.soc_offset),
        ("@PRIMARY_FLASH_CTRL@", primary_flash_ctrl),
        // Register block and interrupt of the I3C core the straps select
        ("@I3C_BASE@", i3c_base),
        ("@I3C_IRQ@", i3c_irq),
        // Handoff table from the MCU ROM
        ("@HANDOFF_BASE@", map.handoff_offset),
    ];

    let mut rendered =
        std::fs::read_to_string(template).unwrap_or_else(|e| panic!("reading {template}: {e}"));
    for (placeholder, value) in substitutions {
        rendered = rendered.replace(placeholder, &format!("{value:#x}"));
    }

    std::fs::write(output, rendered).unwrap_or_else(|e| panic!("writing {output}: {e}"));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn straps(active_i3c: u8) -> McuStraps {
        McuStraps {
            active_i3c,
            ..EMULATOR_MCU_STRAPS
        }
    }

    #[test]
    fn core_0_is_the_default() {
        for map in [&EMULATOR_MEMORY_MAP, &FPGA_MEMORY_MAP] {
            assert_eq!(active_i3c(map, &straps(0)), (map.i3c_offset, 2));
        }
    }

    #[test]
    fn strap_1_selects_core_1() {
        for map in [&EMULATOR_MEMORY_MAP, &FPGA_MEMORY_MAP] {
            assert_eq!(active_i3c(map, &straps(1)), (map.i3c1_offset, 3));
        }
    }

    #[test]
    fn an_invalid_strap_falls_back_to_core_0() {
        let map = &FPGA_MEMORY_MAP;
        assert_eq!(active_i3c(map, &straps(2)), (map.i3c_offset, 2));
    }

    #[test]
    fn the_two_cores_differ() {
        for map in [&EMULATOR_MEMORY_MAP, &FPGA_MEMORY_MAP] {
            assert_ne!(map.i3c_offset, map.i3c1_offset);
        }
    }
}
