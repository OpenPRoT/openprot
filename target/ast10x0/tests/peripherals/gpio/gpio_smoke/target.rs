// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! AST10x0 GPIOA smoke test - kernel side.
//!
//! Only routes the pins (the kernel is the only side with SCU access); the userspace app drives
//! them. Kernel-mode pin driving isn't a pattern anything else in this tree needs — see
//! gpio_smoke_app_main.rs.

#![no_std]
#![no_main]

use console_backend::console_backend_write_all;
use entry as _;
use target_common::{declare_target, TargetInterface};

pub struct Target {}

impl TargetInterface for Target {
    const NAME: &'static str = "AST10x0 GPIOA smoke test";

    fn main() -> ! {
        // Routes exactly the pins the manifest's pin_claims_<process> entries name — generated
        // from this same system.json5, so this can never drift from what the app actually claims.
        // SAFETY: sole call in this binary, at boot, before any process starts.
        unsafe { kernel_pin_routes::route_claimed_pins() };

        codegen::start();
        #[expect(clippy::empty_loop)]
        loop {}
    }

    fn shutdown(code: u32) -> ! {
        let sentinel: &[u8] = if code == 0 {
            b"TEST_RESULT:PASS\n"
        } else {
            b"TEST_RESULT:FAIL\n"
        };
        let _ = console_backend_write_all(sentinel);
        #[expect(clippy::empty_loop)]
        loop {}
    }
}

declare_target!(Target);
