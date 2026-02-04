// Copyright 2025 The Pigweed Authors
//
// Licensed under the Apache License, Version 2.0 (the "License"); you may not
// use this file except in compliance with the License. You may obtain a copy of
// the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS, WITHOUT
// WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied. See the
// License for the specific language governing permissions and limitations under
// the License.
#![no_std]
#![no_main]

use arch_riscv::Arch;
use target_common::{declare_target, TargetInterface};
use {codegen as _, console_backend as _, entry as _};

pub struct Target {}

impl TargetInterface for Target {
    const NAME: &'static str = "Earlgrey Kernelspace Threads";

    fn main() -> ! {
        static mut APP_STATE: threads::AppState<Arch> = threads::AppState::new(Arch);
        // SAFETY: `main` is only executed once, so we never generate more
        // than one `&mut` reference to `APP_STATE`.
        #[allow(static_mut_refs)]
        match threads::main(Arch, unsafe { &mut APP_STATE }) {
            Ok(()) => pw_log::info!("PASS"),
            Err(e) => pw_log::info!("FAIL: {}", e as u32),
        };
        loop {}
    }
}

declare_target!(Target);
