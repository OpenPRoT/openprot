// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0
//
// Host tool that emits the MCU ROM linker script for the emulator platform
// by substituting the `EMULATOR_MEMORY_MAP` values into a template.
//
// # Upstream cross-reference
//
// Mirrors `pub fn rom_ld_script(memory_map: &McuMemoryMap) -> String` in
// `caliptra-mcu-sw/builder/src/rom.rs` (around line 130). The template
// string is a direct copy of the `ROM_LD_TEMPLATE` const from that file.
// That function lives in the `mcu-builder` crate, which we cannot depend on
// from Bazel because its sole dependency is `caliptra_builder`, which has
// unresolved upstream deps (`fslock`, `Crypto` trait wiring, `CARGO` env
// var). Our `//third_party/caliptra/caliptra-mcu-sw:mcu_builder` target is
// tagged `manual` for that reason. If/when `caliptra_builder` becomes
// buildable, this file should be replaced by a thin wrapper that calls
// `mcu_builder::rom_ld_script()` directly.

use mcu_config_emulator::EMULATOR_MEMORY_MAP;

const ROM_LD_TEMPLATE: &str = r#"
/* Licensed under the Apache-2.0 license. */

ENTRY(_start)
OUTPUT_ARCH( "riscv" )

MEMORY
{
  ROM   (rx) : ORIGIN = $ROM_OFFSET, LENGTH = $ROM_SIZE
  RAM  (rwx) : ORIGIN = $DCCM_OFFSET, LENGTH = $DCCM_SIZE /* dedicated SRAM for the ROM stack */
}

SECTIONS
{
    .text :
    {
        *(.text.init )
        *(.text*)
        *(.rodata*)
    } > ROM

    ROM_DATA = .;

    .data : AT(ROM_DATA)
    {
        . = ALIGN(4);
        *(.data*);
        *(.sdata*);
        KEEP(*(.eh_frame))
        . = ALIGN(4);
        PROVIDE( GLOBAL_POINTER = . + 0x800 );
        . = ALIGN(4);
    } > RAM

    .bss (NOLOAD) :
    {
        . = ALIGN(4);
        *(.bss*)
        *(.sbss*)
        *(COMMON)
        . = ALIGN(4);
    } > RAM

    .stack (NOLOAD):
    {
        . = ALIGN(4);
        . = . + STACK_SIZE;
        . = ALIGN(4);
        PROVIDE(STACK_START = . );
    } > RAM

    .estack (NOLOAD):
    {
        . = ALIGN(4);
        . = . + ESTACK_SIZE;
        . = ALIGN(4);
        PROVIDE(ESTACK_START = . );
    }

    _end = . ;
}

BSS_START = ADDR(.bss);
BSS_END = BSS_START + SIZEOF(.bss);
DATA_START = ADDR(.data);
DATA_END = DATA_START + SIZEOF(.data);
ROM_DATA_START = LOADADDR(.data);
STACK_SIZE = $ROM_STACK_SIZE;
STACK_TOP = ORIGIN(RAM) + LENGTH(RAM);
STACK_ORIGIN = STACK_TOP - STACK_SIZE;
ESTACK_SIZE = $ROM_ESTACK_SIZE;
MRAC_VALUE = $MRAC_VALUE;

"#;

fn main() {
    let ld_script = subst::substitute(ROM_LD_TEMPLATE, &EMULATOR_MEMORY_MAP.hash_map()).unwrap();
    println!("{ld_script}");
}
