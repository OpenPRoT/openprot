// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#![no_std]

use userspace::syscall;

#[unsafe(no_mangle)]
pub fn system_lowlevel_console_write(bytes: &[u8]) {
    let _ = syscall::debug_log(bytes);
}
