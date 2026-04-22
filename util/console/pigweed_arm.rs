// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#![no_std]
use core::arch::naked_asm;

/// This is a low-level console output function that works with firmware
/// code.  This function is a wrapper for the pigweed `DebugLog` syscall.
/// We make the syscall directly because this is a static library and we
/// don't want to create duplicate symbols for the syscall crate.
///
/// # Safety
///
/// Callers must supply a valid ptr and length.
#[unsafe(no_mangle)]
#[unsafe(naked)]
pub unsafe extern "C" fn system_lowlevel_console_write(ptr: *const u8, length: usize) {
    // This bit of assembly code is the same as:
    // let _ = syscall::debug_log(bytes);
    naked_asm!("
            push  {{r4-r5, r11}}
            mov   r11, {id}
            mov   r4, r0
            mov   r5, r1
            svc   0
            mov   r0, r4
            mov   r1, r5
            pop  {{r4-r5, r11}}
            bx lr
        ",
        id = const 0xF002_u32,
    );
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
