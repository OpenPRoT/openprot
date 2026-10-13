// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Renders a VeeR pw_kernel system config template for one platform, filling
//! in addresses from upstream caliptra-mcu-sw's memory maps.
//!
//! Usage: render_system_config <emulator|fpga> <template> <output>

use mcu_config::McuMemoryMap;
use mcu_config_emulator::EMULATOR_MEMORY_MAP;
use mcu_config_fpga::FPGA_MEMORY_MAP;

/// Offset of the kernel's RAM within MCU SRAM. This is OpenPRoT's own layout
/// choice (code first, data after), not part of the upstream memory map.
const KERNEL_RAM_OFFSET: u32 = 0x4_0000;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let [_, platform, template, output] = args.as_slice() else {
        panic!("usage: render_system_config <emulator|fpga> <template> <output>");
    };

    let map: &McuMemoryMap = match platform.as_str() {
        "emulator" => &EMULATOR_MEMORY_MAP,
        "fpga" => &FPGA_MEMORY_MAP,
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
    let substitutions = [
        // Start of MCU SRAM. MCU ROM will jump here.
        ("@KERNEL_FLASH_START@", map.sram_offset),
        // Where the system generator starts placing app data
        ("@KERNEL_RAM_START@", map.sram_offset + KERNEL_RAM_OFFSET),
        // MCI register block base offset
        ("@MCI_BASE@", map.mci_offset),
    ];

    let mut rendered =
        std::fs::read_to_string(template).unwrap_or_else(|e| panic!("reading {template}: {e}"));
    for (placeholder, value) in substitutions {
        rendered = rendered.replace(placeholder, &format!("{value:#x}"));
    }

    std::fs::write(output, rendered).unwrap_or_else(|e| panic!("writing {output}: {e}"));
}
