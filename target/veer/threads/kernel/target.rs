// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0
#![no_std]
#![no_main]

use arch_riscv::Arch;
use entry::exit;
use target_common::{declare_target, TargetInterface};
use {codegen as _, console_backend as _};

pub struct Target {}

impl TargetInterface for Target {
    const NAME: &'static str = "Caliptra-MCU Kernelspace Threads";

    fn main() -> ! {
        static mut APP_STATE: threads::AppState<Arch> = threads::AppState::new(Arch);
        // SAFETY: `main` is only executed once, so we never generate more
        // than one `&mut` reference to `APP_STATE`.
        #[allow(static_mut_refs)]
        match threads::main(Arch, unsafe { &mut APP_STATE }) {
            Ok(()) => {
                pw_log::info!("PASS");
                exit(0);
            }
            Err(e) => {
                pw_log::info!("FAIL: {}", e as u32);
                exit(e as u32);
            }
        };
    }
}

declare_target!(Target);
