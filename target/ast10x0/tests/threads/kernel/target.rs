// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! AST10x0 Kernelspace Threads Test

#![no_std]
#![no_main]

use arch_arm_cortex_m::Arch;
use console_backend::console_backend_write_all;
use target_common::{TargetInterface, declare_target};
use entry as _;

pub struct Target {}

impl TargetInterface for Target {
    const NAME: &'static str = "AST10x0 Kernelspace Threads";

    fn main() -> ! {
        static mut APP_STATE: threads::AppState<Arch> = threads::AppState::new(Arch);
        // SAFETY: `main` is only executed once, so we never generate more
        // than one `&mut` reference to `APP_STATE`.
        #[expect(static_mut_refs)]
        let sentinel: &[u8] = match threads::main(Arch, unsafe { &mut APP_STATE }) {
            Ok(()) => b"TEST_RESULT:PASS\n",
            Err(_e) => b"TEST_RESULT:FAIL\n",
        };
        let _ = console_backend_write_all(sentinel);
        #[expect(clippy::empty_loop)]
        loop {}
    }
}

declare_target!(Target);
