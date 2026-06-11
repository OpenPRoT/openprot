// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#![no_std]
#![no_main]

#[path = "test_common.rs"]
mod test_common;

use ast10x0_peripherals::scu::SpiMonitorInstance;
use ast10x0_peripherals::spimonitor::SpiMonitorController;
use console_backend::console_backend_write_all;
use target_common::{declare_target, TargetInterface};
use {console_backend as _, entry as _};

struct Spim2Config;

impl test_common::TestConfig for Spim2Config {
    const INSTANCE: SpiMonitorInstance = SpiMonitorInstance::Spim1;
    const CONTROLLER: SpiMonitorController = SpiMonitorController::Spim1;
}

struct Target;

impl TargetInterface for Target {
    const NAME: &'static str = "AST10x0 External SPIM2 Configuration Test";

    fn main() -> ! {
        let sentinel = if test_common::run::<Spim2Config>().is_ok() {
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
