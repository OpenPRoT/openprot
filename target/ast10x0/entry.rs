// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Entry point for ASPEED AST10x0 target.
#![no_std]
#![no_main]

use arch_arm_cortex_m::Arch;

#[unsafe(no_mangle)]
#[allow(non_snake_case)]
pub extern "C" fn pw_assert_HandleFailure() -> ! {
    use kernel::Arch as _;
    Arch::panic()
}

#[cortex_m_rt::entry]
fn main() -> ! {
    kernel::static_init_state!(static mut INIT_STATE: InitKernelState<Arch>);
    #[allow(static_mut_refs)]
    kernel::main(Arch, unsafe { &mut INIT_STATE });
}
