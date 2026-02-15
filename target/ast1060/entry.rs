// Licensed under the Apache-2.0 license

//! AST1060 BMC boot entry point.
//!
//! Initializes the ARM Cortex-M and hands off to the Pigweed kernel.

#![no_std]
#![no_main]

use arch_arm_cortex_m::Arch;
use kernel::{self as _};

#[unsafe(no_mangle)]
#[allow(non_snake_case)]
pub unsafe extern "C" fn pw_assert_HandleFailure() -> ! {
    use kernel::Arch as _;
    Arch::panic()
}

#[cortex_m_rt::entry]
fn main() -> ! {
    kernel::static_init_state!(static mut INIT_STATE: InitKernelState<Arch>);

    #[allow(static_mut_refs)]
    kernel::main(Arch, unsafe { &mut INIT_STATE });
}
