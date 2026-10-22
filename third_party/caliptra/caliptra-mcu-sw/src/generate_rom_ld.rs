// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0
//
// Host tool that emits the MCU ROM linker script for a platform by
// substituting that platform's memory map into a template.
//
// Usage: generate_rom_ld <emulator|fpga>
//
// # Upstream cross-reference
//
// Mirrors the ROM linker script generation in `caliptra-mcu-sw`:
// `firmware-bundler/src/ld.rs` (`rom_linker_content`) and
// `firmware-bundler/data/default-rom-layout.ld`.

use caliptra_mcu_config_emulator::EMULATOR_MEMORY_MAP;
use caliptra_mcu_config_fpga::FPGA_MEMORY_MAP;

const ROM_LD_TEMPLATE: &str = r#"
/* Licensed under the Apache-2.0 license. */

ENTRY(_start)
OUTPUT_ARCH( "riscv" )

/*
 * The last 48 bytes of the ROM image are reserved for a SHA-384 digest that is
 * appended by the build step (see `append_rom_digest`). Carve them out of the
 * ROM region so the linker refuses to place code or rodata there.
 */
ROM_DIGEST_SIZE = 48;

MEMORY
{
  ROM   (rx) : ORIGIN = $ROM_OFFSET, LENGTH = $ROM_SIZE - ROM_DIGEST_SIZE
  RAM  (rwx) : ORIGIN = $DCCM_OFFSET, LENGTH = $DCCM_SIZE - $HANDOFF_SIZE /* dedicated SRAM for the ROM stack */
  HANDOFF (rw) : ORIGIN = $HANDOFF_OFFSET, LENGTH = $HANDOFF_SIZE
}

SECTIONS
{
    .text :
    {
        *(.text.init )
        *(.text*)
        *(.rodata*)
        . = ALIGN(4);
    } > ROM

    ROM_DATA = .;

    .stack (NOLOAD):
    {
        . = ALIGN(4);
        . = . + STACK_SIZE;
        . = ALIGN(4);
    } > RAM

    .estack (NOLOAD):
    {
        . = ALIGN(4);
        . = . + ESTACK_SIZE;
        . = ALIGN(4);
        PROVIDE(ESTACK_START = . );
    } > RAM

    .data : AT(ROM_DATA)
    {
        . = ALIGN(4);
        *(.data*);
        *(.sdata*);
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

    /* We reserve 1 KB at the end of DCCM for the handoff table.
       This must be 1KB aligned because the runtime requires DCCM regions
       to be 1KB aligned for MEIVT (Machine External Interrupt Vector Table). */
    .handoff (NOLOAD) :
    {
        KEEP(*(.handoff))
    } > HANDOFF

    _end = . ;

    /DISCARD/ :
    {
        *(.eh_frame*)
    }
}

BSS_START = ADDR(.bss);
BSS_END = BSS_START + SIZEOF(.bss);
DATA_START = ADDR(.data);
DATA_END = DATA_START + SIZEOF(.data);
ROM_DATA_START = LOADADDR(.data);
STACK_SIZE = $ROM_STACK_SIZE;
STACK_TOP = ORIGIN(RAM) + SIZEOF(.stack);
STACK_ORIGIN = ORIGIN(RAM);
ESTACK_SIZE = $ROM_ESTACK_SIZE;

"#;

fn main() {
    let platform = std::env::args()
        .nth(1)
        .expect("usage: generate_rom_ld <emulator|fpga>");
    let memory_map = match platform.as_str() {
        "emulator" => &EMULATOR_MEMORY_MAP,
        "fpga" => &FPGA_MEMORY_MAP,
        other => panic!("unknown platform {other:?}; expected emulator or fpga"),
    };
    let ld_script = subst::substitute(ROM_LD_TEMPLATE, &memory_map.hash_map()).unwrap();
    println!("{ld_script}");
}
