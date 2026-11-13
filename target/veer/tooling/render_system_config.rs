// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Renders a VeeR pw_kernel system config template for one platform, filling
//! in addresses from upstream caliptra-mcu-sw's memory maps.
//!
//! Usage: render_system_config <emulator|fpga> <template> <output>

use caliptra_mcu_config::McuMemoryMap;
use caliptra_mcu_config_emulator::EMULATOR_MEMORY_MAP;
use caliptra_mcu_config_fpga::FPGA_MEMORY_MAP;
use caliptra_ss_registers::primary_flash_ctrl::PRIMARY_FLASH_CTRL_ADDR;
use veer_flash::FPGA_PRIMARY_FLASH_CTRL_ADDR;

/// Offset of the kernel's RAM within MCU SRAM. This is OpenPRoT's own layout
/// choice (code first, data after), not part of the upstream memory map.
const KERNEL_RAM_OFFSET: u32 = 0x4_0000;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let [_, platform, template, output] = args.as_slice() else {
        panic!("usage: render_system_config <emulator|fpga> <template> <output>");
    };

    // The memory maps do not include the flash controllers, so their
    // addresses come from the register definitions and the flash driver.
    let (map, primary_flash_ctrl): (&McuMemoryMap, u32) = match platform.as_str() {
        "emulator" => (&EMULATOR_MEMORY_MAP, PRIMARY_FLASH_CTRL_ADDR),
        "fpga" => (&FPGA_MEMORY_MAP, FPGA_PRIMARY_FLASH_CTRL_ADDR as u32),
        other => panic!("unknown platform {other:?}; expected emulator or fpga"),
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
    //   @SOC_IFC_BASE@        Caliptra SoC interface registers.
    //   @PRIMARY_FLASH_CTRL@  Registers of the primary flash controller.
    //   @I3C_BASE@            Caliptra-SS I3C register block.
    //   @HANDOFF_BASE@        Handoff table that the MCU ROM leaves in DCCM.
    let substitutions = [
        // Start of MCU SRAM. MCU ROM will jump here.
        ("@KERNEL_FLASH_START@", map.sram_offset),
        // Where the system generator starts placing app data
        ("@KERNEL_RAM_START@", map.sram_offset + KERNEL_RAM_OFFSET),
        // MCI register block base offset
        ("@MCI_BASE@", map.mci_offset),
        // Caliptra mailbox registers
        ("@MBOX_BASE@", map.mbox_offset),
        // Caliptra SoC interface registers
        ("@SOC_IFC_BASE@", map.soc_offset),
        ("@PRIMARY_FLASH_CTRL@", primary_flash_ctrl),
        // I3C register block base offset
        ("@I3C_BASE@", map.i3c_offset),
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
