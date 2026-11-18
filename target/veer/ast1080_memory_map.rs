// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! AST1080 Caliptra-SS MCU memory map. Upstream caliptra-mcu-sw has no
//! ast1040/ast1080 platform, so the values come from ASPEED's fork
//! (platforms/ast1040/config/src/lib.rs, AST1040_MEMORY_MAP). Shared by the
//! kernel config and the host-side render_system_config tool.
#![no_std]

use caliptra_mcu_config::McuMemoryMap;

pub const AST1080_MEMORY_MAP: McuMemoryMap = McuMemoryMap {
    sram_offset: 0x21c0_0000,
    sram_size: 512 * 1024,
    dccm_offset: 0x5000_0000,
    dccm_size: 16 * 1024,
    pic_offset: 0x6000_0000,
    mci_offset: 0x2100_0000,
    mbox_offset: 0xa002_0000,
    soc_offset: 0xa003_0000,
    i3c_offset: 0x2000_4000,
    handoff_offset: 0x5000_3c00,
    handoff_size: 1024,
    ..caliptra_mcu_config_emulator::EMULATOR_MEMORY_MAP
};

/// ASPEED SPI FMC control registers (platforms/ast1040/config/src/peri.rs).
pub const AST1080_FMC_CTRL_ADDR: u32 = 0x3400_0000;
