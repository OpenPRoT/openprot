// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0
#![no_std]
#![no_main]
use core::arch::global_asm;

use arch_riscv::Arch;
use kernel::{self as _};

#[unsafe(no_mangle)]
#[allow(non_snake_case)]
pub unsafe extern "C" fn pw_assert_HandleFailure() -> ! {
    use kernel::Arch as _;
    Arch::panic()
}

#[riscv_rt::entry]
fn main() -> ! {
    kernel::static_init_state!(static mut INIT_STATE: InitKernelState<Arch>);

    // SAFETY: `main` is only executed once, so we never generate more than one
    // `&mut` reference to `INIT_STATE`.
    #[allow(static_mut_refs)]
    kernel::main(Arch, unsafe { &mut INIT_STATE });
}

pub fn exit(code: u32) -> ! {
    #[cfg(feature = "emulator")]
    unsafe {
        // SAFETY: writing to this address will cause the emulator to exit.
        let exitcode = core::ptr::with_exposed_provenance_mut::<u32>(0x1000_2000);
        exitcode.write_volatile(code);
    }
    loop {}
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
