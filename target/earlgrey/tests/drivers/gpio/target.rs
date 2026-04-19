// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#![no_std]
#![no_main]
use target_common::{TargetInterface, declare_target};
use {console_backend as _, entry as _};

pub struct Target {}

impl TargetInterface for Target {
    const NAME: &'static str = "Earlgrey GPIO Smoke Test";

    fn main() -> ! {
        codegen::start();
        loop {}
    }
}

declare_target!(Target);
