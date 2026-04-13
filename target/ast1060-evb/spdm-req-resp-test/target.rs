// Licensed under the Apache-2.0 license

//! AST1060-EVB SPDM Requester-Responder Test Target

#![no_std]
#![no_main]

use target_common::{declare_target, TargetInterface};
use {console_backend as _, entry as _};

pub struct Target {}

impl TargetInterface for Target {
    const NAME: &'static str = "AST1060-EVB SPDM Req-Resp Test";

    fn main() -> ! {
        codegen::start();
        #[expect(clippy::empty_loop)]
        loop {}
    }

    fn shutdown(code: u32) -> ! {
        pw_log::info!("Shutting down with code {:04x}", code as u32);
        #[expect(clippy::empty_loop)]
        loop {}
    }
}

declare_target!(Target);
