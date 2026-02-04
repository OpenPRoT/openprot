// Copyright 2025 The Pigweed Authors
//
// Licensed under the Apache License, Version 2.0 (the "License"); you may not
// use this file except in compliance with the License. You may obtain a copy of
// the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS, WITHOUT
// WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied. See the
// License for the specific language governing permissions and limitations under
// the License.
#![no_std]
#![no_main]
use core::arch::global_asm;

use arch_riscv::Arch;
use kernel::{self as _};

mod epmp;

#[unsafe(no_mangle)]
#[allow(non_snake_case)]
pub unsafe extern "C" fn pw_assert_HandleFailure() -> ! {
    use kernel::Arch as _;
    Arch::panic()
}

#[riscv_rt::entry]
fn main() -> ! {
    kernel::static_init_state!(static mut INIT_STATE: InitKernelState<Arch>);

    epmp::init();

    // SAFETY: `main` is only executed once, so we never generate more than one
    // `&mut` reference to `INIT_STATE`.
    #[allow(static_mut_refs)]
    kernel::main(Arch, unsafe { &mut INIT_STATE });
}

global_asm!(
    "
    .option push
    .option norvc
    .option norelax
    .balign 256
    .global _mtvec_table
_mtvec_table:
    j _start_trap /* 0: exception and user software interrupt */
    j _start_trap /* 1: supervisor software interrupt */
    j _start_trap /* 2: reserved */
    j _start_trap /* 3: machine software interrupt */
    j _start_trap /* 4: user timer interrupt */
    j _start_trap /* 5: supervisor timer interrupt */
    j _start_trap /* 6: reserved */
    j _start_trap /* 7: machine timer interrupt */
    j _start_trap /* 8: user external interrupt */
    j _start_trap /* 9: supervisor external interrupt */
    j _start_trap /* 10: reserved */
    j _start_trap /* 11: machine external interrupt */
    j _start_trap /* 12: reserved */
    j _start_trap /* 13: reserved */
    j _start_trap /* 14: reserved */
    j _start_trap /* 15: reserved */
    j _start_trap /* 16-30: On Ibex, reserved for 'fast' interrupts */
    j _start_trap
    j _start_trap
    j _start_trap
    j _start_trap
    j _start_trap
    j _start_trap
    j _start_trap
    j _start_trap
    j _start_trap
    j _start_trap
    j _start_trap
    j _start_trap
    j _start_trap
    j _start_trap
    j _start_trap /* 31: reset vector */
    .size _mtvec_table, .-_mtvec_table
    .option pop
    "
);

#[cfg(feature = "verilator")]
global_asm!(
    r#"
    /*
     * We don't want to have separate build targets or linker script templating
     * to run under verilator (e.g. to set the origin to 0x2000_0000 instead of
     * the normal value of 0x2001_0000).
     *
     * Instead, when building for verilator, we add a ".fake_rom_ext" section that
     * the linker script locates at 0x2000_0000.  In this section, we construct
     * the most trivial of ROM_EXT manifests and set the entrypoint to the start
     * offset of the kernel.
     *
     * The verilator test ROM doesn't do any validation of the manifest header.
     * We don't _need_ to set any version numbers or magic identifier words.
     *
     * We set the following:
     *     address_translation: HardenedFalse (we want the test_rom to boot us at 0x20000000).
     *     identifier: So we show up nicely in `opentitantool image manifest show ...`.
     *     length: So we show up nicely...
     *     entry_point: Offset the test_rom will jump to.
     */
    .section .fake_rom_ext, "ax"
    .option push
    .option norvc
    .option norelax

    .global _rom_ext_manifest
    .global _rom_ext_trampoline
_rom_ext_manifest:
    .space 816
    .word 0x1d4                 /* address_translation (HardenedFalse) */
    .word 0x4552544f            /* identifier (OTRE) */
    .word 0                     /* manifest_version (minor, major) */
    .word 0                     /* signed_region_end */
    .word 0x10000               /* length */
    .space 64
    .word _rom_ext_trampoline-_rom_ext_manifest    /* entry_point */
    .space 120

_rom_ext_trampoline:
    la  a0, 0x411f0000          /* base of rv_core_ibex */
    la  a1, 0xa007FFFF          /* target address & size for remap (512K) */
    la  a2, _rom_ext_manifest   /* reads from this source address */

    /* store target to IBUS_ and DBUS_ADDR_MATCHING_1 */
    sw  a1, 0x20(a0)
    sw  a1, 0x40(a0)

    /* store source to IBUS_ and DBUS_REMAP_ADDR_1 */
    sw  a2, 0x28(a0)
    sw  a2, 0x48(a0)

    /* enable via IBUS_ and DBUS_ADDR_EN_1 */
    li  a2, 1
    sw  a2, 0x18(a0)
    sw  a2, 0x38(a0)

    /* next manifest address */
    la  a1, 0xa0010000
    /* next manifest entry point */
    lw  a2, 900(a1)
    add ra, a1, a2
    jalr zero, ra, 0
    .option pop
"#
);
