// Licensed under the Apache-2.0 license

//! AST1030-BMC kernelspace threads test target.

#![no_std]
#![no_main]

use arch_arm_cortex_m::Arch;
use cortex_m_semihosting::debug::{EXIT_FAILURE, EXIT_SUCCESS, exit};
use target_common::{TargetInterface, declare_target};
use {console_backend as _, entry as _};

/// AST1030 threads test target.
pub struct Target {}

impl TargetInterface for Target {
    const NAME: &'static str = "AST1030-BMC Kernelspace Threads";

    fn main() -> ! {
        static mut APP_STATE: threads::AppState<Arch> = threads::AppState::new(Arch);
        // SAFETY: `main` is only executed once, so we never generate more
        // than one `&mut` reference to `APP_STATE`.
        #[expect(static_mut_refs)]
        let exit_status = match threads::main(Arch, unsafe { &mut APP_STATE }) {
            Ok(()) => EXIT_SUCCESS,
            Err(_e) => EXIT_FAILURE,
        };
        exit(exit_status);
        #[expect(clippy::empty_loop)]
        loop {}
    }
}

declare_target!(Target);
