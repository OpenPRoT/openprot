// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! AST1080 Caliptra-SS MCU memory map. Upstream caliptra-mcu-sw has no
//! ast1040/ast1080 platform, so the values come from ASPEED's fork
//! (platforms/ast1040/config/src/lib.rs, AST1040_MEMORY_MAP). Shared by the
//! kernel config and the host-side render_system_config tool.
#![no_std]

use caliptra_mcu_config::{McuMemoryMap, McuStraps};

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
    i3c1_offset: 0x2000_5000,
    handoff_offset: 0x5000_3c00,
    handoff_size: 1024,
    ..caliptra_mcu_config_emulator::EMULATOR_MEMORY_MAP
};

/// Straps, from AST1040_MCU_STRAPS in the same fork file. Core 0 is the active
/// I3C core.
pub const AST1080_MCU_STRAPS: McuStraps = McuStraps {
    i3c_static_addr: 0x3a,
    i3c1_static_addr: 0x3c,
    active_i3c: 0,
    cptra_wdt_cfg0: 100_000_000,
    cptra_wdt_cfg1: 100_000_000,
    mcu_wdt_cfg0: 0xffff_ffff,
    mcu_wdt_cfg1: 0xffff_ffff,
    mcu_wdt_cfg0_manufacturing: 80_000_000,
    mcu_wdt_cfg1_manufacturing: 1,
    mcu_wdt_cfg0_debug: 80_000_000,
    mcu_wdt_cfg1_debug: 1,
};

/// ASPEED SPI FMC control registers (platforms/ast1040/config/src/peri.rs).
pub const AST1080_FMC_CTRL_ADDR: u32 = 0x3400_0000;
