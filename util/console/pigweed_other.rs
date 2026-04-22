// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#![no_std]

/// This is a low-level console output function that works with firmware
/// code.  This function is a wrapper for the pigweed `DebugLog` syscall.
/// We make the syscall directly because this is a static library and we
/// don't want to create duplicate symbols for the syscall crate.
///
/// # Safety
///
/// Callers must supply a valid ptr and length.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn system_lowlevel_console_write(_ptr: *const u8, _length: usize) {
    // You need to implement the equivlent of:
    // let _ = syscall::debug_log(bytes);
    unimplemented!();
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
