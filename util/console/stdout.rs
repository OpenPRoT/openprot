// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

use std::io::Write;

#[unsafe(no_mangle)]
extern "Rust" fn system_lowlevel_console_write(bytes: &[u8]) {
    let _ = std::io::stdout().write_all(bytes);
    let _ = std::io::stdout().flush();
}
