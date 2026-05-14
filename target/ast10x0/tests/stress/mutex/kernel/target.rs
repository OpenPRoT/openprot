// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#![no_std]
#![no_main]

use arch_arm_cortex_m::Arch;
use console_backend::console_backend_write_all;
use entry as _;
use target_common::{TargetInterface, declare_target};

pub struct Target {}

impl TargetInterface for Target {
    const NAME: &'static str = "AST10x0 Kernel Mutex Stress";

    fn main() -> ! {
        static mut APP_STATE: mutex::AppState<Arch> = mutex::AppState::new(Arch);
        // SAFETY: `main` is only executed once, so we never generate more
        // than one `&mut` reference to `APP_STATE`.
        #[expect(static_mut_refs)]
        let _ = mutex::main(Arch, unsafe { &mut APP_STATE });
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
