// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! AST10x0 GPIO interrupt bring-up - kernel side.

#![no_std]
#![no_main]

use console_backend::console_backend_write_all;
use entry as _;
use target_common::{declare_target, TargetInterface};

pub struct Target {}

impl TargetInterface for Target {
    const NAME: &'static str = "AST10x0 GPIO IRQ Bringup";

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
