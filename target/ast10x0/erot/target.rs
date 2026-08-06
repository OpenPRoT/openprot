// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! OpenPRoT eRoT production image for the AST1060 — skeleton.
//!
//! Kernel boots, announces itself, and parks. The composition plan —
//! which services this image grows and in what order — is documented in
//! `system.json5`, the file that will hold them. Board bring-up (pinctrl
//! for the I²C buses and the reset/ready GPIO lines) joins here as the
//! services that own that hardware land.

#![no_std]
#![no_main]

use console_backend::console_backend_write_all;
use target_common::{declare_target, TargetInterface};
use {console_backend as _, entry as _};

pub struct Target {}

impl TargetInterface for Target {
    const NAME: &'static str = "OpenPRoT eRoT (AST1060)";

    fn main() -> ! {
        pw_log::info!("=== OpenPRoT eRoT (AST1060) ===");
        let _ = console_backend_write_all(b"openprot erot: kernel up; no services composed yet\n");

        #[expect(clippy::empty_loop)]
        loop {}
    }
}

declare_target!(Target);
