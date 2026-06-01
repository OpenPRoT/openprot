// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#![no_std]
#![no_main]

use console_backend::console_backend_write_all;
use entry as _;
use target_common::{declare_target, TargetInterface};

pub struct Target {}

impl TargetInterface for Target {
    const NAME: &'static str = "AST10x0 IPC Stress";

    fn main() -> ! {
        codegen::start();
        #[expect(clippy::empty_loop)]
        loop {}
    }

    fn shutdown(code: u32) -> ! {
        if code != 0 {
            let _ = console_backend_write_all(b"TEST_RESULT:FAIL\n");
        }
        #[expect(clippy::empty_loop)]
        loop {}
    }
}

declare_target!(Target);
