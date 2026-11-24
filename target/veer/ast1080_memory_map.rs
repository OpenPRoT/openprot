// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! AST1080 Caliptra-SS MCU memory map. Upstream caliptra-mcu-sw has no
//! ast1040/ast1080 platform, so the values come from ASPEED's fork
//! (platforms/ast1040/config/src/lib.rs, AST1040_MEMORY_MAP), copied in full so
//! no emulator value is inherited. Shared by the
//! kernel config and the host-side render_system_config tool.
#![no_std]

use caliptra_mcu_config::{McuMemoryMap, McuStraps, MemoryRegionType};

pub const AST1080_MEMORY_MAP: McuMemoryMap = McuMemoryMap {
    rom_offset: 0x0000_0000,
    rom_size: 96 * 1024,
    rom_stack_size: 0x2e00,
    rom_estack_size: 0x200,
    rom_properties: MemoryRegionType::MEMORY,

    dccm_offset: 0x5000_0000,
    dccm_size: 16 * 1024,
    dccm_properties: MemoryRegionType::MEMORY,

    sram_offset: 0x21c0_0000,
    sram_size: 512 * 1024,
    sram_properties: MemoryRegionType::MEMORY,

    storage_size: 0x400,

    pic_offset: 0x6000_0000,
    pic_properties: MemoryRegionType::MMIO,

    i3c_offset: 0x2000_4000,
    i3c_size: 0x1000,
    i3c_properties: MemoryRegionType::MMIO,

    i3c1_offset: 0x2000_5000,
    i3c1_size: 0x1000,
    i3c1_properties: MemoryRegionType::MMIO,

    mci_offset: 0x2100_0000,
    mci_size: 0xe0_0000,
    mci_properties: MemoryRegionType::MMIO,

    mbox_offset: 0xa002_0000,
    mbox_size: 0x28,
    mbox_properties: MemoryRegionType::MMIO,

    soc_offset: 0xa003_0000,
    soc_size: 0x5e0,
    soc_properties: MemoryRegionType::MMIO,

    otp_offset: 0x7000_0000,
    otp_size: 0x140,
    otp_properties: MemoryRegionType::MMIO,

    lc_offset: 0x7000_0400,
    lc_size: 0x8c,
    lc_properties: MemoryRegionType::MMIO,
    handoff_offset: 0x5000_3C00,
    handoff_size: 1024,

    staging_sram_offset: 0xb00c_0000,
    staging_sram_size: 1024 * 1024,
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
