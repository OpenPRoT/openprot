// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#![no_std]
#![no_main]
use target_common::{declare_target, TargetInterface};
use unittest_core::TestsResult;
use {codegen as _, console_backend as _, entry as _, integration_tests as _};

pub struct Target {}

impl TargetInterface for Target {
    const NAME: &'static str = "Earlgrey Unittest Runner";

    fn main() -> ! {
        // riscv does not run ctors, so we do it manually. Note that this is
        // required in order to register tests, which is a prerequisite to
        // calling `run_all_tests` below.
        unsafe { target_common::run_ctors() };

        match unittest_core::run_all_tests!() {
            TestsResult::AllPassed => pw_log::info!("PASS"),
            TestsResult::SomeFailed => pw_log::info!("FAIL: 1"),
        }
        loop {}
    }
}

declare_target!(Target);
