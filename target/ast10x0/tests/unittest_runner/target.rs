// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#![no_std]
#![no_main]

use console_backend::console_backend_write_all;
use target_common::{TargetInterface, declare_target};
use unittest_core::TestsResult;
use {entry as _, integration_tests as _};

pub struct Target {}

impl TargetInterface for Target {
    const NAME: &'static str = "AST10x0 Unittest Runner";

    fn main() -> ! {
        // cortex_m_rt does not run ctors, so we do it manually. Note that this
        // is required in order to register tests, which is a prerequisite to
        // calling `run_all_tests` below.
        unsafe { target_common::run_ctors() };

        let sentinel: &[u8] = match unittest_core::run_all_tests!() {
            TestsResult::AllPassed => b"TEST_RESULT:PASS\n",
            TestsResult::SomeFailed => b"TEST_RESULT:FAIL\n",
        };
        let _ = console_backend_write_all(sentinel);
        loop {}
    }
}

declare_target!(Target);
